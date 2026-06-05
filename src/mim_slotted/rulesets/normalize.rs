use crate::mim_slotted::{MimSlotted, analysis::MimSlottedAnalysis};
use slotted_egraphs::Rewrite;

pub fn rules() -> Vec<Rewrite<MimSlotted, MimSlottedAnalysis>> {
    let rules = vec![
        normalize_three_tuple(),
        normalize_three_pack(),
        normalize_two_tuple(),
        normalize_two_pack(),
        core_mul_comm(),
    ];

    rules
}

fn normalize_three_tuple() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(tuple (cons ?a (cons ?a (cons ?a nil))))";
    let outpat = "(pack $dummy (scope (lit 3 Nat) ?a))";
    Rewrite::new("normalize-three-tuple", pat, outpat)
}

fn normalize_three_pack() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(pack $dummy (scope (lit 3 Nat) ?a))";
    let outpat = "(tuple (cons ?a (cons ?a (cons ?a nil))))";
    Rewrite::new("normalize-three-pack", pat, outpat)
}

fn normalize_two_tuple() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(tuple (cons ?a (cons ?a nil)))";
    let outpat = "(pack $dummy (scope (lit 2 Nat) ?a))";
    Rewrite::new("normalize-three-tuple", pat, outpat)
}

fn normalize_two_pack() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(pack $dummy (scope (lit 2 Nat) ?a))";
    let outpat = "(tuple (cons ?a (cons ?a nil)))";
    Rewrite::new("normalize-three-pack", pat, outpat)
}

fn core_mul_comm() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(app %core.nat.mul (tuple (cons ?a (cons ?b nil))))";
    let outpat = "(app %core.nat.mul (tuple (cons ?b (cons ?a nil))))";
    Rewrite::new("core-mul-comm", pat, outpat)
}
