use core::future::poll_fn;
use core::marker::PhantomData;
use core::mem;
use core::task::Poll;

use super::raw;

/// 在执行器中创建新任务的令牌
///
/// 当调用任务方法时（比如 `#[embassy_executor::task] async fn my_task() { ... }`），
/// 返回值是一个的 `SpawnToken`，它代表这个任务的实例。你必须将它放到一个执行器中去创建任务，如  [`Spawner::spawn()`]
///
/// 范型参数 `S` 决定执行器在其他线程中是否可以创建任务。如果 `S: Send`，则可以，这允许将其放到 [`SendSpawner`]来生成任务。
/// 否则，就不可以，只能在当前线程中使用 [`Spawner`] 来生成任务。
///
/// # 异常
/// 销毁一个 SpawnToken 实例会导致异常。你不该以这种方式“中止”任务的创建。
/// 一旦你调用一个任务函数，并得到一个 SpawnToken，就 *必须* 创建它。
///
/// ---
/// Token to spawn a newly-created task in an executor.
///
/// When calling a task function (like `#[embassy_executor::task] async fn my_task() { ... }`), the returned
/// value is a `SpawnToken` that represents an instance of the task, ready to spawn. You must
/// then spawn it into an executor, typically with [`Spawner::spawn()`].
///
/// The generic parameter `S` determines whether the task can be spawned in executors
/// in other threads or not. If `S: Send`, it can, which allows spawning it into a [`SendSpawner`].
/// If not, it can't, so it can only be spawned into the current thread's executor, with [`Spawner`].
///
/// # Panics
///
/// Dropping a SpawnToken instance panics. You may not "abort" spawning a task in this way.
/// Once you've invoked a task function and obtained a SpawnToken, you *must* spawn it.
#[must_use = "Calling a task function does nothing on its own. You must spawn the returned SpawnToken, typically with Spawner::spawn()"]
pub struct SpawnToken<S> {
    raw_task: Option<raw::TaskRef>,
    phantom: PhantomData<*mut S>,
}

impl<S> SpawnToken<S> {
    pub(crate) unsafe fn new(raw_task: raw::TaskRef) -> Self {
        Self {
            raw_task: Some(raw_task),
            phantom: PhantomData,
        }
    }

    /// 返回一个 SpawnToken， 代表创建任务失败
    ///
    /// ---
    /// Return a SpawnToken that represents a failed spawn.
    pub fn new_failed() -> Self {
        Self {
            raw_task: None,
            phantom: PhantomData,
        }
    }
}

impl<S> Drop for SpawnToken<S> {
    fn drop(&mut self) {
        // TODO deallocate the task instead.
        panic!("SpawnToken instances may not be dropped. You must pass them to Spawner::spawn()")
    }
}

/// 创建任务时返回的错误
///
/// ---
/// Error returned when spawning a task.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpawnError {
    /// 此任务正在运行的实例太多。
    ///
    /// 默认情况下，一个被 `#[embassy_executor::task]` 标记的任务，在运行期间，只能有一个实例。
    /// 你可以使用 `#[embassy_executor::task(pool_size = 4)]` 来让多个实例并行运行，这也会
    /// 消耗更多的内存。
    ///
    /// ---
    /// Too many instances of this task are already running.
    ///
    /// By default, a task marked with `#[embassy_executor::task]` can only have one instance
    /// running at a time. You may allow multiple instances to run in parallel with
    /// `#[embassy_executor::task(pool_size = 4)]`, at the cost of higher RAM usage.
    Busy,
}

/// 生成任务到执行器中的句柄
///
/// 这个生成器可以生成任何任务（无论任务是否 Send），但是它只能在它执行器所在的线程中使用（它自己本身非 Send）
///
/// 如果你想在另一个线程中生成任务，请使用 [SendSpawner]。
///
/// ---
/// Handle to spawn tasks into an executor.
///
/// This Spawner can spawn any task (Send and non-Send ones), but it can
/// only be used in the executor thread (it is not Send itself).
///
/// If you want to spawn tasks from another thread, use [SendSpawner].
#[derive(Copy, Clone)]
pub struct Spawner {
    executor: &'static raw::Executor,
    not_send: PhantomData<*mut ()>,
}

impl Spawner {
    pub(crate) fn new(executor: &'static raw::Executor) -> Self {
        Self {
            executor,
            not_send: PhantomData,
        }
    }

    /// 从当前执行器中获取生成器
    ///
    /// 这个函数是 `async` 的，仅是为了访问当前异步上下文。它会立即返回，不会阻塞。
    ///
    /// #异常
    /// 如果当前执行器不是的 Embassy 执行器就会发生异常。
    ///
    /// ---
    /// Get a Spawner for the current executor.
    ///
    /// This function is `async` just to get access to the current async
    /// context. It returns instantly, it does not block/yield.
    ///
    /// # Panics
    ///
    /// Panics if the current executor is not an Embassy executor.
    pub async fn for_current_executor() -> Self {
        poll_fn(|cx| {
            let task = raw::task_from_waker(cx.waker());
            let executor = unsafe { task.header().executor.get().unwrap_unchecked() };
            let executor = unsafe { raw::Executor::wrap(executor) };
            Poll::Ready(Self::new(executor))
        })
        .await
    }

