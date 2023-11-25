//! 等待多个future中第一个完成的future <br />
//! Wait for the first of several futures to complete.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// [`select`]的输出结果 <br />
/// Result for [`select`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Either<A, B> {
    /// 第一个future先完成 <br />
    /// First future finished first.
    First(A),
    /// 第二个future先完成 <br />
    /// Second future finished first.
    Second(B),
}

/// 等待两个future中的其中一个完成
///
/// 这个函数返回一个新的future，它会轮询所有的futures。
/// 只要他们其中有一个已经运行结束，就立即可以作为结果返回
///
/// 其他的future会被丢弃
/// ---
/// Wait for one of two futures to complete.
///
/// This function returns a new future which polls all the futures.
/// When one of them completes, it will complete with its result value.
///
/// The other future is dropped.
pub fn select<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future,
    B: Future,
{
    Select { a, b }
}

/// 用于 [`select`] 函数的Future。 <br />
/// Future for the [`select`] function.
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

/// [`select3`]的输出结果。 <br />
/// Result for [`select3`]. 
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Either3<A, B, C> {
    /// 第一个future先完成 <br />
    /// First future finished first.
    First(A),
    /// 第二个future先完成 <br />
    /// Second future finished first.
    Second(B),
    /// 第三个future先完成 <br />
    /// Third future finished first.
    Third(C),
}

/// 跟[`select`]一样, 只不过支持更多个future。<br />
/// Same as [`select`], but with more futures.
pub fn select3<A, B, C>(a: A, b: B, c: C) -> Select3<A, B, C>
where
    A: Future,
    B: Future,
    C: Future,
{
    Select3 { a, b, c }
}

/// 用于 [`select3`] 函数的Future。 <br />
/// Future for the [`select3`] function.
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

/// [`select4`]的输出结果。 <br />
/// Result for [`select4`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Either4<A, B, C, D> {
    /// 第一个future先完成 <br />
    /// First future finished first.
    First(A),
    /// 第二个future先完成 <br />
    /// Second future finished first.
    Second(B),
    /// 第三个future先完成 <br />
    /// Third future finished first.
    Third(C),
    /// 第四个future先完成 <br />
    /// Fourth future finished first.
    Fourth(D),
}

/// 跟[`select`]一样, 只不过支持更多个future。 <br />
/// Same as [`select`], but with more futures.
pub fn select4<A, B, C, D>(a: A, b: B, c: C, d: D) -> Select4<A, B, C, D>
where
    A: Future,
    B: Future,
    C: Future,
    D: Future,
{
    Select4 { a, b, c, d }
}

/// 用于 [`select4`] 函数的Future。 <br />
/// Future for the [`select4`] function.
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

/// 用于 [`select_array`] 函数的Future。 <br />
/// Future for the [`select_array`] function.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct SelectArray<Fut, const N: usize> {
    inner: [Fut; N],
}

/// 创建一个新的 future，它将在 所有future组成的数组中 选择合适的future。
///
/// 返回的future将会等待任何一个future准备就绪，
/// 一旦其完成，将返回解析的项目，以及准备就绪的 future 的索引。
///
///  如果数组为空，那么结果 future 将永远处于 Pending 状态。
/// 
/// ---
/// Creates a new future which will select over an array of futures.
///
/// The returned future will wait for any future to be ready. Upon
/// completion the item resolved will be returned, along with the index of the
/// future that was ready.
///
/// If the array is empty, the resulting future will be Pending forever.
pub fn select_array<Fut: Future, const N: usize>(arr: [Fut; N]) -> SelectArray<Fut, N> {
    SelectArray { inner: arr }
}

impl<Fut: Future, const N: usize> Future for SelectArray<Fut, N> {
    type Output = (Fut::Output, usize);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 由于 self 被固定（pinned），inner 无法move。由于 inner 无法move，其元素也无法move。
        // 因此，访问 inner 并固定（pin）对包含的 futures 的引用是安全的。”
        // Safety: Since `self` is pinned, `inner` cannot move. Since `inner` cannot move,
        // its elements also cannot move. Therefore it is safe to access `inner` and pin
        // references to the contained futures.
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

/// 用于 [`select_slice`] 函数的Futrue。 <br />
/// Future for the [`select_slice`] function.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct SelectSlice<'a, Fut> {
    inner: &'a mut [Fut],
}

/// 创建一个新的 future，它将在 所有future组成的切片中 选择合适的future。
///
/// 返回的future将会等待任何一个future准备就绪，
/// 一旦其完成，将返回解析的项目，以及准备就绪的 future 的索引。
///
///  如果切片为空，那么结果 future 将永远处于 Pending 状态。
///
/// ---
/// Creates a new future which will select over a slice of futures.
///
/// The returned future will wait for any future to be ready. Upon
/// completion the item resolved will be returned, along with the index of the
/// future that was ready.
///
/// If the slice is empty, the resulting future will be Pending forever.
pub fn select_slice<'a, Fut: Future>(slice: &'a mut [Fut]) -> SelectSlice<'a, Fut> {
    SelectSlice { inner: slice }
}

impl<'a, Fut: Future> Future for SelectSlice<'a, Fut> {
    type Output = (Fut::Output, usize);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 由于 self 被固定（pinned），inner 无法move。由于 inner 无法move，其元素也无法move。
        // 因此，访问 inner 并固定（pin）对包含的 futures 的引用是安全的。”
        // Safety: Since `self` is pinned, `inner` cannot move. Since `inner` cannot move,
        // its elements also cannot move. Therefore it is safe to access `inner` and pin
        // references to the contained futures.
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
