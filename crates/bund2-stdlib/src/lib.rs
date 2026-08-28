//! The standard library: native words, effects, and JIT lowerings.

#![forbid(unsafe_code)]

pub mod stack;

pub mod console;

/// Register everything this crate provides.
pub fn register_all(r: &mut bund2_api::Registry) {
    stack::register(r);
    console::register(r);
}
