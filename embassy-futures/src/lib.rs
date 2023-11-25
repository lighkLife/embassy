#![no_std]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

// 这个模块必须放在前面，这样其他模块才能看到它的宏。 <br />
// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

mod block_on;
mod yield_now;

pub mod join;
pub mod select;

pub use block_on::*;
pub use yield_now::*;
