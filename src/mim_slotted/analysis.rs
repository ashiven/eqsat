use crate::ffi::bridge::RuleSet;
use crate::mim_slotted::types::{TypeAnalysis, TypeData};
// AUTOGEN START: slotted-analysis-rust-import
// AUTOGEN END: slotted-analysis-rust-import
use crate::mim_slotted::{MimSlotted, RULESETS};
use slotted_egraphs::*;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct MimSlottedAnalysis;

#[derive(Clone, Eq, PartialEq, Default)]
pub struct AnalysisData {
    pub type_: Option<TypeData>,
}

impl AnalysisData {
    fn combine(&mut self, other: AnalysisData) {
        self.type_ = self.type_.take().or(other.type_);
    }
}

fn combined_make(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
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

impl Analysis<MimSlotted> for MimSlottedAnalysis {
    type Data = AnalysisData;

    fn make(eg: &EGraph<MimSlotted, Self>, enode: &MimSlotted) -> Self::Data {
        combined_make(eg, enode)
    }

    fn merge(l: Self::Data, r: Self::Data) -> Self::Data {
        combined_merge(l, r)
    }

    fn modify(_eg: &mut EGraph<MimSlotted, Self>, _id: Id) {}
}
