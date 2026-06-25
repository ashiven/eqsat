#![allow(unused_imports)]
use crate::egg::Mim;
use egg::*;

// AUTOGEN START: egg-cost-rust-impl
// AUTOGEN END: egg-cost-rust-impl

#[derive(Debug)]
pub struct MyCost;
impl CostFunction<Mim> for MyCost {
    type Cost = usize;
    fn cost<C>(&mut self, enode: &Mim, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        enode.fold(1, |sum, id| sum.saturating_add(costs(id)))
    }
}
