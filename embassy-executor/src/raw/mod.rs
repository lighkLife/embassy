//! 原始执行器
//!
//! 这个模块暴露 "原生" 执行器和任务结构体，可以用作更低层次的控制。
//!
//! ## 警告： 这里有危险
//! 使用这个模块需要尊重微妙的安全契约。更推荐使用[executor wrappers](crate::Executor)和
//! [`embassy_executor::task`](embassy_macros::task)宏，这些都是安全的。
//!
//! ---
//! Raw executor.
//!
//! This module exposes "raw" Executor and Task structs for more low level control.
//!
//! ## WARNING: here be dragons!
//!
//! Using this module requires respecting subtle safety contracts. If you can, prefer using the safe
//! [executor wrappers](crate::Executor) and the [`embassy_executor::task`](embassy_macros::task) macro, which are fully safe.

#[cfg_attr(target_has_atomic = "ptr", path = "run_queue_atomics.rs")]
#[cfg_attr(not(target_has_atomic = "ptr"), path = "run_queue_critical_section.rs")]
mod run_queue;

#[cfg_attr(all(cortex_m, target_has_atomic = "8"), path = "state_atomics_arm.rs")]
#[cfg_attr(all(not(cortex_m), target_has_atomic = "8"), path = "state_atomics.rs")]
#[cfg_attr(not(target_has_atomic = "8"), path = "state_critical_section.rs")]
mod state;

#[cfg(feature = "integrated-timers")]
mod timer_queue;
pub(crate) mod util;
#[cfg_attr(feature = "turbowakers", path = "waker_turbo.rs")]
mod waker;

use core::future::Future;
use core::marker::PhantomData;
use core::mem;
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll};

#[cfg(feature = "integrated-timers")]
use embassy_time::driver::{self, AlarmHandle};
#[cfg(feature = "integrated-timers")]
use embassy_time::Instant;
#[cfg(feature = "rtos-trace")]
use rtos_trace::trace;

use self::run_queue::{RunQueue, RunQueueItem};
use self::state::State;
use self::util::{SyncUnsafeCell, UninitCell};
pub use self::waker::task_from_waker;
use super::SpawnToken;

/// 任务指针指向的原始任务头
///
/// ---
/// Raw task header for use in task pointers.
pub(crate) struct TaskHeader {
    pub(crate) state: State,
    pub(crate) run_queue_item: RunQueueItem,
    pub(crate) executor: SyncUnsafeCell<Option<&'static SyncExecutor>>,
    poll_fn: SyncUnsafeCell<Option<unsafe fn(TaskRef)>>,

    #[cfg(feature = "integrated-timers")]
    pub(crate) expires_at: SyncUnsafeCell<Instant>,
    #[cfg(feature = "integrated-timers")]
    pub(crate) timer_queue_item: timer_queue::TimerQueueItem,
}

/// 这本质上是一个 `&'static TaskStorage<F>`，其中 Future 的类型已被擦除
///
/// ---
/// This is essentially a `&'static TaskStorage<F>` where the type of the future has been erased.
#[derive(Clone, Copy)]
pub struct TaskRef {
    ptr: NonNull<TaskHeader>,
}

unsafe impl Send for TaskRef where &'static TaskHeader: Send {}
unsafe impl Sync for TaskRef where &'static TaskHeader: Sync {}

impl TaskRef {
    fn new<F: Future + 'static>(task: &'static TaskStorage<F>) -> Self {
        Self {
            ptr: NonNull::from(task).cast(),
        }
    }

    /// 安全性： 这个指针必须已经通过 `Task::as_ptr`来获取
    ///
    /// ---
    /// Safety: The pointer must have been obtained with `Task::as_ptr`
    pub(crate) unsafe fn from_ptr(ptr: *const TaskHeader) -> Self {
        Self {
            ptr: NonNull::new_unchecked(ptr as *mut TaskHeader),
        }
    }

    pub(crate) fn header(self) -> &'static TaskHeader {
        unsafe { self.ptr.as_ref() }
    }

    /// 返回的指针对于整个 TaskStorage 都是有效的
    ///
    /// ---
    /// The returned pointer is valid for the entire TaskStorage.
    pub(crate) fn as_ptr(self) -> *const TaskHeader {
        self.ptr.as_ptr()
    }
}

