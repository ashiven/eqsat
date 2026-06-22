use crate::RuleSet;
use crate::mim_egg::Mim;
use crate::mim_egg::RULESETS;
use crate::mim_egg::analysis::MimAnalysis;
use egg::Rewrite;

pub mod core;
pub mod math;
// AUTOGEN START: egg-ruleset-rust-mod
// AUTOGEN END: egg-ruleset-rust-mod

pub fn get_rules() -> Vec<Rewrite<Mim, MimAnalysis>> {
    let mut rules = Vec::new();

    #[allow(clippy::single_match)]
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            match *ruleset {
                RuleSet::Core => rules.extend(core::rules()),
                RuleSet::Math => rules.extend(math::rules()),
                // AUTOGEN START: slotted-ruleset-rust-match
                // AUTOGEN END: slotted-ruleset-rust-match
                _ => (),
            }
        }
    });

    rules
}
