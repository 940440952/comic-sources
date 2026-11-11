#![cfg_attr(target_arch = "wasm32", no_std)]
extern crate alloc;

pub mod parser;

#[cfg(target_arch = "wasm32")]
mod source;