///
/// 可以生成任务的原始存储器
/// Raw storage in which a task can be spawned.
///
/// 这个结构体拥有生成一个 future 为 `F` 的任务所需的内存。
/// 在给定的时间，`TaskStorage` 可能处于已生成或未生成状态。
/// 你///可以使用 [`TaskStorage::spawn()`] 来生成它，如果它已经被生成，那么这个方法将会失败。
///
/// `TaskStorage` 必须一直存活，即使任务已经结束运行，它仍然不会被释放。因此，相关的方法需要 `&'static self`。
/// 但是，它可以被重用。
///
/// [embassy_executor::task](embassy_macros::task) 宏内部会分配一个 `TaskStorage` 类型的静态数组。
/// 使用原始 `Task` 的最常见原因是控制在哪里给任务非配内存：在栈上，或者在堆上，例如使用 `Box::leak` 等。
///
/// 需要使用 repr(C) 来保证任务在结构体的内存布局中，偏移量为 0
/// 这使得 TaskHeader 和 TaskStorage 指针互相转换是安全的。
///
/// ---
/// Raw storage in which a task can be spawned.
///
/// This struct holds the necessary memory to spawn one task whose future is `F`.
/// At a given time, the `TaskStorage` may be in spawned or not-spawned state. You
/// may spawn it with [`TaskStorage::spawn()`], which will fail if it is already spawned.
///
/// A `TaskStorage` must live forever, it may not be deallocated even after the task has finished
/// running. Hence the relevant methods require `&'static self`. It may be reused, however.
///
/// Internally, the [embassy_executor::task](embassy_macros::task) macro allocates an array of `TaskStorage`s
/// in a `static`. The most common reason to use the raw `Task` is to have control of where
/// the memory for the task is allocated: on the stack, or on the heap with e.g. `Box::leak`, etc.

// repr(C) is needed to guarantee that the Task is located at offset 0
// This makes it safe to cast between TaskHeader and TaskStorage pointers.
#[repr(C)]
pub struct TaskStorage<F: Future + 'static> {
    raw: TaskHeader,
    future: UninitCell<F>, // Valid if STATE_SPAWNED
}

