use crate::RuleSet;
use crate::egg::Mim;
use crate::egg::analysis::MimAnalysis;
use egg::Rewrite;
use std::cell::RefCell;

thread_local! {
    pub static RULESETS: RefCell<Vec<RuleSet>> = const { RefCell::new(vec![]) };
}

pub mod core;
pub mod math;
// AUTOGEN START: egg-ruleset-rust-mod
// AUTOGEN END: egg-ruleset-rust-mod

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
                RuleSet::Core => rules.extend(core::rules()),
                RuleSet::Math => rules.extend(math::rules()),
                // AUTOGEN START: egg-ruleset-rust-match
                // AUTOGEN END: egg-ruleset-rust-match
                _ => (),
            }
        }
    });

    rules
}
