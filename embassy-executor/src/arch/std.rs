#[cfg(feature = "executor-interrupt")]
compile_error!("`executor-interrupt` is not supported with `arch-std`.");

#[cfg(feature = "executor-thread")]
pub use thread::*;
#[cfg(feature = "executor-thread")]
mod thread {
    use std::marker::PhantomData;
    use std::sync::{Condvar, Mutex};

    #[cfg(feature = "nightly")]
    pub use embassy_macros::main_std as main;

    use crate::{raw, Spawner};

    #[export_name = "__pender"]
    fn __pender(context: *mut ()) {
        let signaler: &'static Signaler = unsafe { std::mem::transmute(context) };
        signaler.signal()
    }

    /// std 环境下的单线程执行器
    ///
    /// ---
    /// Single-threaded std-based executor.
    pub struct Executor {
        inner: raw::Executor,
        not_send: PhantomData<*mut ()>,
        signaler: &'static Signaler,
    }

    impl Executor {
        /// Create a new Executor.
        pub fn new() -> Self {
            let signaler = Box::leak(Box::new(Signaler::new()));
            Self {
                inner: raw::Executor::new(signaler as *mut Signaler as *mut ()),
                not_send: PhantomData,
                signaler,
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
                unsafe { self.inner.poll() };
                self.signaler.wait()
            }
        }
    }

    struct Signaler {
        mutex: Mutex<bool>,
        condvar: Condvar,
    }

    impl Signaler {
        fn new() -> Self {
            Self {
                mutex: Mutex::new(false),
                condvar: Condvar::new(),
            }
        }

        fn wait(&self) {
            let mut signaled = self.mutex.lock().unwrap();
            while !*signaled {
                signaled = self.condvar.wait(signaled).unwrap();
            }
            *signaled = false;
        }

        fn signal(&self) {
            let mut signaled = self.mutex.lock().unwrap();
            *signaled = true;
            self.condvar.notify_one();
        }
    }
}