impl<F: Future + 'static> TaskStorage<F> {
    const NEW: Self = Self::new();

    /// 创建一个新的任务存储器，处于未创建的状态
    ///
    /// ---
    /// Create a new TaskStorage, in not-spawned state.
    pub const fn new() -> Self {
        Self {
            raw: TaskHeader {
                state: State::new(),
                run_queue_item: RunQueueItem::new(),
                executor: SyncUnsafeCell::new(None),
                // Note: this is lazily initialized so that a static `TaskStorage` will go in `.bss`
                poll_fn: SyncUnsafeCell::new(None),

                #[cfg(feature = "integrated-timers")]
                expires_at: SyncUnsafeCell::new(Instant::from_ticks(0)),
                #[cfg(feature = "integrated-timers")]
                timer_queue_item: timer_queue::TimerQueueItem::new(),
            },
            future: UninitCell::uninit(),
        }
    }

    /// 尝试创建任务
    ///
    /// `future` 闭包用来构造 future。 可以生成任务的情况下，它才被调用。它是一个闭包，
    /// 而不是简单的 `future: F` 参数，用来确保在适当的位置构造 future，多亏 NRVO 优化，
    /// 避免了在栈上的临时复制。
    ///
    /// 如果人物已经被创建，并且还没有运行结束，这个方法会失败。在这种情况下，这个错误会被推迟：
    /// 返回一个空的 SpawnToken，从而导致调用 [`Spawner::spawn()`](super::Spawner::spawn)
    /// 返回错误。
    ///
    /// 一旦任务已经运行结束，你可以再一次创建它。允许在另一个不同的执行器中创建它。
    ///
    /// NRVO优化，即命名返回值优化（Named Return Value Optimization），是Visual C++2005及之后版本支持的一种优化技术。
    /// 当一个函数的返回值是一个对象时，正常的返回语句的执行过程是将这个对象从当前函数的局部作用域拷贝到返回区，以便调用者可以访问。
    /// 然而，如果所有的返回语句都返回同一个对象，NRVO优化的作用是在这个对象建立的时候直接在返回区建立，
    /// 从而避免了函数返回时调用拷贝构造函数的需要，减少了对象的创建与销毁过程。
    ///
    /// ---
    /// Try to spawn the task.
    ///
    /// The `future` closure constructs the future. It's only called if spawning is
    /// actually possible. It is a closure instead of a simple `future: F` param to ensure
    /// the future is constructed in-place, avoiding a temporary copy in the stack thanks to
    /// NRVO optimizations.
    ///
    /// This function will fail if the task is already spawned and has not finished running.
    /// In this case, the error is delayed: a "poisoned" SpawnToken is returned, which will
    /// cause [`Spawner::spawn()`](super::Spawner::spawn) to return the error.
    ///
    /// Once the task has finished running, you may spawn it again. It is allowed to spawn it
    /// on a different executor.
    pub fn spawn(&'static self, future: impl FnOnce() -> F) -> SpawnToken<impl Sized> {
        let task = AvailableTask::claim(self);
        match task {
            Some(task) => task.initialize(future),
            None => SpawnToken::new_failed(),
        }
    }

    unsafe fn poll(p: TaskRef) {
        let this = &*(p.as_ptr() as *const TaskStorage<F>);

        let future = Pin::new_unchecked(this.future.as_mut());
        let waker = waker::from_task(p);
        let mut cx = Context::from_waker(&waker);
        match future.poll(&mut cx) {
            Poll::Ready(_) => {
                this.future.drop_in_place();
                this.raw.state.despawn();

                #[cfg(feature = "integrated-timers")]
                this.raw.expires_at.set(Instant::MAX);
            }
            Poll::Pending => {}
        }

        // 编译器会调用 waker 的 drop 方法，不过对我们的 waker 来说，这是一个空操作（drop里面没有任何操作）
        // the compiler is emitting a virtual call for waker drop, but we know
        // it's a noop for our waker.
        mem::forget(waker);
    }

    #[doc(hidden)]
    #[allow(dead_code)]
    fn _assert_sync(self) {
        fn assert_sync<T: Sync>(_: T) {}

        assert_sync(self)
    }
}

/// 一个未初始化的 [`TaskStorage`].
///
/// ---
/// An uninitialized [`TaskStorage`].
pub struct AvailableTask<F: Future + 'static> {
    task: &'static TaskStorage<F>,
}

