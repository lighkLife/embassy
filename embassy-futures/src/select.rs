//! 等待多个future中第一个完成的

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// [`select`]的输出结果
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Either<A, B> {
    /// 第一个future先完成
    First(A),
    /// 第二个future先完成
    Second(B),
}

/// 等待两个future中的其中一个完成
///
/// This function returns a new future which polls all the futures.
/// When one of them completes, it will complete with its result value.
/// 这个函数返回一个新的future，它会轮询所有的futures。
/// 只要他们其中的一个返回，就立即可以作为结果返回
///
/// The other future is dropped.
pub fn select<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future,
    B: Future,
{
    Select { a, b }
}

/// [`select`]函数在Future中的实现
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Select<A, B> {
    a: A,
    b: B,
}

impl<A: Unpin, B: Unpin> Unpin for Select<A, B> {}

impl<A, B> Future for Select<A, B>
where
    A: Future,
    B: Future,
{
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let a = unsafe { Pin::new_unchecked(&mut this.a) };
        let b = unsafe { Pin::new_unchecked(&mut this.b) };
        if let Poll::Ready(x) = a.poll(cx) {
            return Poll::Ready(Either::First(x));
        }
        if let Poll::Ready(x) = b.poll(cx) {
            return Poll::Ready(Either::Second(x));
        }
        Poll::Pending
    }
}

// ====================================================================

/// [`select3`]的输出结果。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Either3<A, B, C> {
    /// 第一个future先完成
    First(A),
    /// 第二个future先完成
    Second(B),
    /// 第三个future先完成
    Third(C),
}

/// 跟[`select`]一样, 只不过支持更多个future
pub fn select3<A, B, C>(a: A, b: B, c: C) -> Select3<A, B, C>
where
    A: Future,
    B: Future,
    C: Future,
{
    Select3 { a, b, c }
}

/// [`select3`]函数的Future实现
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Select3<A, B, C> {
    a: A,
    b: B,
    c: C,
}

impl<A, B, C> Future for Select3<A, B, C>
where
    A: Future,
    B: Future,
    C: Future,
{
    type Output = Either3<A::Output, B::Output, C::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let a = unsafe { Pin::new_unchecked(&mut this.a) };
        let b = unsafe { Pin::new_unchecked(&mut this.b) };
        let c = unsafe { Pin::new_unchecked(&mut this.c) };
        if let Poll::Ready(x) = a.poll(cx) {
            return Poll::Ready(Either3::First(x));
        }
        if let Poll::Ready(x) = b.poll(cx) {
            return Poll::Ready(Either3::Second(x));
        }
        if let Poll::Ready(x) = c.poll(cx) {
            return Poll::Ready(Either3::Third(x));
        }
        Poll::Pending
    }
}

// ====================================================================

/// [`select4`]的输出结果
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Either4<A, B, C, D> {
    /// 第一个future先完成
    First(A),
    /// 第二个future先完成
    Second(B),
    /// 第三个future先完成
    Third(C),
    /// 第四个future先完成
    Fourth(D),
}

/// 跟[`select`]一样, 只不过支持更多个future
pub fn select4<A, B, C, D>(a: A, b: B, c: C, d: D) -> Select4<A, B, C, D>
where
    A: Future,
    B: Future,
    C: Future,
    D: Future,
{
    Select4 { a, b, c, d }
}

/// [`select4`]函数的Future实现
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Select4<A, B, C, D> {
    a: A,
    b: B,
    c: C,
    d: D,
}

impl<A, B, C, D> Future for Select4<A, B, C, D>
where
    A: Future,
    B: Future,
    C: Future,
    D: Future,
{
    type Output = Either4<A::Output, B::Output, C::Output, D::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let a = unsafe { Pin::new_unchecked(&mut this.a) };
        let b = unsafe { Pin::new_unchecked(&mut this.b) };
        let c = unsafe { Pin::new_unchecked(&mut this.c) };
        let d = unsafe { Pin::new_unchecked(&mut this.d) };
        if let Poll::Ready(x) = a.poll(cx) {
            return Poll::Ready(Either4::First(x));
        }
        if let Poll::Ready(x) = b.poll(cx) {
            return Poll::Ready(Either4::Second(x));
        }
        if let Poll::Ready(x) = c.poll(cx) {
            return Poll::Ready(Either4::Third(x));
        }
        if let Poll::Ready(x) = d.poll(cx) {
            return Poll::Ready(Either4::Fourth(x));
        }
        Poll::Pending
    }
}

