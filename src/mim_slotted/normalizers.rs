#![allow(dead_code)]
#![allow(unused_variables)]
// TODO: An analysis that adds equivalences that were "normalized away"
// This includes for instance:
// - The order of commutative operations like %core.nat.mul (a, b) being changed to %core.nat.mul (b, a)
// - A tuple of equivalent elements being reduced to a pack i.e. (3, 3, 3) => <3; 3>
use crate::mim_slotted::MimSlotted;
use crate::mim_slotted::analysis::{AnalysisData, MimSlottedAnalysis};
use slotted_egraphs::*;

pub type NormalizeData = ();

pub struct NormalizeAnalysis;
impl NormalizeAnalysis {
    pub fn make(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
        make_normalize(eg, enode)
    }
    pub fn merge(l: AnalysisData, r: AnalysisData) -> AnalysisData {
        merge_normalize(l, r)
    }
}

pub fn make_normalize(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    enode: &MimSlotted,
) -> AnalysisData {
    AnalysisData::default()
}

pub fn merge_normalize(l: AnalysisData, r: AnalysisData) -> AnalysisData {
    AnalysisData::default()
}
