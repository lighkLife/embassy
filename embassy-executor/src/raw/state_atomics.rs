use core::sync::atomic::{AtomicU32, Ordering};

/// 任务被创建（拥有一个 future）
///
/// ---
/// Task is spawned (has a future)
pub(crate) const STATE_SPAWNED: u32 = 1 << 0;
/// 任务在执行器的运行队列中
///
/// ---
/// Task is in the executor run queue
pub(crate) const STATE_RUN_QUEUED: u32 = 1 << 1;
/// 任务在执行器的时间队列中
///
/// ---
/// Task is in the executor timer queue
#[cfg(feature = "integrated-timers")]
pub(crate) const STATE_TIMER_QUEUED: u32 = 1 << 2;

pub(crate) struct State {
    state: AtomicU32,
}

impl State {
    pub const fn new() -> State {
        Self {
            state: AtomicU32::new(0),
        }
    }

    /// 如果任务空闲，将其标记为 spawned + run_queued，再返回
    /// If task is idle, mark it as spawned + run_queued and return true.
    #[inline(always)]
    pub fn spawn(&self) -> bool {
        self.state
            .compare_exchange(0, STATE_SPAWNED | STATE_RUN_QUEUED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// 取消任务的 spawned 标记
    /// Unmark the task as spawned.
    #[inline(always)]
    pub fn despawn(&self) {
        self.state.fetch_and(!STATE_SPAWNED, Ordering::AcqRel);
    }

    /// 如果任务已经被创建，并且没有在运行队列中，将其标记为 run-queued。如果成功，返回 true
    /// Mark the task as run-queued if it's spawned and isn't already run-queued. Return true on success.
    #[inline(always)]
    pub fn run_enqueue(&self) -> bool {
        self.state
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |state| {
                // If already scheduled, or if not started,
                if (state & STATE_RUN_QUEUED != 0) || (state & STATE_SPAWNED == 0) {
                    None
                } else {
                    // Mark it as scheduled
                    Some(state | STATE_RUN_QUEUED)
                }
            })
            .is_ok()
    }

    /// 取消任务的 run-queued 标记，返回任务是否被创建
    /// Unmark the task as run-queued. Return whether the task is spawned.
    #[inline(always)]
    pub fn run_dequeue(&self) -> bool {
        let state = self.state.fetch_and(!STATE_RUN_QUEUED, Ordering::AcqRel);
        state & STATE_SPAWNED != 0
    }

    /// 将任务标记为 timer-queued。返回任务是否新入队（之前没有入队过）
    /// Mark the task as timer-queued. Return whether it was newly queued (i.e. not queued before)
    #[cfg(feature = "integrated-timers")]
    #[inline(always)]
    pub fn timer_enqueue(&self) -> bool {
        let old_state = self.state.fetch_or(STATE_TIMER_QUEUED, Ordering::AcqRel);
        old_state & STATE_TIMER_QUEUED == 0
    }

    /// 取消任务的 timer-queued 标记
    /// Unmark the task as timer-queued.
    #[cfg(feature = "integrated-timers")]
    #[inline(always)]
    pub fn timer_dequeue(&self) {
        self.state.fetch_and(!STATE_TIMER_QUEUED, Ordering::AcqRel);
    }
}
