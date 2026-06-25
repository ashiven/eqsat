use crate::ffi::bridge::bridge::RuleSet;
use crate::slotted::Mim;
use crate::slotted::RULESETS;
use crate::slotted::analysis::MimAnalysis;
use slotted_egraphs::*;

pub mod normalize;
pub mod rise;
pub mod standard;
// AUTOGEN START: slotted-ruleset-rust-mod
// AUTOGEN END: slotted-ruleset-rust-mod

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