impl<F: Future + 'static> AvailableTask<F> {
    /// 尝试申请一个 [`TaskStorage`].
    ///
    /// 如果任务以及被创建，并且没有运行结束，这个方法会返回 `None`
    ///
    /// ---
    /// Try to claim a [`TaskStorage`].
    ///
    /// This function returns `None` if a task has already been spawned and has not finished running.
    pub fn claim(task: &'static TaskStorage<F>) -> Option<Self> {
        task.raw.state.spawn().then(|| Self { task })
    }

    fn initialize_impl<S>(self, future: impl FnOnce() -> F) -> SpawnToken<S> {
        unsafe {
            self.task.raw.poll_fn.set(Some(TaskStorage::<F>::poll));
            self.task.future.write_in_place(future);

            let task = TaskRef::new(self.task);

            SpawnToken::new(task)
        }
    }

    /// 初始化 [`TaskStorage`] 来运行给定的 future。
    ///
    /// ---
    /// Initialize the [`TaskStorage`] to run the given future.
    pub fn initialize(self, future: impl FnOnce() -> F) -> SpawnToken<F> {
        self.initialize_impl::<F>(future)
    }

    /// 初始化 [`TaskStorage`] 来运行给定的 future。
    ///
    /// # 安全性
    ///
    /// `future` 必须是 `move || my_async_fn(args)` 形式的闭包，`my_async_fn` 是一个 `async fn`，
    /// 不是一个手写的 `Future`。
    ///
    ///
    /// ---
    /// Initialize the [`TaskStorage`] to run the given future.
    ///
    /// # Safety
    ///
    /// `future` must be a closure of the form `move || my_async_fn(args)`, where `my_async_fn`
    /// is an `async fn`, NOT a hand-written `Future`.
    #[doc(hidden)]
    pub unsafe fn __initialize_async_fn<FutFn>(self, future: impl FnOnce() -> F) -> SpawnToken<FutFn> {
        // 但使用 send-spawning 生成任务时，我们在该线程中构建标future，通过将其放入队列，将其发送到执行器线程。因此，理论上，
        // send-spawning 需要 future 的 `F` 是 `Send` 的。
        //
        // 问题是这比实际所需的限制更严格。一旦 future 开始执行，它永远不会被发送到另一个线程中。它只会在生成时被发送。
        // 只要任务的参数是 Send 的就足够了。（实际上，很容易意外地让你的 future 变为非 Send 的， 比如在 `.await` 之间
        // 使用一个 `Rc` 或 `&RefCell`）
        //
        // 在第一次 poll 中
        //
        // ---
        // When send-spawning a task, we construct the future in this thread, and effectively
        // "send" it to the executor thread by enqueuing it in its queue. Therefore, in theory,
        // send-spawning should require the future `F` to be `Send`.
        //
        // The problem is this is more restrictive than needed. Once the future is executing,
        // it is never sent to another thread. It is only sent when spawning. It should be
        // enough for the task's arguments to be Send. (and in practice it's super easy to
        // accidentally make your futures !Send, for example by holding an `Rc` or a `&RefCell` across an `.await`.)
        //
        // We can do it by sending the task args and constructing the future in the executor thread
        // on first poll. However, this cannot be done in-place, so it'll waste stack space for a copy
        // of the args.
        //
        // Luckily, an `async fn` future contains just the args when freshly constructed. So, if the
        // args are Send, it's OK to send a !Send future, as long as we do it before first polling it.
        //
        // (Note: this is how the generators are implemented today, it's not officially guaranteed yet,
        // but it's possible it'll be guaranteed in the future. See zulip thread:
        // https://rust-lang.zulipchat.com/#narrow/stream/187312-wg-async/topic/.22only.20before.20poll.22.20Send.20futures )
        //
        // The `FutFn` captures all the args, so if it's Send, the task can be send-spawned.
        // This is why we return `SpawnToken<FutFn>` below.
        //
        // This ONLY holds for `async fn` futures. The other `spawn` methods can be called directly
        // by the user, with arbitrary hand-implemented futures. This is why these return `SpawnToken<F>`.
        self.initialize_impl::<FutFn>(future)
    }
}

/// 原始存储器，可以保存多个相同类型的任务
///
/// 这本质上是一个 `[TaskStorage<F>; N]`
///
/// ---
/// Raw storage that can hold up to N tasks of the same type.
///
/// This is essentially a `[TaskStorage<F>; N]`.
pub struct TaskPool<F: Future + 'static, const N: usize> {
    pool: [TaskStorage<F>; N],
}

