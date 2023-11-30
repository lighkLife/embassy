use core::ptr;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, Ordering};

use super::{TaskHeader, TaskRef};
use crate::raw::util::SyncUnsafeCell;

pub(crate) struct RunQueueItem {
    next: SyncUnsafeCell<Option<TaskRef>>,
}

impl RunQueueItem {
    pub const fn new() -> Self {
        Self {
            next: SyncUnsafeCell::new(None),
        }
    }
}

/// 原子任务队列，使用一个非常简单的无锁链表队列实现。
///
/// 入队任务时，task.next 指向旧队头，任务自动成为队头。
///
/// 出队是批量操作的：通过将队头替换为null来清空队列，然后遍历出队的任务。
///
/// 注意出队后遍历的顺序与入队顺序相反。这对于我们的目的来说是可接受的：这样不会产生公平性问题，
/// 因为在当前批次完成之前，不会进行下一个批次的操作，因此，即使某个任务马上又入队（例如使用自己的唤醒器唤醒自己）
/// 也不会阻止当前批次的任务继续运行
///
/// ---
/// Atomic task queue using a very, very simple lock-free linked-list queue:
///
/// To enqueue a task, task.next is set to the old head, and head is atomically set to task.
///
/// Dequeuing is done in batches: the queue is emptied by atomically replacing head with
/// null. Then the batch is iterated following the next pointers until null is reached.
///
/// Note that batches will be iterated in the reverse order as they were enqueued. This is OK
/// for our purposes: it can't create fairness problems since the next batch won't run until the
/// current batch is completely processed, so even if a task enqueues itself instantly (for example
/// by waking its own waker) can't prevent other tasks from running.
pub(crate) struct RunQueue {
    head: AtomicPtr<TaskHeader>,
}

impl RunQueue {
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// 入队一个元素。若队列空，则返回 true
    ///
    /// # 安全的
    ///
    /// `item` 不能以及在任何队列中。
    ///
    /// ---
    /// Enqueues an item. Returns true if the queue was empty.
    ///
    /// # Safety
    ///
    /// `item` must NOT be already enqueued in any queue.
    #[inline(always)]
    pub(crate) unsafe fn enqueue(&self, task: TaskRef) -> bool {
        let mut was_empty = false;

        self.head
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev| {
                was_empty = prev.is_null();
                unsafe {
                    // 安全的：这个指正只能是 null 或 有效
                    // safety: the pointer is either null or valid
                    let prev = NonNull::new(prev).map(|ptr| TaskRef::from_ptr(ptr.as_ptr()));
                    // 安全的： 这里不存在并发访问 `next`
                    // safety: there are no concurrent accesses to `next`
                    task.header().run_queue_item.next.set(prev);
                }
                Some(task.as_ptr() as *mut _)
            })
            .ok();

        was_empty
    }

    /// 清空队列，然后对原队列中的每一个任务调用 `on_task`
    /// 注意： `on_task` 可以让更多的任务入队，在这种情况下，它们被留在队列中，然后在*下一次*调用 `dequeue_all`时
    /// 被处理，而*不是*当前
    ///
    /// ---
    /// Empty the queue, then call `on_task` for each task that was in the queue.
    /// NOTE: It is OK for `on_task` to enqueue more tasks. In this case they're left in the queue
    /// and will be processed by the *next* call to `dequeue_all`, *not* the current one.
    pub(crate) fn dequeue_all(&self, on_task: impl Fn(TaskRef)) {
        // 自动清空队列
        // Atomically empty the queue.
        let ptr = self.head.swap(ptr::null_mut(), Ordering::AcqRel);

        // 安全的： 这个指正只能是 null 或者有效
        // safety: the pointer is either null or valid
        let mut next = unsafe { NonNull::new(ptr).map(|ptr| TaskRef::from_ptr(ptr.as_ptr())) };

        // 便利之前在队列中的任务链表
        // Iterate the linked list of tasks that were previously in the queue.
        while let Some(task) = next {
            // 如果任务重新入队它自己， `next` 会被覆盖。
            // 因此，现读取 `next` 指针，然后再处理任务。
            // 安全的：这里不存在并发访问 `next`
            // If the task re-enqueues itself, the `next` pointer will get overwritten.
            // Therefore, first read the next pointer, and only then process the task.
            // safety: there are no concurrent accesses to `next`
            next = unsafe { task.header().run_queue_item.next.get() };

            on_task(task);
        }
    }
}
