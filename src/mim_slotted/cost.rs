use slotted_egraphs::*;

use crate::mim_slotted::MimSlotted;

// TODO: Implement
#[allow(dead_code)]
struct RiseCost;
impl CostFunction<MimSlotted> for RiseCost {
    type Cost = ();

    fn cost<C>(&self, _enode: &MimSlotted, _costs: C) -> Self::Cost
    where
        C: Fn(Id) -> Self::Cost,
    {
    }
}
