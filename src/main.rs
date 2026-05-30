#[allow(unused_imports)]
use eqsat_rs::{
    eqsat_egg, eqsat_slotted,
    ffi::bridge::{CostFn, OptionSelected, RuleSet},
    pretty_ffi,
};
use std::fs;

fn main() {
    let example = fs::read_to_string("./examples/import.slotted").expect("Failed to read file.");
    let selected = OptionSelected::none();
    let rec_exprs = eqsat_slotted(&example, selected, vec![], CostFn::AstSize);

    print!("{}", pretty_ffi(rec_exprs, 80));
}
