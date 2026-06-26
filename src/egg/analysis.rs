#![allow(unused_imports)]
use crate::egg::Mim;
use crate::egg::RULESETS;
use crate::egg::rulesets::core::{CoreAnalysis, CoreData, core_make, core_merge, core_modify};
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
    pub core: Option<CoreData>,
    // AUTOGEN START: egg-analysis-rust-data
    // AUTOGEN END: egg-analysis-rust-data
}

impl AnalysisData {
    fn combine(&mut self, other: AnalysisData) {
        self.type_ = self.type_.take().or(other.type_);
        self.core = self.core.take().or(other.core);
        // AUTOGEN START: egg-analysis-rust-combine
        // AUTOGEN END: egg-analysis-rust-combine
    }
}

fn combined_make(eg: &mut EGraph<Mim, MimAnalysis>, enode: &Mim, id: Id) -> AnalysisData {
    let mut combined_data = AnalysisData::default();

    // Analyses applied for all rulesets
    let type_data = TypeAnalysis::make(eg, enode, id);
    combined_data.combine(type_data);

    // Ruleset-specific analyses
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            match *ruleset {
                RuleSet::Core => {
                    let data = CoreAnalysis::make(eg, enode, id);
                    combined_data.combine(data);
                }
                // AUTOGEN START: egg-analysis-rust-make
                // AUTOGEN END: egg-analysis-rust-make
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
        #[allow(unused_variables)]
        let combined_merge = &mut combined_merge;

        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            match *ruleset {
                RuleSet::Core => {
                    let merge = CoreAnalysis::merge(l, r.clone());
                    *combined_merge =
                        DidMerge(combined_merge.0 | merge.0, combined_merge.1 | merge.1);
                }
                // AUTOGEN START: egg-analysis-rust-merge
                // AUTOGEN END: egg-analysis-rust-merge
                _ => (),
            };
        }
    });

    combined_merge
}

impl Analysis<Mim> for MimAnalysis {
    type Data = AnalysisData;

    fn merge(&mut self, a: &mut Self::Data, b: Self::Data) -> DidMerge {
        combined_merge(a, b)
    }

    fn make(egraph: &mut EGraph<Mim, Self>, enode: &Mim, id: Id) -> Self::Data {
        combined_make(egraph, enode, id)
    }

    fn modify(egraph: &mut EGraph<Mim, Self>, id: Id) {
        // TODO: Need combined_modify implementation as above
        CoreAnalysis::modify(egraph, id)
    }
}
