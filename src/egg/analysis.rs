#![allow(unused_imports)]
use crate::egg::Mim;
use crate::egg::RULESETS;
use crate::egg::rulesets::core::{CoreData, core_make, core_merge, core_modify};
use crate::ffi::bridge::RuleSet;
// AUTOGEN START: egg-analysis-rust-import
// AUTOGEN END: egg-analysis-rust-import
use crate::egg::types::{TypeAnalysis, TypeData};
use egg::*;

#[derive(Default, Clone)]
pub struct MimAnalysis;
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct AnalysisData {
    pub type_: Option<TypeData>,
    pub core_data: Option<CoreData>,
    // AUTOGEN START: egg-analysis-rust-data
    // AUTOGEN END: egg-analysis-rust-data
}

impl AnalysisData {
    fn combine(&mut self, other: AnalysisData) {
        self.type_ = self.type_.take().or(other.type_);
        // AUTOGEN START: egg-analysis-rust-combine
        // AUTOGEN END: egg-analysis-rust-combine
    }
}

fn combined_make(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let mut combined_data = AnalysisData::default();

    // Analyses applied for all rulesets
    let type_data = TypeAnalysis::make(eg, enode);
    combined_data.combine(type_data);

    // Ruleset-specific analyses
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            #[allow(clippy::match_single_binding)]
            match *ruleset {
                // AUTOGEN START: slotted-analysis-rust-make
                // AUTOGEN END: slotted-analysis-rust-make
                _ => (),
            };
        }
    });

    combined_data
}

fn combined_merge(l: &mut AnalysisData, r: AnalysisData) -> DidMerge {
    let mut combined_merge = DidMerge(false, false);

    // Analyses applied for all rulesets
    let type_merge = TypeAnalysis::merge(l, r.clone());
    combined_merge = combined_merge | type_merge;

    // Ruleset-specific analyses
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            #[allow(clippy::match_single_binding)]
            match *ruleset {
                // AUTOGEN START: slotted-analysis-rust-merge
                // AUTOGEN END: slotted-analysis-rust-merge
                _ => (),
            };
        }
    });

    combined_merge
}

impl Analysis<Mim> for MimAnalysis {
    type Data = AnalysisData;

    fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
        // core_merge(a, b)
        combined_merge(a, b)
    }

    fn make(egraph: &mut EGraph<Mim, Self>, enode: &Mim, _id: Id) -> Self::Data {
        // core_make(egraph, enode, _id)
        combined_make(egraph, enode)
    }

    fn modify(_egraph: &mut EGraph<Mim, Self>, _id: Id) {
        // core_modify(egraph, id)
    }
}
