//! **What §S8's floor check costs an entry, by count** — RFC-0005 criterion
//! 11, read the way D117 reads criterion 7.
//!
//! `floor_cost` in `benches/interpret.rs` takes the same difference by the
//! clock, and on 2026-10-12 its rows read between 34 ns faster and 21 ns
//! slower for the checked arm across three rounds: the clock does not resolve
//! one compare and branch. A count does. This enters one compiled body a
//! fixed number of times and exits, so that
//!
//!     /usr/bin/time -l floor_count checked   10000000 1
//!     /usr/bin/time -l floor_count unchecked 10000000 1
//!
//! differ in `instructions retired` by the check and nothing else, and the
//! difference over the entry count is the check's cost in instructions.
//!
//! Arguments: `checked` or `unchecked`, how many entries, and how many
//! `1 drop` pairs the body holds. It prints how many entries ran and how many
//! the floor declined; a run that declined any measured the decline.

use bund2_jit::lower::{Compiler, LastCall};
use std::hint::black_box;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let checked = match args.get(1).map(String::as_str) {
        Some("checked") => true,
        Some("unchecked") => false,
        _ => {
            bund2_api::errln!("usage: floor_count checked|unchecked <entries> <pairs>");
            return;
        }
    };
    let entries: u64 = args.get(2).and_then(|n| n.parse().ok()).unwrap_or(0);
    let pairs: usize = args.get(3).and_then(|n| n.parse().ok()).unwrap_or(1);

    let body = match bund2_syntax::compile(&"1 drop ".repeat(pairs)) {
        Ok(b) => b,
        Err(e) => {
            bund2_api::errln!("floor_count: the body did not parse: {e:?}");
            return;
        }
    };

    bund2_interp::set_stack_region_with_share(
        bund2_interp::stack_marker(),
        8 * 1024 * 1024,
        1024 * 1024,
    );
    let mut vm = bund2_runtime::Runtime::new().interp;
    let table = bund2_stdlib::fragments::published(&vm.registry).unwrap_or_default();
    let comp = if checked {
        Compiler::new(table)
    } else {
        Compiler::without_floor_check(table)
    };
    let Ok(mut comp) = comp else {
        bund2_api::errln!("floor_count: no compiler could be built");
        return;
    };
    let cells = vm.cells().base();
    let word = match comp.compile_word(&body, LastCall::Ordinary, cells, &mut vm) {
        Ok(w) => w,
        Err(e) => {
            bund2_api::errln!("floor_count: the body did not compile: {e}");
            return;
        }
    };

    let mut ran = 0u64;
    for _ in 0..entries {
        if black_box(comp.run(word, &mut vm, black_box(&body))) == Ok(true) {
            ran += 1;
        }
    }
    bund2_api::errln!(
        "floor_count: floor check {}, sites {:?}, {ran} of {entries} entries ran, \
         {} declined at the floor",
        comp.checks_the_floor(),
        comp.inlined_sites(word),
        comp.floor_declines()
    );
}
