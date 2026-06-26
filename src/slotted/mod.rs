use crate::ffi::bridge::{CostFn, OptionSelected, RecExprFFI, RuleSet};
use crate::slotted::cost::MaxAstSize;
use crate::slotted::rewrite::{filter_selected, rewrite_sexprs};
use crate::slotted::rules::convert_rules;
use crate::slotted::util::split_sexprs;
// AUTOGEN START: slotted-cost-rust-import
// AUTOGEN END: slotted-cost-rust-import
use crate::slotted::rulesets::get_rules;
use slotted_egraphs::*;
use stacker::grow;
use std::cell::RefCell;

pub mod analysis;
pub mod cost;
pub mod equiv;
pub mod rewrite;
pub mod rules;
pub mod rulesets;
pub mod types;
pub mod util;

#[cfg(test)]
mod test;

// Parsing rec exprs with type annotations can become very stack intensive
// so we preemptively increase the stack size to avoid stack overflows.
const PARSE_STACK_SIZE: usize = 8 * 1024 * 1024;

// We keep track of the selected rulesets in a global variable because they
// need to be accessed repeatedly in the analysis and it was too tedious to
// pass them on by parameters or otherwise.
thread_local! {
    pub static RULESETS: RefCell<Vec<RuleSet>> = const { RefCell::new(vec![]) };
}

define_language! {
    pub enum Mim {
        // TERMS

        // (let $name (scope <definition> <expr>))
        Let(Bind<AppliedId>) = "let",
        // (lam $var-name (scope <filter> <body>))
        Lam(Bind<AppliedId>) = "lam",
        // (con $var-name (scope <filter> <body>))
        Con(Bind<AppliedId>) = "con",
        // (fun $var-name (scope <filter> <body>))
        Fun(Bind<AppliedId>) = "fun",
        // (app <callee> <arg>)
        App(AppliedId, AppliedId) = "app",
        // (var $name)
        Var(Slot) = "var",
        // (lit <value> <type>)
        Lit(AppliedId, AppliedId) = "lit",
        // (pack $var (scope <arity> <body>))
        Pack(Bind<AppliedId>) = "pack",
        // (tuple <elem-cons>)
        Tuple(AppliedId) = "tuple",
        // (extract <tuple> <index>)
        Extract(AppliedId, AppliedId) = "extract",
        // (insert <tuple> <index> <value>)
        Insert(AppliedId, AppliedId, AppliedId) = "insert",
        // (rule <name> <meta-var-cons> <lhs> <rhs> <guard>)
        Rule(AppliedId, AppliedId, AppliedId, AppliedId, AppliedId) = "rule",
        // (inj <type> <value>)
        Inj(AppliedId, AppliedId) = "inj",
        // (merge <type> <type-cons>)
        Merge(AppliedId, AppliedId) = "merge",
        // (axm <name>)
        Axm(AppliedId) = "axm",
        // (match <op-cons>)
        Match(AppliedId) = "match",
        // (proxy <type> <pass> <tag> <op-cons>)
        Proxy(AppliedId, AppliedId, AppliedId, AppliedId) = "proxy",


        // TYPES

        // (join <type-cons>)
        Join(AppliedId) = "join",
        // (meet <type-cons>)
        Meet(AppliedId) = "meet",
        // (bot <type>)
        Bot(AppliedId) = "bot",
        // (top <type>)
        Top(AppliedId) = "top",
        // (arr $var (scope <arity> <body>))
        Arr(Bind<AppliedId>) = "arr",
        // (sigma $var (scope <type-cons> nil))
        Sigma(Bind<AppliedId>) = "sigma",
        // (pi* $var (scope <domain> <codomain>))
        ImplicitPi(Bind<AppliedId>) = "pi*",
        // (pi $var (scope <domain> <codomain>))
        Pi(Bind<AppliedId>) = "pi",
        // (cn $var (scope <domain> <codomain>))
        Cn(Bind<AppliedId>) = "cn",
        // (fn $var (scope <domain> <codomain>))
        Fn(Bind<AppliedId>) = "fn",
        // (idx <size>)
        Idx(AppliedId) = "idx",
        // (hole <type>)
        Hole(AppliedId) = "hole",
        // (type <level>)
        Type(AppliedId) = "type",
        // (reform <meta_type>)
        Reform(AppliedId) = "reform",


        // STRUCTURAL

        // We use this to annotate every term in the sexpr with a type as in (@ Bool (lit tt))
        // The sexprs we initially receive from the sexpr backend will be wrapped in types if we
        // provide the compiler flag --sexpr-include-types. However, we will not work with type-wrapped
        // sexprs during equality saturation as types are expected to be invariant per eclass.
        // We instead parse an initial typed RecExpr from which we extract all the type information
        // and then create an untyped RecExpr. The extracted type information will be added to
        // the egraph as analysis data that is merged upon eclass merges.
        // However, we have to reannotate the untyped RecExpr after equality saturation because
        // the rewrite phase requires type information for reconstruction.
        TypeWrap(AppliedId, AppliedId) = "@",

        // This is used to represent the meta variables introduced by rule declarations
        // without clashing with the 'var' nodes using slots.
        // (metavar <name>)
        MetaVar(AppliedId) = "metavar",

        // A root-level sexpr (in most cases this will be a closed/top-level continuation)
        // We introduce a node for this to avoid having to write (con extern main ...) to bind
        // named, top-level constructs and can instead write (root extern main (con ...)).
        // This allows us to omit names from lambda definitions entirely so we can get the full
        // benefits of slotted-egraphs while still having a binder for such constructs.
        // (root <extern> <name> <definition>)
        Root(AppliedId, AppliedId, AppliedId) = "root",

        // This is needed so we can bind a lambda variable to both its filter and body
        // and also bind a let variable to both its definition and its expression:
        // (scope <filter> <body>) or (scope <definition> <expression>)  i.e.: (let $foo (scope <def> <expr>))
        Scope(AppliedId, AppliedId) = "scope",

        // Enables variadic language constructs such as Tuple, Sigma, Match, ...
        // (cons <elem> <next>)
        Cons(AppliedId, AppliedId) = "cons",
        Nil() = "nil",

        // Leaf nodes
        Num(u64),
        Symbol(Symbol),
    }
}

