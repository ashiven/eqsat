use slotted_egraphs::*;

use crate::slotted::Mim;

pub struct MaxAstSize;

impl CostFunction<Mim> for MaxAstSize {
    type Cost = i64;

    fn cost<C>(&self, enode: &Mim, costs: C) -> Self::Cost
    where
        C: Fn(Id) -> Self::Cost,
    {
        let mut s: i64 = 1;
        for x in enode.applied_id_occurrences() {
            s = s.saturating_sub(costs(x.id));
        }
        s
    }
}

// AUTOGEN START: slotted-cost-rust-impl
// AUTOGEN END: slotted-cost-rust-impl
