use crate::ffi::bridge::bridge::{MimKind, NodeFFI, RecExprFFI};
use crate::ffi::{FFI, FFIInner};
use crate::slotted::MimSlotted;
use crate::slotted::analysis::MimSlottedAnalysis;
use slotted_egraphs::{EGraph, RecExpr};
use std::collections::HashMap;

impl FFI for RecExpr<MimSlotted> {
    type EG = EGraph<MimSlotted, MimSlottedAnalysis>;

    fn to_ffi(&self, egraph: Option<&Self::EG>) -> RecExprFFI {
        fn to_ffi_internal(
            rec_expr: &RecExpr<MimSlotted>,
            nodes: &mut Vec<NodeFFI>,
            added: &mut HashMap<NodeFFI, usize>,
            egraph: Option<&EGraph<MimSlotted, MimSlottedAnalysis>>,
        ) -> usize {
            let child_ids: Vec<usize> = rec_expr
                .children
                .iter()
                .map(|child| to_ffi_internal(child, nodes, added, egraph))
                .collect();

            let new_node = rec_expr.node.to_ffi_with_childs(&child_ids, egraph);

            if added.contains_key(&new_node) {
                return *added.get(&new_node).unwrap();
            }

            let id = nodes.len();
            nodes.push(new_node);
            id
        }

        let mut nodes: Vec<NodeFFI> = Vec::new();
        let mut added = HashMap::<NodeFFI, usize>::new();
        to_ffi_internal(self, &mut nodes, &mut added, egraph);
        RecExprFFI { nodes }
    }
}

impl FFIInner for MimSlotted {
    type EG = EGraph<MimSlotted, MimSlottedAnalysis>;

