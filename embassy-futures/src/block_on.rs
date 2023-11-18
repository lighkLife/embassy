use core::future::Future;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

static VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RawWaker::new(ptr::null(), &VTABLE), |_| {}, |_| {}, |_| {});

/// 以忙循环（busy loop）的方式运行一个future直到其完成
///
/// block_on函数接收一个实现了Future trait的参数fut，
/// 然后在一个忙循环中调用.poll()方法，这会阻塞当前线程并占用100%的CPU，直到Future完成。
/// future中的Waker机制没有被使用。 （？？？但是你下面不是用了Waker了吗）
///
/// 您可以使用这个来并发地运行多个future，例如[join][crate::join]。 
///
/// 这适合于没有或只有有限并发需求的系统，对功耗没有严格要求。
/// 对于更复杂的用例，更推荐使用像`embassy-executor`这样的"真正"的执行器，
/// 它支持多任务，并在没有任务需要工作时让核心进入睡眠状态。
pub fn block_on<F: Future>(mut fut: F) -> F::Output {
    // safety: 在这一行之后，我们不会move这个future
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };

    let raw_waker = RawWaker::new(ptr::null(), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(res) = fut.as_mut().poll(&mut cx) {
            return res;
        }
    }
}

/// poll一次这个future
pub fn poll_once<F: Future>(mut fut: F) -> Poll<F::Output> {
    // safety: 在这一行之后，我们不会move这个future
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };

    let raw_waker = RawWaker::new(ptr::null(), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut cx = Context::from_waker(&waker);

    fut.as_mut().poll(&mut cx)
}
