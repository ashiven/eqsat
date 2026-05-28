use crate::mim_slotted::MimSlotted;
use crate::mim_slotted::types::{TypeData, make_type, merge_type};
use slotted_egraphs::*;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct MimSlottedAnalysis;

#[derive(Clone, Eq, PartialEq)]
pub struct AnalysisData {
    pub type_: Option<TypeData>,
}

// TODO: Merge results of different analyses
// and only compute for provided rulesets.

impl Analysis<MimSlotted> for MimSlottedAnalysis {
    type Data = AnalysisData;

    fn make(eg: &EGraph<MimSlotted, Self>, enode: &MimSlotted) -> Self::Data {
        make_type(eg, enode)
    }

    fn merge(l: Self::Data, r: Self::Data) -> Self::Data {
        merge_type(l, r)
    }

    fn modify(_eg: &mut EGraph<MimSlotted, Self>, _id: Id) {}
}
