use crate::mim_egg::Mim;
use crate::mim_egg::analysis::MimAnalysis;
use crate::mim_slotted::MimSlotted;
use crate::mim_slotted::analysis::MimSlottedAnalysis;
use crate::{
    eqsat_egg, eqsat_slotted, node_ffi_str, pretty_egg, pretty_slotted, reaches_egg,
    reaches_slotted, type_str,
};
use bridge::{MimKind, NodeFFI, OptionSelected, RecExprFFI};
use egg::{EGraph, Id, RecExpr};
#[allow(unused_imports)]
use slotted_egraphs::{
    AstSize, EGraph as EGraphSlotted, Extractor, RecExpr as RecExprSlotted, lookup_rec_expr,
};
use std::collections::HashMap;
use std::fmt;

#[cxx::bridge]
pub mod bridge {
    #[derive(Debug)]
    enum RuleSet {
        // Egg
        Core,
        Math,
        // AUTOGEN START: egg-ruleset-rust-ffi
        // AUTOGEN END: egg-ruleset-rust-ffi

        // Slotted
        Standard,
        Rise,
        Normalize,
        // AUTOGEN START: slotted-ruleset-rust-ffi
        // AUTOGEN END: slotted-ruleset-rust-ffi
    }

    #[derive(Debug)]
    enum CostFn {
        // Egg/Slotted
        AstSize,

        // Egg
        AstDepth,
        // AUTOGEN START: egg-cost-rust-ffi
        // AUTOGEN END: egg-cost-rust-ffi

        // Slotted
        MaxAstSize,
        // AUTOGEN START: slotted-cost-rust-ffi
        // AUTOGEN END: slotted-cost-rust-ffi
    }

    #[derive(Debug, Hash, Default)]
    enum MimKind {
        Let,
        Lam,
        Con,
        Fun,
        App,
        Var,
        Lit,
        Pack,
        Tuple,
        Extract,
        Insert,
        Rule,
        Inj,
        Merge,
        Axm,
        Match,
        Proxy,
        Join,
        Meet,
        Bot,
        Top,
        Arr,
        Sigma,
        ImplicitPi,
        Pi,
        Cn,
        Fn,
        Idx,
        Hole,
        Type,
        Reform,
        TypeWrap,
        MetaVar,
        Root,
        Scope,
        Cons,
        #[default]
        Nil,
        Num,
        Symbol,
    }

    #[derive(Debug, Hash, Default, Eq, PartialEq)]
    struct NodeFFI {
        kind: MimKind,
        children: Vec<u32>,
        num: u64,
        symbol: String,
        slot: String,
        type_: RecExprFFI,
    }

    #[derive(Debug, Hash, Default, Eq, PartialEq)]
    struct RecExprFFI {
        nodes: Vec<NodeFFI>,
    }

    struct OptionSelected {
        option: *mut Vec<String>,
    }

    extern "Rust" {
        fn eqsat_egg(
            sexpr: &str,
            selected: OptionSelected,
            rulesets: Vec<RuleSet>,
            cost_fn: CostFn,
        ) -> Vec<RecExprFFI>;
        fn reaches_egg(
            sexpr: &str,
            rulesets: Vec<RuleSet>,
            start_name: &str,
            end_name: &str,
            max_steps: usize,
        ) -> bool;
        fn pretty_egg(sexpr: &str, line_len: usize) -> String;

        fn eqsat_slotted(
            sexpr: &str,
            selected: OptionSelected,
            rulesets: Vec<RuleSet>,
            cost_fn: CostFn,
        ) -> Vec<RecExprFFI>;
        fn reaches_slotted(
            sexpr: &str,
            rulesets: Vec<RuleSet>,
            start_name: &str,
            end_name: &str,
            max_steps: usize,
        ) -> bool;
        fn pretty_slotted(sexpr: &str, line_len: usize) -> String;

        fn pretty_ffi(sexpr: Vec<RecExprFFI>, line_len: usize) -> String;
        fn node_ffi_str(node: NodeFFI) -> String;
        fn type_str(type_: RecExprFFI, line_len: usize) -> String;
    }
}

impl OptionSelected {
    pub fn none() -> Self {
        OptionSelected {
            option: std::ptr::null_mut(),
        }
    }
}

