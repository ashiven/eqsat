use crate::ffi::bridge::RuleSet;
use crate::slotted::types::{TypeAnalysis, TypeData};
// AUTOGEN START: slotted-analysis-rust-import
// AUTOGEN END: slotted-analysis-rust-import
use crate::slotted::{Mim, RULESETS};
use slotted_egraphs::*;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct MimAnalysis;

#[derive(Clone, Eq, PartialEq, Default)]
pub struct AnalysisData {
    pub type_: Option<TypeData>,
    // AUTOGEN START: slotted-analysis-rust-data
    // AUTOGEN END: slotted-analysis-rust-data
}

impl AnalysisData {
    fn combine(&mut self, other: AnalysisData) {
        self.type_ = self.type_.take().or(other.type_);
        // AUTOGEN START: slotted-analysis-rust-combine
        // AUTOGEN END: slotted-analysis-rust-combine
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
            match *ruleset {
                RuleSet::Rise => {
                    // let data = TypeAnalysis::make(eg, enode);
                    // combined_data.combine(data)
                }
                // AUTOGEN START: slotted-analysis-rust-make
                // AUTOGEN END: slotted-analysis-rust-make
                _ => (),
            };
        }
    });

    combined_data
}

fn combined_merge(l: AnalysisData, r: AnalysisData) -> AnalysisData {
    let mut combined_data = AnalysisData::default();

    // Analyses applied for all rulesets
    let type_data = TypeAnalysis::merge(l.clone(), r.clone());
    combined_data.combine(type_data);

    // Ruleset-specific analyses
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            match *ruleset {
                RuleSet::Rise => {
                    // let data = TypeAnalysis::merge(l.clone(), r.clone());
                    // combined_data.combine(data)
                }
                // AUTOGEN START: slotted-analysis-rust-merge
                // AUTOGEN END: slotted-analysis-rust-merge
                _ => (),
            };
        }
    });

    combined_data
}

impl Analysis<Mim> for MimAnalysis {
    type Data = AnalysisData;

    fn make(eg: &EGraph<Mim, Self>, enode: &Mim) -> Self::Data {
        combined_make(eg, enode)
    }

    fn merge(l: Self::Data, r: Self::Data) -> Self::Data {
        combined_merge(l, r)
    }

    fn modify(_eg: &mut EGraph<Mim, Self>, _id: Id) {}
}
