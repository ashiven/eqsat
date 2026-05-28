use crate::ffi::bridge::RuleSet;
use crate::mim_slotted::types::{TypeAnalysis, TypeData};
use crate::mim_slotted::{MimSlotted, RULESETS};
use slotted_egraphs::*;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct MimSlottedAnalysis;

#[derive(Clone, Eq, PartialEq, Default)]
pub struct AnalysisData {
    pub type_: Option<TypeData>,
}

impl AnalysisData {
    fn combine(self, other: AnalysisData) -> Self {
        Self {
            type_: self.type_.or(other.type_),
        }
    }
}

fn combined_make(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let mut combined_data = AnalysisData::default();

    // Analyses applied for all rulesets
    let type_data = TypeAnalysis::make(eg, enode);
    combined_data = combined_data.combine(type_data);

    // Ruleset-specific analyses
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            match *ruleset {
                RuleSet::Rise => {
                    // let type_data = TypeAnalysis::make(eg, enode);
                    // combined_data = combined_data.combine(type_data)
                }
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
    combined_data = combined_data.combine(type_data);

    // Ruleset-specific analyses
    RULESETS.with(|rulesets_global| {
        for ruleset in rulesets_global.borrow().iter() {
            #[allow(clippy::single_match)]
            match *ruleset {
                RuleSet::Rise => {
                    // let type_data = TypeAnalysis::merge(l.clone(), r.clone());
                    // combined_data = combined_data.combine(type_data)
                }
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
