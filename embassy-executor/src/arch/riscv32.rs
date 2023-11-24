#[cfg(feature = "executor-interrupt")]
compile_error!("`executor-interrupt` is not supported with `arch-riscv32`.");

#[cfg(feature = "executor-thread")]
pub use thread::*;
#[cfg(feature = "executor-thread")]
mod thread {
    use core::marker::PhantomData;

    #[cfg(feature = "nightly")]
    pub use embassy_macros::main_riscv as main;
    use portable_atomic::{AtomicBool, Ordering};

    use crate::{raw, Spawner};

    /// 全局的原子变量，用来追踪当前是否有活可干，使用它是因为 sev() 在 RISCV 平台上不可用
    ///
    /// ---
    /// global atomic used to keep track of whether there is work to do since sev() is not available on RISCV
    static SIGNAL_WORK_THREAD_MODE: AtomicBool = AtomicBool::new(false);

    #[export_name = "__pender"]
    fn __pender(_context: *mut ()) {
        SIGNAL_WORK_THREAD_MODE.store(true, Ordering::SeqCst);
    }

    /// RISCV32 执行器
    ///
    /// ---
    /// RISCV32 Executor
    pub struct Executor {
        inner: raw::Executor,
        not_send: PhantomData<*mut ()>,
    }

    impl Executor {
        /// 创建一个新的执行器
        ///
        /// ---
        /// Create a new Executor.
        pub fn new() -> Self {
            Self {
                inner: raw::Executor::new(core::ptr::null_mut()),
                not_send: PhantomData,
            }
        }

        /// 启动执行器
        ///
        /// 这里调用 `init` 闭包，通过 [`Spawner`] 在当前执行器上生成任务。使用它来生成初始任务。
        /// `init` 返回后，执行器开始运行任务。
        ///
        /// 为了后面生成其他的任务，你可以保存 [`Spawner``]的副本(它是 `Copy`的)，例如，通过将其
        /// 作为参数传递给初始任务。
        ///
        /// 这个函数需要 `&'static mut self`。换而言之， Executor 实例需要永久存储，并允许可变访问。
        /// 下面这些方法可以实现：
        ///
        /// - 使用 [StaticCell](https://docs.rs/static_cell/latest/static_cell/) (safe)
        /// - 使用 `static mut` (unsafe)
        /// - 保存在局变量中，但需要当前函数是永远不会返回的（比如 `fn main() -> !`），使用 `transmute` 来升级他的生命周期（unsafe）。
        ///
        /// 这个函数永远不会返回
        ///
        /// ---
        /// Run the executor.
        ///
        /// The `init` closure is called with a [`Spawner`] that spawns tasks on
        /// this executor. Use it to spawn the initial task(s). After `init` returns,
        /// the executor starts running the tasks.
        ///
        /// To spawn more tasks later, you may keep copies of the [`Spawner`] (it is `Copy`),
        /// for example by passing it as an argument to the initial tasks.
        ///
        /// This function requires `&'static mut self`. This means you have to store the
        /// Executor instance in a place where it'll live forever and grants you mutable
        /// access. There's a few ways to do this:
        ///
        /// - a [StaticCell](https://docs.rs/static_cell/latest/static_cell/) (safe)
        /// - a `static mut` (unsafe)
        /// - a local variable in a function you know never returns (like `fn main() -> !`), upgrading its lifetime with `transmute`. (unsafe)
        ///
        /// This function never returns.
        pub fn run(&'static mut self, init: impl FnOnce(Spawner)) -> ! {
            init(self.inner.spawner());

            loop {
                unsafe {
                    self.inner.poll();
                    // 我们无须担心 load 和 store 操作之间的竞争条件，因为中断只会把它设置为 true
                    // we do not care about race conditions between the load and store operations, interrupts
                    //will only set this value to true.
                    critical_section::with(|_| {
                        // 如果有活可干，进入下一轮循环去执行
                        // if there is work to do, loop back to polling
                        // TODO can we relax this?
                        if SIGNAL_WORK_THREAD_MODE.load(Ordering::SeqCst) {
                            SIGNAL_WORK_THREAD_MODE.store(false, Ordering::SeqCst);
                        }
                        // 没啥要干的，等待中断
                        // if not, wait for interrupt
                        else {
                            core::arch::asm!("wfi");
                        }
                    });
                    // 在等待的时候发生中断，程序将会执行到这里
                    // if an interrupt occurred while waiting, it will be serviced here
                }
            }
        }
    }
}
