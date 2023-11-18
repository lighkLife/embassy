use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// 让当前的任务（task）让步（yield）一次，并允许其他的任务先运行
///
/// 这可以用来轻松快速地实现简单的异步原语，而无需使用唤醒器（wakers）。
/// 下面的代码片段将等待某个条件成立，同时还允许其他任务并发运行（不会垄断执行器线程）。
///
/// ```rust,no_run
/// while !some_condition() {
///     yield_now().await;
/// }
/// ```
///
/// 但是，这种方法的缺点是它会在忙循环（busy loop）中一直不停工作，然后占用100%的CPU，
/// 而如果正确地使用唤醒器可以在等待时让CPU休眠来避免这种情况。
///
/// 内部实现是：
/// 在第一次轮询时，future唤醒自身并返回`Poll::Pending`。
/// 而在第二次轮询时，它会返回`Poll::Ready`
pub fn yield_now() -> impl Future<Output = ()> {
    YieldNowFuture { yielded: false }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
struct YieldNowFuture {
    yielded: bool,
}

impl Future for YieldNowFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