    /// 创建一个任务到执行器中
    ///
    /// 你可以调用任务函数来获取`token`（比如一个被`#[embassy_executor::task]`标记的函数）
    ///
    /// ---
    /// Spawn a task into an executor.
    ///
    /// You obtain the `token` by calling a task function (i.e. one marked with `#[embassy_executor::task]`).
    pub fn spawn<S>(&self, token: SpawnToken<S>) -> Result<(), SpawnError> {
        let task = token.raw_task;
        mem::forget(token);

        match task {
            Some(task) => {
                unsafe { self.executor.spawn(task) };
                Ok(())
            }
            None => Err(SpawnError::Busy),
        }
    }

    // 被 `embassy_macros::main!` 宏使用，创建任务失败时抛出一个错误。
    // 这里允许有条件地使用 `defmt::unwrap!` 没有在 `embassy_macros`
    // 包中引入 `defmt` 特性，就需要使用 `-Z namespaced-features`。
    /// 创建一个任务到执行器中，失败时会发生异常。
    ///
    /// # 异常
    /// 如果创建失败就会异常
    ///
    /// ---
    // Used by the `embassy_macros::main!` macro to throw an error when spawn
    // fails. This is here to allow conditional use of `defmt::unwrap!`
    // without introducing a `defmt` feature in the `embassy_macros` package,
    // which would require use of `-Z namespaced-features`.
    /// Spawn a task into an executor, panicking on failure.
    ///
    /// # Panics
    ///
    /// Panics if the spawning fails.
    pub fn must_spawn<S>(&self, token: SpawnToken<S>) {
        unwrap!(self.spawn(token));
    }

    /// 将当前 Spawner 转换为 SendSpawner。这样就可以将生成器发送到其他线程，
    /// 但是这个 spawner 会就只能生成 Send 的任务。
    ///
    /// ---
    /// Convert this Spawner to a SendSpawner. This allows you to send the
    /// spawner to other threads, but the spawner loses the ability to spawn
    /// non-Send tasks.
    pub fn make_send(&self) -> SendSpawner {
        SendSpawner::new(&self.executor.inner)
    }
}

/// 在任意线程中，生成任务到执行器中的句柄
///
/// 这个生成器可以被任意现场使用（它是 Send 的），但是它只能创建 Send 的任务。
/// 这是因为生成器本质上是将任务 “发送” 给执行器的线程。
///
/// 如果你线创建 non-Send 的任务，请使用 [Spawner].
///
/// ---
/// Handle to spawn tasks into an executor from any thread.
///
/// This Spawner can be used from any thread (it is Send), but it can
/// only spawn Send tasks. The reason for this is spawning is effectively
/// "sending" the tasks to the executor thread.
///
/// If you want to spawn non-Send tasks, use [Spawner].
#[derive(Copy, Clone)]
pub struct SendSpawner {
    executor: &'static raw::SyncExecutor,
}

impl SendSpawner {
    pub(crate) fn new(executor: &'static raw::SyncExecutor) -> Self {
        Self { executor }
    }

    /// 从当前执行器中获取生成器
    ///
    /// 这个函数是 `async` 的，仅是为了访问当前异步上下文。它会立即返回，不会阻塞。
    ///
    /// #异常
    /// 如果当前执行器不是的 Embassy 执行器就会发生异常。
    ///
    /// ---
    /// Get a Spawner for the current executor.
    ///
    /// This function is `async` just to get access to the current async
    /// context. It returns instantly, it does not block/yield.
    ///
    /// # Panics
    ///
    /// Panics if the current executor is not an Embassy executor.
    pub async fn for_current_executor() -> Self {
        poll_fn(|cx| {
            let task = raw::task_from_waker(cx.waker());
            let executor = unsafe { task.header().executor.get().unwrap_unchecked() };
            Poll::Ready(Self::new(executor))
        })
        .await
    }

    /// 创建一个任务到执行器中
    ///
    /// 你可以调用任务函数来获取`token`（比如一个被`#[embassy_executor::task]`标记的函数）
    ///
    /// ---
    /// Spawn a task into an executor.
    ///
    /// You obtain the `token` by calling a task function (i.e. one marked with `#[embassy_executor::task]`).
    pub fn spawn<S: Send>(&self, token: SpawnToken<S>) -> Result<(), SpawnError> {
        let header = token.raw_task;
        mem::forget(token);

        match header {
            Some(header) => {
                unsafe { self.executor.spawn(header) };
                Ok(())
            }
            None => Err(SpawnError::Busy),
        }
    }

    /// 创建一个任务到执行器中，失败时会发生异常。
    ///
    /// # 异常
    /// 如果创建失败就会异常
    ///
    /// ---
    /// Spawn a task into an executor, panicking on failure.
    ///
    /// # Panics
    ///
    /// Panics if the spawning fails.
    pub fn must_spawn<S: Send>(&self, token: SpawnToken<S>) {
        unwrap!(self.spawn(token));
    }
}
