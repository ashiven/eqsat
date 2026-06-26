use crate::egg::Mim;
use crate::egg::analysis::MimAnalysis;
use crate::ffi::bridge::{MimKind, NodeFFI, RecExprFFI};
use crate::ffi::{FFI, FFIInner};
use egg::{EGraph, Id, RecExpr};

impl FFI for RecExpr<Mim> {
    type EGraph = EGraph<Mim, MimAnalysis>;

    fn to_ffi(&self, egraph: Option<&Self::EGraph>) -> RecExprFFI {
        let nodes = if let Some(egraph) = egraph
            && let Some(ids) = egraph.lookup_expr_ids(self)
        {
            self.iter()
                .zip(ids)
                .map(|(n, id)| {
                    n.to_ffi(
                        egraph[id]
                            .data
                            .type_
                            .clone()
                            .map(|t| t.to_ffi(Some(egraph))),
                    )
                })
                .collect()
        } else {
            self.iter().map(|n| n.to_ffi(None)).collect()
        };
        RecExprFFI { nodes }
    }
}

impl FFIInner for Mim {
    type EGraph = EGraph<Mim, MimAnalysis>;

    fn to_ffi(&self, type_: Option<RecExprFFI>) -> NodeFFI {
        fn new_node_ffi(
            kind: MimKind,
            children: &[Id],
            num: Option<u64>,
            symbol: Option<String>,
            type_: Option<RecExprFFI>,
        ) -> NodeFFI {
            let converted_ids = children.iter().map(|id| usize::from(*id) as u32).collect();

            NodeFFI {
                kind,
                children: converted_ids,
                num: num.unwrap_or_default(),
                symbol: symbol.unwrap_or_default(),
                slot: String::new(),
                type_: type_.unwrap_or(RecExprFFI { nodes: vec![] }),
            }
        }

        match self {
            Mim::Let(children) => new_node_ffi(MimKind::Let, children, None, None, type_),
            Mim::Lam(children) => new_node_ffi(MimKind::Lam, children, None, None, type_),
            Mim::Con(children) => new_node_ffi(MimKind::Con, children, None, None, type_),
            Mim::Fun(children) => new_node_ffi(MimKind::Fun, children, None, None, type_),
            Mim::App(children) => new_node_ffi(MimKind::App, children, None, None, type_),
            Mim::Var(child) => new_node_ffi(MimKind::Var, &[*child], None, None, type_),
            Mim::Lit(children) => new_node_ffi(MimKind::Lit, children, None, None, type_),
            Mim::Pack(children) => new_node_ffi(MimKind::Pack, children, None, None, type_),
            Mim::Tuple(children) => new_node_ffi(MimKind::Tuple, children, None, None, type_),
            Mim::Extract(children) => new_node_ffi(MimKind::Extract, children, None, None, type_),
            Mim::Insert(children) => new_node_ffi(MimKind::Insert, children, None, None, type_),
            Mim::Rule(children) => new_node_ffi(MimKind::Rule, children, None, None, type_),
            Mim::Inj(children) => new_node_ffi(MimKind::Inj, children, None, None, type_),
            Mim::Merge(children) => new_node_ffi(MimKind::Merge, children, None, None, type_),
            Mim::Axm(child) => new_node_ffi(MimKind::Axm, &[*child], None, None, type_),
            Mim::Match(children) => new_node_ffi(MimKind::Match, children, None, None, type_),
            Mim::Proxy(children) => new_node_ffi(MimKind::Proxy, children, None, None, type_),
            Mim::Join(children) => new_node_ffi(MimKind::Join, children, None, None, type_),
            Mim::Meet(children) => new_node_ffi(MimKind::Meet, children, None, None, type_),
            Mim::Bot(child) => new_node_ffi(MimKind::Bot, &[*child], None, None, type_),
            Mim::Top(child) => new_node_ffi(MimKind::Top, &[*child], None, None, type_),
            Mim::Arr(children) => new_node_ffi(MimKind::Arr, children, None, None, type_),
            Mim::Sigma(children) => new_node_ffi(MimKind::Sigma, children, None, None, type_),
            Mim::Fn_(children) => new_node_ffi(MimKind::Fn, children, None, None, type_),
            Mim::Cn(children) => new_node_ffi(MimKind::Cn, children, None, None, type_),
            Mim::Pi(children) => new_node_ffi(MimKind::Pi, children, None, None, type_),
            Mim::ImplicitPi(children) => {
                new_node_ffi(MimKind::ImplicitPi, children, None, None, type_)
            }
            Mim::Idx(child) => new_node_ffi(MimKind::Idx, &[*child], None, None, type_),
            Mim::Hole(child) => new_node_ffi(MimKind::Hole, &[*child], None, None, type_),
            Mim::Type(child) => new_node_ffi(MimKind::Type, &[*child], None, None, type_),
            Mim::Reform(child) => new_node_ffi(MimKind::Type, &[*child], None, None, type_),
            Mim::TypeWrap(children) => new_node_ffi(MimKind::TypeWrap, children, None, None, type_),
            Mim::Root(children) => new_node_ffi(MimKind::Root, children, None, None, type_),
            Mim::MetaVar(children) => new_node_ffi(MimKind::MetaVar, children, None, None, type_),
            Mim::Num(n) => new_node_ffi(MimKind::Num, &[], Some(*n), None, type_),
            Mim::Symbol(s) => new_node_ffi(MimKind::Symbol, &[], None, Some(s.clone()), type_),
        }
    }
}