pub(crate) fn pretty_ffi(sexprs: Vec<RecExprFFI>, line_len: usize) -> String {
    let mut res = String::new();

    for (i, sexpr) in sexprs.iter().enumerate() {
        res.push_str(&sexpr.pretty(line_len));
        if i < sexprs.len() - 1 {
            res.push_str("\n\n");
        } else {
            res.push('\n');
        }
    }

    res
}

impl fmt::Display for NodeFFI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            MimKind::Let => f.write_str("let"),
            MimKind::Lam => f.write_str("lam"),
            MimKind::Con => f.write_str("con"),
            MimKind::Fun => f.write_str("fun"),
            MimKind::App => f.write_str("app"),
            MimKind::Var => f.write_str("var"),
            MimKind::Lit => f.write_str("lit"),
            MimKind::Pack => f.write_str("pack"),
            MimKind::Tuple => f.write_str("tuple"),
            MimKind::Extract => f.write_str("extract"),
            MimKind::Insert => f.write_str("insert"),
            MimKind::Rule => f.write_str("rule"),
            MimKind::Inj => f.write_str("inj"),
            MimKind::Merge => f.write_str("merge"),
            MimKind::Axm => f.write_str("axm"),
            MimKind::Match => f.write_str("match"),
            MimKind::Proxy => f.write_str("proxy"),
            MimKind::Join => f.write_str("join"),
            MimKind::Meet => f.write_str("meet"),
            MimKind::Bot => f.write_str("bot"),
            MimKind::Top => f.write_str("top"),
            MimKind::Arr => f.write_str("arr"),
            MimKind::Sigma => f.write_str("sigma"),
            MimKind::ImplicitPi => f.write_str("pi*"),
            MimKind::Pi => f.write_str("pi"),
            MimKind::Cn => f.write_str("cn"),
            MimKind::Fn => f.write_str("fn"),
            MimKind::Idx => f.write_str("idx"),
            MimKind::Hole => f.write_str("hole"),
            MimKind::Type => f.write_str("type"),
            MimKind::Reform => f.write_str("reform"),
            MimKind::TypeWrap => f.write_str("@"),
            MimKind::MetaVar => f.write_str("metavar"),
            MimKind::Root => f.write_str("root"),
            MimKind::Scope => f.write_str("scope"),
            MimKind::Cons => f.write_str("cons"),
            MimKind::Nil => f.write_str("nil"),
            MimKind::Num => f.write_str(&self.num.to_string()),
            MimKind::Symbol => f.write_str(&self.symbol),
            _ => todo!(),
        }
    }
}

pub trait FFI {
    type EG;

    fn to_ffi(&self, egraph: Option<&Self::EG>) -> RecExprFFI;
}

pub trait FFIInner {
    type EG;

    fn to_ffi(&self, _type_: Option<RecExprFFI>) -> NodeFFI {
        Default::default()
    }
    fn to_ffi_with_childs(&self, _children: &[usize], _egraph: Option<&Self::EG>) -> NodeFFI {
        Default::default()
    }
}

impl FFI for RecExpr<Mim> {
    type EG = EGraph<Mim, MimAnalysis>;

    fn to_ffi(&self, egraph: Option<&Self::EG>) -> RecExprFFI {
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
    type EG = EGraph<Mim, MimAnalysis>;

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
            Mim::Var(children) => new_node_ffi(MimKind::Var, children, None, None, type_),
            Mim::Lit(children) => new_node_ffi(MimKind::Lit, children, None, None, type_),
            Mim::Pack(children) => new_node_ffi(MimKind::Pack, children, None, None, type_),
            Mim::Tuple(children) => new_node_ffi(MimKind::Tuple, children, None, None, type_),
            Mim::Extract(children) => new_node_ffi(MimKind::Extract, children, None, None, type_),
            Mim::Insert(children) => new_node_ffi(MimKind::Insert, children, None, None, type_),
            Mim::Rule(children) => new_node_ffi(MimKind::Rule, children, None, None, type_),
            Mim::Inj(children) => new_node_ffi(MimKind::Inj, children, None, None, type_),
            Mim::Merge(children) => new_node_ffi(MimKind::Merge, children, None, None, type_),
            Mim::Axm(children) => new_node_ffi(MimKind::Axm, children, None, None, type_),
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
            Mim::Num(n) => new_node_ffi(MimKind::Num, &[], Some(*n), None, type_),
            Mim::Symbol(s) => new_node_ffi(MimKind::Symbol, &[], None, Some(s.clone()), type_),
        }
    }
}

