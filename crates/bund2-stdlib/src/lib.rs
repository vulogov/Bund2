//! The standard library: native words, effects, and JIT lowerings.

#![forbid(unsafe_code)]

pub mod stack;

pub mod console;

pub mod logic;

pub mod values;

/// Register everything this crate provides.
pub fn register_all(r: &mut bund2_api::Registry) {
    stack::register(r);
    console::register(r);
    logic::register(r);
    values::register_words(r);
}
