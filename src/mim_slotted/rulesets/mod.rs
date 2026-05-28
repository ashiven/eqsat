use crate::ffi::bridge::RuleSet;
use crate::mim_slotted::MimSlotted;
use crate::mim_slotted::RULESETS;
use crate::mim_slotted::analysis::MimSlottedAnalysis;
use slotted_egraphs::*;

pub mod rise;
pub mod standard;

pub fn get_rules() -> Vec<Rewrite<MimSlotted, MimSlottedAnalysis>> {
    let mut rules = Vec::new();

    #[allow(clippy::single_match)]
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            match *ruleset {
                RuleSet::Standard => rules.extend(standard::rules()),
                RuleSet::Rise => rules.extend(rise::rules()),
                _ => (),
            }
        }
    });

    rules
}
