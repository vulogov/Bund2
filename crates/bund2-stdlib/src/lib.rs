//! The standard library: native words, effects, and JIT lowerings.

#![forbid(unsafe_code)]

pub mod stack;

pub mod console;

pub mod logic;

pub mod values;

pub mod math;

pub mod control;

pub mod conditional;

pub mod report;

/// Register everything this crate provides.
pub fn register_all(r: &mut bund2_api::Registry) {
    stack::register(r);
    console::register(r);
    logic::register(r);
    values::register_words(r);
    math::register(r);
    control::register(r);
    conditional::register(r);
}
