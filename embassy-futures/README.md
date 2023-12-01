# embassy-futures

一个 Embassy 项目

提供与 futures 相关的工具，兼容 `no_std`且未使用 `alloc`。针对代码大小进行了优化，适合嵌入式系统。

- Future 组件，如 `join` 和 `select` 函数

- 在没有完整的执行器（executor）的情况下使用 `async` 的工具：`block_on` 和 `yield_now`。

## Interoperability 互操作性

本库中的 futures 可以在任何执行器上运行。

## 最低支持的 Rust 版本 (MSRV)

Embassy 保证在发布时的最新稳定版 Rust 上编译。它可能也支持在旧版本上编译，但这可能在任何新的补丁发布后发生改变（失效）。

## License

根据以下任一许可证授权

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

任您选择









---





# embassy-futures

An [Embassy](https://embassy.dev) project.

Utilities for working with futures, compatible with `no_std` and not using `alloc`. Optimized for code size,
ideal for embedded systems

- Future combinators, like [`join`](join) and [`select`](select)
- Utilities to use `async` without a fully fledged executor: [`block_on`](block_on::block_on) and [`yield_now`](yield_now::yield_now).

## Interoperability

Futures from this crate can run on any executor.

## Minimum supported Rust version (MSRV)

Embassy is guaranteed to compile on the latest stable Rust version at the time of release. It might compile with older versions but that may change in any new patch release.

## License

This work is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