pub fn equality_saturate(
    sexpr: &str,
    selected: OptionSelected,
    rulesets: Vec<RuleSet>,
    cost_fn: CostFn,
) -> Vec<RecExprFFI> {
    set_rulesets(rulesets);

    let mut sexprs = split_sexprs(sexpr);
    let mut rules = get_rules();

    convert_rules(&mut sexprs, &mut rules);

    // This gives us a bool-mask over our sexprs, marking sexprs that should be
    // rewritten with 'true' and those that shouldn't with 'false'.
    let selected = filter_selected(&sexprs, selected);

    match cost_fn {
        CostFn::AstSize => rewrite_sexprs(&sexprs, &selected, rules, || AstSize),
        CostFn::MaxAstSize => rewrite_sexprs(&sexprs, &selected, rules, || MaxAstSize),
        // AUTOGEN START: slotted-cost-rust-match
        // AUTOGEN END: slotted-cost-rust-match
        _ => panic!("Unknown cost function provided."),
    }
}

pub fn pretty(sexpr: &str, _line_len: usize) -> String {
    let sexprs = split_sexprs(sexpr);

    let mut res = String::new();
    for (i, sexpr) in sexprs.iter().enumerate() {
        let parsed: RecExpr<Mim> = grow(PARSE_STACK_SIZE, || RecExpr::parse(sexpr).unwrap());
        res.push_str(&parsed.to_string());
        if i < sexprs.len() - 1 {
            res.push_str("\n\n");
        } else {
            res.push('\n');
        }
    }

    res
}

fn set_rulesets(rulesets: Vec<RuleSet>) {
    RULESETS.with(|rulesets_global| {
        let mut rulesets_global = rulesets_global.borrow_mut();
        *rulesets_global = rulesets;
    });
}