impl FFI for RecExprSlotted<MimSlotted> {
    type EG = EGraphSlotted<MimSlotted, MimSlottedAnalysis>;

    fn to_ffi(&self, egraph: Option<&Self::EG>) -> RecExprFFI {
        fn to_ffi_internal(
            rec_expr: &RecExprSlotted<MimSlotted>,
            nodes: &mut Vec<NodeFFI>,
            added: &mut HashMap<NodeFFI, usize>,
            egraph: Option<&EGraphSlotted<MimSlotted, MimSlottedAnalysis>>,
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
    type EG = EGraphSlotted<MimSlotted, MimSlottedAnalysis>;

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

/* ------------------------------------------------------------ */
/* ---- Pretty-printing implementation from the egg library --- */
/* ------------------------------------------------------------ */

// Source: https://github.com/egraphs-good/egg/blob/main/src/sexp.rs
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum Sexpr {
    String(String),
    List(Vec<Sexpr>),
    Empty,
}

impl fmt::Display for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sexpr::String(s) => {
                if s.contains(' ') || s.contains('(') || s.contains(')') || s.is_empty() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "{}", s)
                }
            }
            Sexpr::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
            Sexpr::Empty => write!(f, "()"),
        }
    }
}

// Source: https://github.com/egraphs-good/egg/blob/main/src/util.rs
fn pretty_print(buf: &mut String, sexpr: &Sexpr, width: usize, level: usize) -> std::fmt::Result {
    use std::fmt::Write;
    if let Sexpr::List(list) = sexpr {
        let indent = sexpr.to_string().len() > width;
        write!(buf, "(")?;

        for (i, val) in list.iter().enumerate() {
            if indent && i > 0 {
                writeln!(buf)?;
                for _ in 0..level {
                    write!(buf, "  ")?;
                }
            }
            pretty_print(buf, val, width, level + 1)?;
            if !indent && i < list.len() - 1 {
                write!(buf, " ")?;
            }
        }

        write!(buf, ")")?;
        Ok(())
    } else {
        write!(buf, "{}", sexpr.to_string().trim_matches('"'))
    }
}

// Source: https://github.com/egraphs-good/egg/blob/main/src/language.rs
impl RecExprFFI {
    fn to_sexpr(&self) -> Sexpr {
        let last = self.nodes.len() - 1;
        self.to_sexpr_rec(last, &mut |_| None)
    }

    fn to_sexpr_rec(&self, i: usize, f: &mut impl FnMut(u32) -> Option<String>) -> Sexpr {
        let node = &self.nodes[i];
        let op = Sexpr::String(node.to_string());
        if node.children.is_empty() && node.slot.is_empty() {
            op
        } else {
            let mut vec = vec![op];
            for child in node.children.iter() {
                vec.push(if let Some(s) = f(*child) {
                    return Sexpr::String(s);
                } else if (*child as usize) < i {
                    self.to_sexpr_rec(*child as usize, f)
                } else {
                    Sexpr::String(format!("<<<< CYCLE to {} = {:?} >>>>", i, node))
                })
            }
            // Some nodes introduce or use slots which don't
            // have their own nodes so we insert them manually.
            match node.kind {
                MimKind::Let => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Lam => {
                    if !node.slot.is_empty() {
                        vec.insert(vec.len() - 1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Con => {
                    if !node.slot.is_empty() {
                        vec.insert(vec.len() - 1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Fun => {
                    if !node.slot.is_empty() {
                        vec.insert(vec.len() - 1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Pack => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Var => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::ImplicitPi => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Pi => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Cn => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Fn => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Sigma => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Arr => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                _ => (),
            }
            Sexpr::List(vec)
        }
    }

    pub fn pretty(&self, width: usize) -> String {
        let sexp = self.to_sexpr();

        let mut buf = String::new();
        pretty_print(&mut buf, &sexp, width, 1).unwrap();
        buf
    }
}

/* ------------------------------------------------------------ */
/* ------------------------------------------------------------ */
/* ------------------------------------------------------------ */
