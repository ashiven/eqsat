use crate::ffi::bridge::{MimKind, NodeFFI, RecExprFFI};
use crate::ffi::{FFI, FFIInner};
use crate::slotted::Mim;
use crate::slotted::analysis::MimAnalysis;
use slotted_egraphs::{EGraph, RecExpr};
use std::collections::HashMap;

impl FFI for RecExpr<Mim> {
    type EGraph = EGraph<Mim, MimAnalysis>;

    fn to_ffi(&self, egraph: Option<&Self::EGraph>) -> RecExprFFI {
        fn to_ffi_internal(
            rec_expr: &RecExpr<Mim>,
            nodes: &mut Vec<NodeFFI>,
            added: &mut HashMap<NodeFFI, usize>,
            egraph: Option<&EGraph<Mim, MimAnalysis>>,
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

impl FFIInner for Mim {
    type EGraph = EGraph<Mim, MimAnalysis>;

    fn to_ffi_with_childs(&self, children: &[usize], egraph: Option<&Self::EGraph>) -> NodeFFI {
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
            Mim::Let(bind) => new_node_ffi(
                MimKind::Let,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Lam(bind) => new_node_ffi(
                MimKind::Lam,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Con(bind) => new_node_ffi(
                MimKind::Con,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Fun(bind) => new_node_ffi(
                MimKind::Fun,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::App(..) => new_node_ffi(MimKind::App, children, None, None, None, type_),
            Mim::Var(slot) => new_node_ffi(
                MimKind::Var,
                children,
                None,
                None,
                Some(format!("{}", slot)),
                type_,
            ),
            Mim::Lit(..) => new_node_ffi(MimKind::Lit, children, None, None, None, type_),
            Mim::Pack(bind) => new_node_ffi(
                MimKind::Pack,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Tuple(..) => new_node_ffi(MimKind::Tuple, children, None, None, None, type_),
            Mim::Extract(..) => new_node_ffi(MimKind::Extract, children, None, None, None, type_),
            Mim::Insert(..) => new_node_ffi(MimKind::Insert, children, None, None, None, type_),
            Mim::Rule(..) => new_node_ffi(MimKind::Rule, children, None, None, None, type_),
            Mim::Inj(..) => new_node_ffi(MimKind::Inj, children, None, None, None, type_),
            Mim::Merge(..) => new_node_ffi(MimKind::Merge, children, None, None, None, type_),
            Mim::Axm(..) => new_node_ffi(MimKind::Axm, children, None, None, None, type_),
            Mim::Match(..) => new_node_ffi(MimKind::Match, children, None, None, None, type_),
            Mim::Proxy(..) => new_node_ffi(MimKind::Proxy, children, None, None, None, type_),
            Mim::Join(..) => new_node_ffi(MimKind::Join, children, None, None, None, type_),
            Mim::Meet(..) => new_node_ffi(MimKind::Meet, children, None, None, None, type_),
            Mim::Bot(..) => new_node_ffi(MimKind::Bot, children, None, None, None, type_),
            Mim::Top(..) => new_node_ffi(MimKind::Top, children, None, None, None, type_),
            Mim::Arr(bind) => new_node_ffi(
                MimKind::Arr,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Sigma(bind) => new_node_ffi(
                MimKind::Sigma,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::ImplicitPi(bind) => new_node_ffi(
                MimKind::ImplicitPi,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Pi(bind) => new_node_ffi(
                MimKind::Pi,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Cn(bind) => new_node_ffi(
                MimKind::Cn,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Fn(bind) => new_node_ffi(
                MimKind::Fn,
                children,
                None,
                None,
                Some(format!("{}", bind.slot)),
                type_,
            ),
            Mim::Idx(..) => new_node_ffi(MimKind::Idx, children, None, None, None, type_),
            Mim::Hole(..) => new_node_ffi(MimKind::Hole, children, None, None, None, type_),
            Mim::Type(..) => new_node_ffi(MimKind::Type, children, None, None, None, type_),
            Mim::Reform(..) => new_node_ffi(MimKind::Type, children, None, None, None, type_),
            Mim::TypeWrap(..) => new_node_ffi(MimKind::TypeWrap, children, None, None, None, type_),
            Mim::MetaVar(..) => new_node_ffi(MimKind::MetaVar, children, None, None, None, type_),
            Mim::Root(..) => new_node_ffi(MimKind::Root, children, None, None, None, type_),
            Mim::Scope(..) => new_node_ffi(MimKind::Scope, children, None, None, None, type_),
            Mim::Cons(..) => new_node_ffi(MimKind::Cons, children, None, None, None, type_),
            Mim::Nil() => new_node_ffi(MimKind::Nil, children, None, None, None, type_),
            Mim::Num(n) => new_node_ffi(MimKind::Num, children, Some(*n), None, None, type_),
            Mim::Symbol(s) => new_node_ffi(
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