    fn to_ffi_with_childs(&self, children: &[usize], egraph: Option<&Self::EG>) -> NodeFFI {
        fn new_node_ffi(
            kind: MimKind,
            children: &[usize],
            num: Option<u64>,
            symbol: Option<String>,
            slot: Option<String>,
            type_: Option<RecExprFFI>,
        ) -> NodeFFI {
            let converted_ids = children.iter().map(|id| *id as u32).collect();

            NodeFFI {
                kind,
                children: converted_ids,
                num: num.unwrap_or_default(),
                symbol: symbol.unwrap_or_default(),
                slot: slot.unwrap_or_default(),
                type_: type_.unwrap_or(RecExprFFI { nodes: vec![] }),
            }
        }

        let mut type_ = None;

        if let Some(egraph) = egraph {
            let eclass_id = egraph.lookup(self);
            type_ = if let Some(eclass_id) = eclass_id {
                let type_ = egraph.analysis_data(eclass_id.id).type_.clone();

                // TODO:
                // - The code below was meant to fix the issue of types that depend on slots
                //   of other terms in the e-graph (see types::type_depending_on_outer_slot)
                // - It was meant to work by having the type of the analysis data be represented
                //   in the e-graph as well to ensure that the external slots it contains get
                //   updated when the external terms that bind these slots are
                // - This doesn't happen, however, which I believe is because the types that
                //   we add to the e-graph (In the test case above, a type containing $bar
                //   which was bound by the surrounding let) aren't subterms of the terms
                //   introducing the external slots
                // - In our example, the extracted type below will still only contain $bar
                //   instead of the updated slot of the let, which would be something like $f13
                //
                // if let Some(t) = &type_ {
                //     let t_id = lookup_rec_expr(t, egraph);
                //     if let Some(id) = t_id {
                //         let extractor = Extractor::new(egraph, AstSize);
                //         type_ = Some(extractor.extract(&id, egraph));
                //     };
                // }

                type_.map(|type_| type_.to_ffi(None))
            } else {
                None
            };
        }

        match &self {
            MimSlotted::Let(bind) => new_node_ffi(
                MimKind::Let,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Lam(bind) => new_node_ffi(
                MimKind::Lam,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Con(bind) => new_node_ffi(
                MimKind::Con,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Fun(bind) => new_node_ffi(
                MimKind::Fun,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::App(..) => new_node_ffi(MimKind::App, children, None, None, None, type_),
            MimSlotted::Var(slot) => new_node_ffi(
                MimKind::Var,
                children,
                None,
                None,
                Some(format!("{}", slot)),
                type_,
            ),
            MimSlotted::Lit(..) => new_node_ffi(MimKind::Lit, children, None, None, None, type_),
            MimSlotted::Pack(bind) => new_node_ffi(
                MimKind::Pack,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Tuple(..) => {
                new_node_ffi(MimKind::Tuple, children, None, None, None, type_)
            }
            MimSlotted::Extract(..) => {
                new_node_ffi(MimKind::Extract, children, None, None, None, type_)
            }
            MimSlotted::Insert(..) => {
                new_node_ffi(MimKind::Insert, children, None, None, None, type_)
            }
            MimSlotted::Rule(..) => new_node_ffi(MimKind::Rule, children, None, None, None, type_),
            MimSlotted::Inj(..) => new_node_ffi(MimKind::Inj, children, None, None, None, type_),
            MimSlotted::Merge(..) => {
                new_node_ffi(MimKind::Merge, children, None, None, None, type_)
            }
            MimSlotted::Axm(..) => new_node_ffi(MimKind::Axm, children, None, None, None, type_),
            MimSlotted::Match(..) => {
                new_node_ffi(MimKind::Match, children, None, None, None, type_)
            }
            MimSlotted::Proxy(..) => {
                new_node_ffi(MimKind::Proxy, children, None, None, None, type_)
            }
            MimSlotted::Join(..) => new_node_ffi(MimKind::Join, children, None, None, None, type_),
            MimSlotted::Meet(..) => new_node_ffi(MimKind::Meet, children, None, None, None, type_),
            MimSlotted::Bot(..) => new_node_ffi(MimKind::Bot, children, None, None, None, type_),
            MimSlotted::Top(..) => new_node_ffi(MimKind::Top, children, None, None, None, type_),
            MimSlotted::Arr(bind) => new_node_ffi(
                MimKind::Arr,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Sigma(bind) => new_node_ffi(
                MimKind::Sigma,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::ImplicitPi(bind) => new_node_ffi(
                MimKind::ImplicitPi,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Pi(bind) => new_node_ffi(
                MimKind::Pi,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Cn(bind) => new_node_ffi(
                MimKind::Cn,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Fn(bind) => new_node_ffi(
                MimKind::Fn,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            MimSlotted::Idx(..) => new_node_ffi(MimKind::Idx, children, None, None, None, type_),
            MimSlotted::Hole(..) => new_node_ffi(MimKind::Hole, children, None, None, None, type_),
            MimSlotted::Type(..) => new_node_ffi(MimKind::Type, children, None, None, None, type_),
            MimSlotted::Reform(..) => {
                new_node_ffi(MimKind::Type, children, None, None, None, type_)
            }
            MimSlotted::TypeWrap(..) => {
                new_node_ffi(MimKind::TypeWrap, children, None, None, None, type_)
            }
            MimSlotted::MetaVar(..) => {
                new_node_ffi(MimKind::MetaVar, children, None, None, None, type_)
            }
            MimSlotted::Root(..) => new_node_ffi(MimKind::Root, children, None, None, None, type_),
            MimSlotted::Scope(..) => {
                new_node_ffi(MimKind::Scope, children, None, None, None, type_)
            }
            MimSlotted::Cons(..) => new_node_ffi(MimKind::Cons, children, None, None, None, type_),
            MimSlotted::Nil() => new_node_ffi(MimKind::Nil, children, None, None, None, type_),
            MimSlotted::Num(n) => new_node_ffi(MimKind::Num, children, Some(*n), None, None, type_),
            MimSlotted::Symbol(s) => new_node_ffi(
                MimKind::Symbol,
                children,
                None,
                Some(s.to_string()),
                None,
                type_,
            ),
        }
    }
}
