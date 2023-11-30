use core::mem;
use core::task::{RawWaker, RawWakerVTable, Waker};

use super::{wake_task, TaskHeader, TaskRef};

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

unsafe fn clone(p: *const ()) -> RawWaker {
    RawWaker::new(p, &VTABLE)
}

unsafe fn wake(p: *const ()) {
    wake_task(TaskRef::from_ptr(p as *const TaskHeader))
}

unsafe fn drop(_: *const ()) {
    // nop
}

pub(crate) unsafe fn from_task(p: TaskRef) -> Waker {
    Waker::from_raw(RawWaker::new(p.as_ptr() as _, &VTABLE))
}

/// 从唤醒器（waker）中获取任务指针。
/// Get a task pointer from a waker.
///
/// 此方法可以用于优化等待队列，队列项只需要存储任务指针（一个机器字），不需要存储整个唤醒器（两个机器字）。
/// 可以节省一些内存，避免动态分配。
///
///
/// 可以使用返回的任务指针 [`wake_task`](super::wake_task) 来唤醒任务。
///
/// # Panics
///
/// 如果唤醒器不是由 Embassy 执行者创建的，就会 Panics
///
///
/// ---
/// Get a task pointer from a waker.
///
/// This can be used as an optimization in wait queues to store task pointers
/// (1 word) instead of full Wakers (2 words). This saves a bit of RAM and helps
/// avoid dynamic dispatch.
///
/// You can use the returned task pointer to wake the task with [`wake_task`](super::wake_task).
///
/// # Panics
///
/// Panics if the waker is not created by the Embassy executor.
pub fn task_from_waker(waker: &Waker) -> TaskRef {
    // 安全的： 可以正常执行，因为 WakerHack 和 Waker 的的布局相同。
    // 这并不能真正的保证永远执行成功，因为这个结构体是 `repr(Rust)` ，在当前的实现中是OK的
    //
    //---
    // safety: OK because WakerHack has the same layout as Waker.
    // This is not really guaranteed because the structs are `repr(Rust)`, it is
    // indeed the case in the current implementation.
    // TODO use waker_getters when stable. https://github.com/rust-lang/rust/issues/96992
    let hack: &WakerHack = unsafe { mem::transmute(waker) };
    if hack.vtable != &VTABLE {
        panic!("Found waker not created by the Embassy executor. `embassy_time::Timer` only works with the Embassy executor.")
    }

    // 安全的： 我们的 wakers 都是使用 `TaskRef::as_ptr` 创建的
    // safety: our wakers are always created with `TaskRef::as_ptr`
    unsafe { TaskRef::from_ptr(hack.data as *const TaskHeader) }
}

struct WakerHack {
    data: *const (),
    vtable: &'static RawWakerVTable,
}
