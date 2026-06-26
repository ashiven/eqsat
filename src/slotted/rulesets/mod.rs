use crate::ffi::bridge::RuleSet;
use crate::slotted::Mim;
use crate::slotted::analysis::MimAnalysis;
use slotted_egraphs::*;
use std::cell::RefCell;

// We keep track of the selected rulesets in a global variable because they
// need to be accessed repeatedly in the analysis and it was too tedious to
// pass them on by parameters or otherwise.
thread_local! {
    pub static RULESETS: RefCell<Vec<RuleSet>> = const { RefCell::new(vec![]) };
}

pub mod normalize;
pub mod rise;
pub mod standard;
// AUTOGEN START: slotted-ruleset-rust-mod
// AUTOGEN END: slotted-ruleset-rust-mod

pub fn set_rulesets(rulesets: Vec<RuleSet>) {
    RULESETS.with(|rulesets_global| {
        let mut rulesets_global = rulesets_global.borrow_mut();
        *rulesets_global = rulesets;
    });
}

pub fn get_rules() -> Vec<Rewrite<Mim, MimAnalysis>> {
    let mut rules = Vec::new();

    #[allow(clippy::single_match)]
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            match *ruleset {
                RuleSet::Standard => rules.extend(standard::rules()),
                RuleSet::Rise => rules.extend(rise::rules()),
                RuleSet::Normalize => rules.extend(normalize::rules()),
                // AUTOGEN START: slotted-ruleset-rust-match
                // AUTOGEN END: slotted-ruleset-rust-match
                _ => (),
            }
        }
    });

    rules
}