// ====================================================================

/// [`select_array`]函数的Future实现
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct SelectArray<Fut, const N: usize> {
    inner: [Fut; N],
}

/// Creates a new future which will select over an array of futures.
///
/// The returned future will wait for any future to be ready. Upon
/// completion the item resolved will be returned, along with the index of the
/// future that was ready.
///
/// 创建一个新的 future，它将在 所有future组成的数组中 选择合适的future。
///
/// 返回的future将会等待任何一个future准备就绪，
/// 一旦其完成，将返回解析的项目，以及准备就绪的 future 的索引。
///
///  如果数组为空，那么结果 future 将永远处于 Pending 状态。
pub fn select_array<Fut: Future, const N: usize>(arr: [Fut; N]) -> SelectArray<Fut, N> {
    SelectArray { inner: arr }
}

impl<Fut: Future, const N: usize> Future for SelectArray<Fut, N> {
    type Output = (Fut::Output, usize);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: Since `self` is pinned, `inner` cannot move. Since `inner` cannot move,
        // its elements also cannot move. Therefore it is safe to access `inner` and pin
        // references to the contained futures.
        // 由于 self 被固定（pinned），inner 无法move。由于 inner 无法move，其元素也无法move。
        // 因此，访问 inner 并固定（pin）对包含的 futures 的引用是安全的。”
        let item = unsafe {
            self.get_unchecked_mut()
                .inner
                .iter_mut()
                .enumerate()
                .find_map(|(i, f)| match Pin::new_unchecked(f).poll(cx) {
                    Poll::Pending => None,
                    Poll::Ready(e) => Some((i, e)),
                })
        };

        match item {
            Some((idx, res)) => Poll::Ready((res, idx)),
            None => Poll::Pending,
        }
    }
}

// ====================================================================

/// [`select_slice`]函数的Futrue实现
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct SelectSlice<'a, Fut> {
    inner: &'a mut [Fut],
}

/// Creates a new future which will select over a slice of futures.
///
/// The returned future will wait for any future to be ready. Upon
/// completion the item resolved will be returned, along with the index of the
/// future that was ready.
///
/// If the slice is empty, the resulting future will be Pending forever.
/// 创建一个新的 future，它将在 所有future组成的切片中 选择合适的future。
///
/// 返回的future将会等待任何一个future准备就绪，
/// 一旦其完成，将返回解析的项目，以及准备就绪的 future 的索引。
///
///  如果切片为空，那么结果 future 将永远处于 Pending 状态。
pub fn select_slice<'a, Fut: Future>(slice: &'a mut [Fut]) -> SelectSlice<'a, Fut> {
    SelectSlice { inner: slice }
}

impl<'a, Fut: Future> Future for SelectSlice<'a, Fut> {
    type Output = (Fut::Output, usize);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: Since `self` is pinned, `inner` cannot move. Since `inner` cannot move,
        // its elements also cannot move. Therefore it is safe to access `inner` and pin
        // references to the contained futures.
        // 由于 self 被固定（pinned），inner 无法move。由于 inner 无法move，其元素也无法move。
        // 因此，访问 inner 并固定（pin）对包含的 futures 的引用是安全的。”
        let item = unsafe {
            self.get_unchecked_mut()
                .inner
                .iter_mut()
                .enumerate()
                .find_map(|(i, f)| match Pin::new_unchecked(f).poll(cx) {
                    Poll::Pending => None,
                    Poll::Ready(e) => Some((i, e)),
                })
        };

        match item {
            Some((idx, res)) => Poll::Ready((res, idx)),
            None => Poll::Pending,
        }
    }
}
