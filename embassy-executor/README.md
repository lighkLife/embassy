# embassy-executor

为嵌入式使用而设计的 async/await 执行器。

- 没有`alloc`，不需要堆。任务的 futures 是静态分配的。
- 支持的任务数量可以改动，执行器可以处理1到1000个任务，而无需配置/调优。
- 集成定时器队列: 很容易就可以休眠，只需执行 `timer::after secs(1).await;`。
- 不会一直轮询：CPU在没有工作要做时，可以使用中断或`WFE/SEV`进入休眠状态。
- 有效轮询：执行器只会执行被唤醒的任务，而不是所有的任务。
- 公平：一个任务不能独占CPU时间，即使它被频繁唤醒。在给定任务第二次轮询之前，其他任务都有机会运行。
- 支持创建多个执行器实例，以运行具有不同优先级的任务。高优先级的任务可以抢占低优先级的任务。
  
---
An async/await executor designed for embedded usage.

- No `alloc`, no heap needed. Task futures are statically allocated.
- No "fixed capacity" data structures, executor works with 1 or 1000 tasks without needing config/tuning.
- Integrated timer queue: sleeping is easy, just do `Timer::after_secs(1).await;`.
- No busy-loop polling: CPU sleeps when there's no work to do, using interrupts or `WFE/SEV`.
- Efficient polling: a wake will only poll the woken task, not all of them.
- Fair: a task can't monopolize CPU time even if it's constantly being woken. All other tasks get a chance to run before a given task gets polled for the second time.
- Creating multiple executor instances is supported, to run tasks with multiple priority levels. This allows higher-priority tasks to preempt lower-priority tasks.
