use crate::RuleSet;
use crate::mim_egg::Mim;
use crate::mim_egg::analysis::MimAnalysis;
use egg::Rewrite;

pub mod core;
pub mod math;
// AUTOGEN START: egg-ruleset-rust-mod
// AUTOGEN END: egg-ruleset-rust-mod

pub fn get_rules(rulesets: Vec<RuleSet>) -> Vec<Rewrite<Mim, MimAnalysis>> {
    let mut rules = Vec::new();

    for ruleset in rulesets {
        match ruleset {
            RuleSet::Core => rules.extend(core::rules()),
            RuleSet::Math => rules.extend(math::rules()),
            // AUTOGEN START: egg-ruleset-rust-match
            // AUTOGEN END: egg-ruleset-rust-match
            _ => (),
        }
    }

    rules
}