impl<F: Future + 'static, const N: usize> TaskPool<F, N> {
    /// 创建一个 TaskPool， 其中的所有任务都是未创建的状态
    ///
    /// ---
    /// Create a new TaskPool, with all tasks in non-spawned state.
    pub const fn new() -> Self {
        Self {
            pool: [TaskStorage::NEW; N],
        }
    }

    fn spawn_impl<T>(&'static self, future: impl FnOnce() -> F) -> SpawnToken<T> {
        match self.pool.iter().find_map(AvailableTask::claim) {
            Some(task) => task.initialize_impl::<T>(future),
            None => SpawnToken::new_failed(),
        }
    }

    /// 尝试在池中创建任务
    ///
    /// 在 [`TaskStorage::spawn()`] 可以看到详情
    ///
    /// 这个方法会遍历池，并且在第一个空闲的存储位置创建任务。如果没有空闲的，就会返回一个空的 SpawnToken，
    /// 空的 SpawnToken 会导致 [`Spawner::spawn()`](super::Spawner::spawn) 返回错误。
    ///
    /// ---
    /// Try to spawn a task in the pool.
    ///
    /// See [`TaskStorage::spawn()`] for details.
    ///
    /// This will loop over the pool and spawn the task in the first storage that
    /// is currently free. If none is free, a "poisoned" SpawnToken is returned,
    /// which will cause [`Spawner::spawn()`](super::Spawner::spawn) to return the error.
    pub fn spawn(&'static self, future: impl FnOnce() -> F) -> SpawnToken<impl Sized> {
        self.spawn_impl::<F>(future)
    }

    /// 类似 spawn()，但是，如果参数是 Send 的，即使 future 不是 Send 的，允许任务被 send-spawned。
    ///
    /// 不被 semver 保证范围，不要直接调用它。仅打算被 embassy 宏使用。
    ///
    /// 安全性： `future` 必须是 `move || my_async_fn(args)` 形式的闭包，`my_async_fn` 是一个 `async fn`，
    /// 不是一个手写的 `Future`。
    ///
    /// ---
    /// Like spawn(), but allows the task to be send-spawned if the args are Send even if
    /// the future is !Send.
    ///
    /// Not covered by semver guarantees. DO NOT call this directly. Intended to be used
    /// by the Embassy macros ONLY.
    ///
    /// SAFETY: `future` must be a closure of the form `move || my_async_fn(args)`, where `my_async_fn`
    /// is an `async fn`, NOT a hand-written `Future`.
    #[doc(hidden)]
    pub unsafe fn _spawn_async_fn<FutFn>(&'static self, future: FutFn) -> SpawnToken<impl Sized>
        where
            FutFn: FnOnce() -> F,
    {
        // See the comment in AvailableTask::__initialize_async_fn for explanation.
        self.spawn_impl::<FutFn>(future)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Pender(*mut ());

unsafe impl Send for Pender {}
unsafe impl Sync for Pender {}

impl Pender {
    pub(crate) fn pend(self) {
        extern "Rust" {
            fn __pender(context: *mut ());
        }
        unsafe { __pender(self.0) };
    }
}

pub(crate) struct SyncExecutor {
    run_queue: RunQueue,
    pender: Pender,

    #[cfg(feature = "integrated-timers")]
    pub(crate) timer_queue: timer_queue::TimerQueue,
    #[cfg(feature = "integrated-timers")]
    alarm: AlarmHandle,
}

impl SyncExecutor {
    pub(crate) fn new(pender: Pender) -> Self {
        #[cfg(feature = "integrated-timers")]
            let alarm = unsafe { unwrap!(driver::allocate_alarm()) };

        Self {
            run_queue: RunQueue::new(),
            pender,

            #[cfg(feature = "integrated-timers")]
            timer_queue: timer_queue::TimerQueue::new(),
            #[cfg(feature = "integrated-timers")]
            alarm,
        }
    }

    /// 向任务队列中入队一个任务
    ///
    /// # 安全性
    /// - `task` 必须有一个有效的指针，用来创建任务。
    /// - `task` 必须设置为在此执行器中运行。
    /// - `task` 不能已经入队(在这个执行器或另一个执行器中)
    ///
    ///
    /// ---
    /// Enqueue a task in the task queue
    ///
    /// # Safety
    /// - `task` must be a valid pointer to a spawned task.
    /// - `task` must be set up to run in this executor.
    /// - `task` must NOT be already enqueued (in this executor or another one).
    #[inline(always)]
    unsafe fn enqueue(&self, task: TaskRef) {
        #[cfg(feature = "rtos-trace")]
        trace::task_ready_begin(task.as_ptr() as u32);

        if self.run_queue.enqueue(task) {
            self.pender.pend();
        }
    }

    #[cfg(feature = "integrated-timers")]
    fn alarm_callback(ctx: *mut ()) {
        let this: &Self = unsafe { &*(ctx as *const Self) };
        this.pender.pend();
    }

    pub(super) unsafe fn spawn(&'static self, task: TaskRef) {
        task.header().executor.set(Some(self));

        #[cfg(feature = "rtos-trace")]
        trace::task_new(task.as_ptr() as u32);

        self.enqueue(task);
    }

    /// # 安全性：
    ///
    /// 与  [`Executor::poll`] 相同，只能在创建这个执行器的线程上调用它
    ///
    ///
    /// # Safety
    ///
    /// Same as [`Executor::poll`], plus you must only call this on the thread this executor was created.
    pub(crate) unsafe fn poll(&'static self) {
        #[cfg(feature = "integrated-timers")]
        driver::set_alarm_callback(self.alarm, Self::alarm_callback, self as *const _ as *mut ());

        #[allow(clippy::never_loop)]
        loop {
            #[cfg(feature = "integrated-timers")]
            self.timer_queue.dequeue_expired(Instant::now(), wake_task_no_pend);

            self.run_queue.dequeue_all(|p| {
                let task = p.header();

                #[cfg(feature = "integrated-timers")]
                task.expires_at.set(Instant::MAX);

                if !task.state.run_dequeue() {
                    // 如果任务不是在运行中，忽视它。这可能发生在如下的场景中：
                    //  - 任务被出队，开始 poll
                    //  - 当任务被 poll 时，它被唤醒。 它被放在队列中。
                    //  - 任务 poll 结束，返回  done=true
                    //  - RUNNING 位被清除，但是任务仍在队列
                    //
                    // ---
                    // If task is not running, ignore it. This can happen in the following scenario:
                    //   - Task gets dequeued, poll starts
                    //   - While task is being polled, it gets woken. It gets placed in the queue.
                    //   - Task poll finishes, returning done=true
                    //   - RUNNING bit is cleared, but the task is already in the queue.
                    return;
                }

                #[cfg(feature = "rtos-trace")]
                trace::task_exec_begin(p.as_ptr() as u32);

                // Run the task
                task.poll_fn.get().unwrap_unchecked()(p);

                #[cfg(feature = "rtos-trace")]
                trace::task_exec_end();

                // Enqueue or update into timer_queue
                #[cfg(feature = "integrated-timers")]
                self.timer_queue.update(p);
            });

            #[cfg(feature = "integrated-timers")]
            {
                // 如果设置的时间是过去时间，则 set alarm 可能返回 false
                // 在这种情况下，执行另一个轮询循环迭代。
                // ---
                // If this is already in the past, set_alarm might return false
                // In that case do another poll loop iteration.
                let next_expiration = self.timer_queue.next_expiration();
                if driver::set_alarm(self.alarm, next_expiration.as_ticks()) {
                    break;
                }
            }

            #[cfg(not(feature = "integrated-timers"))]
            {
                break;
            }
        }

        #[cfg(feature = "rtos-trace")]
        trace::system_idle();
    }
}

/// 原始执行器
///
/// 这是 Embassy 执行器的核心。它是底层的需要手动处理唤醒和任务执行。更推荐使用 [上层执行器](crate::Executor)
///
/// 原始执行器将唤醒和调度留给你去处理：
///
/// - 为了让执行器工作，调用 `poll()`。这会运行队列中的所有任务（所有“想运行”的任务）。
/// - 如下所示，你必须提供一个 pender 函数。执行器会通知你有工作可以做了。你必须尽可能快的调用你的`poll()`。
/// - 启用 `arch-xx` 特性会为你定义一个 pender 函数。这意味着你只能使用架构/平台为你实现的执行器。
///   如果你有一个不同的执行器，你就不能启用 `arch-xx`特性。
///
/// pender 可以在*任何*上下文中被调用： 任何线程，任何中断优先级。它也可能被任何执行器的方法同时调用，
/// 你必须正确的处理这些情况。
///
/// 特别说明，你不能在 pender 的回调中直接调用 `poll`，因为这违反了 `poll` 不能被重入调用的要求。
///
/// pender 函数被暴露的名称必须为  `__pender` ，并且需要如下的签名：
///
/// ```rust
/// #[export_name = "__pender"]
/// fn pender(context: *mut ()) {
///    // schedule `poll()` to be called
/// }
/// ```
///
/// `context` 参数是执行器将传递给 pender 的任意数据。你可以在调用 [`Executor::new()`] 时设置 `context`。
/// 例如，你可以使用它来区分执行器，也可以传一个回调指针。
///
/// ---
/// Raw executor.
///
/// This is the core of the Embassy executor. It is low-level, requiring manual
/// handling of wakeups and task polling. If you can, prefer using one of the
/// [higher level executors](crate::Executor).
///
/// The raw executor leaves it up to you to handle wakeups and scheduling:
///
/// - To get the executor to do work, call `poll()`. This will poll all queued tasks (all tasks
///   that "want to run").
/// - You must supply a pender function, as shown below. The executor will call it to notify you
///   it has work to do. You must arrange for `poll()` to be called as soon as possible.
/// - Enabling `arch-xx` features will define a pender function for you. This means that you
///   are limited to using the executors provided to you by the architecture/platform
///   implementation. If you need a different executor, you must not enable `arch-xx` features.
///
/// The pender can be called from *any* context: any thread, any interrupt priority
/// level, etc. It may be called synchronously from any `Executor` method call as well.
/// You must deal with this correctly.
///
/// In particular, you must NOT call `poll` directly from the pender callback, as this violates
/// the requirement for `poll` to not be called reentrantly.
///
/// The pender function must be exported with the name `__pender` and have the following signature:
///
/// ```rust
/// #[export_name = "__pender"]
/// fn pender(context: *mut ()) {
///    // schedule `poll()` to be called
/// }
/// ```
///
/// The `context` argument is a piece of arbitrary data the executor will pass to the pender.
/// You can set the `context` when calling [`Executor::new()`]. You can use it to, for example,
/// differentiate between executors, or to pass a pointer to a callback that should be called.
#[repr(transparent)]
pub struct Executor {
    pub(crate) inner: SyncExecutor,

    _not_sync: PhantomData<*mut ()>,
}

impl Executor {
    pub(crate) unsafe fn wrap(inner: &SyncExecutor) -> &Self {
        mem::transmute(inner)
    }

    /// 创建一个新的执行器
    ///
    /// 但执行其有活要干时，他就会调用 pender 函数，并把 `context` 传给它。
    ///
    /// 在 [`Executor`] 的文档中可以看到 pender 的详情。
    ///
    /// ---
    /// Create a new executor.
    ///
    /// When the executor has work to do, it will call the pender function and pass `context` to it.
    ///
    /// See [`Executor`] docs for details on the pender.
    pub fn new(context: *mut ()) -> Self {
        Self {
            inner: SyncExecutor::new(Pender(context)),
            _not_sync: PhantomData,
        }
    }

    /// 在当前执行器中创建任务
    ///
    /// # 安全性
    ///
    /// `task` 必须是一个有效的指针，必须已经初始化，但还未被分配任务。
    ///
    /// 在当前执行器以外的线程中，可以使用 `unsafe` 去调用这个方法。
    /// 在这种情况下，这个任务的 Future 必须时 Send 的。这个是因为本质上是发送任务给执行器的线程。
    ///
    ///
    /// ---
    /// Spawn a task in this executor.
    ///
    /// # Safety
    ///
    /// `task` must be a valid pointer to an initialized but not-already-spawned task.
    ///
    /// It is OK to use `unsafe` to call this from a thread that's not the executor thread.
    /// In this case, the task's Future must be Send. This is because this is effectively
    /// sending the task to the executor thread.
    pub(super) unsafe fn spawn(&'static self, task: TaskRef) {
        self.inner.spawn(task)
    }

    /// 拉取当前执行器队列中的所有任务
    ///
    /// 这个方法会遍历队列中所有待拉取的任务（如刚被创建的或被唤醒的）。其他任务不会被拉取。
    ///
    /// 在收到 pender 调用后，必须调用 `poll`。即使 pender 没有被调用，也可以去调用 `poll`，
    /// 只是这样会浪费资源。。
    ///
    /// # 安全性
    ///
    /// 禁止在相同的执行器中重入的调用 `poll` 。
    ///
    /// 特别注意 `poll` 可能会被并发调用。因此，禁止在 pender 的回调中直接调用 `poll()`。换而言之，
    /// 回调方法必须稍后再以某种调度方式调用 `poll()`，这时你必须确认 `poll()` 已经没有在运行。
    ///
    /// ---
    /// Poll all queued tasks in this executor.
    ///
    /// This loops over all tasks that are queued to be polled (i.e. they're
    /// freshly spawned or they've been woken). Other tasks are not polled.
    ///
    /// You must call `poll` after receiving a call to the pender. It is OK
    /// to call `poll` even when not requested by the pender, but it wastes
    /// energy.
    ///
    /// # Safety
    ///
    /// You must NOT call `poll` reentrantly on the same executor.
    ///
    /// In particular, note that `poll` may call the pender synchronously. Therefore, you
    /// must NOT directly call `poll()` from the pender callback. Instead, the callback has to
    /// somehow schedule for `poll()` to be called later, at a time you know for sure there's
    /// no `poll()` already running.
    pub unsafe fn poll(&'static self) {
        self.inner.poll()
    }

    /// 获取一个在当前执行器中生成任务的生成器
    ///
    /// 可以多次调用这个方法来获取多个 `Spawner`。你也可以复制 `Spawner`。
    ///
    /// ---
    /// Get a spawner that spawns tasks in this executor.
    ///
    /// It is OK to call this method multiple times to obtain multiple
    /// `Spawner`s. You may also copy `Spawner`s.
    pub fn spawner(&'static self) -> super::Spawner {
        super::Spawner::new(self)
    }
}

/// 通过 `TaskRef` 唤醒任务
///
/// 你可以使用 [`task_from_waker`] 来通过一个 `Waker` 得到 `TaskRef`
///
/// ---
/// Wake a task by `TaskRef`.
///
/// You can obtain a `TaskRef` from a `Waker` using [`task_from_waker`].
pub fn wake_task(task: TaskRef) {
    let header = task.header();
    if header.state.run_enqueue() {
        // We have just marked the task as scheduled, so enqueue it.
        unsafe {
            let executor = header.executor.get().unwrap_unchecked();
            executor.enqueue(task);
        }
    }
}

/// 通过 `TaskRef` 唤醒一个任务，但不调用 pend
///
/// 你可以使用 [`task_from_waker`] 来通过一个 `Waker` 得到 `TaskRef`
///
/// ---
/// Wake a task by `TaskRef` without calling pend.
///
/// You can obtain a `TaskRef` from a `Waker` using [`task_from_waker`].
pub fn wake_task_no_pend(task: TaskRef) {
    let header = task.header();
    if header.state.run_enqueue() {
        // We have just marked the task as scheduled, so enqueue it.
        unsafe {
            let executor = header.executor.get().unwrap_unchecked();
            executor.run_queue.enqueue(task);
        }
    }
}

#[cfg(feature = "integrated-timers")]
struct TimerQueue;

#[cfg(feature = "integrated-timers")]
impl embassy_time::queue::TimerQueue for TimerQueue {
    fn schedule_wake(&'static self, at: Instant, waker: &core::task::Waker) {
        let task = waker::task_from_waker(waker);
        let task = task.header();
        unsafe {
            let expires_at = task.expires_at.get();
            task.expires_at.set(expires_at.min(at));
        }
    }
}

#[cfg(feature = "integrated-timers")]
embassy_time::timer_queue_impl!(static TIMER_QUEUE: TimerQueue = TimerQueue);

#[cfg(feature = "rtos-trace")]
impl rtos_trace::RtosTraceOSCallbacks for Executor {
    fn task_list() {
        // We don't know what tasks exist, so we can't send them.
    }
    #[cfg(feature = "integrated-timers")]
    fn time() -> u64 {
        Instant::now().as_micros()
    }
    #[cfg(not(feature = "integrated-timers"))]
    fn time() -> u64 {
        0
    }
}

#[cfg(feature = "rtos-trace")]
rtos_trace::global_os_callbacks! {Executor}
