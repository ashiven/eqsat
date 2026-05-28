use crate::ffi::FFI;
use crate::ffi::bridge::{CostFn, RecExprFFI, RuleSet};
use crate::mim_slotted::analysis::MimSlottedAnalysis;
use crate::mim_slotted::rulesets::get_rules;
use crate::mim_slotted::types::{TypedRecExpr, add_expr_typed, extract_type_annotations};
use regex::Regex;
use slotted_egraphs::*;
use stacker::grow;
use std::cell::RefCell;

pub mod analysis;
pub mod cost;
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
    pub enum MimSlotted {
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
        // (hole <type>) - does it even make sense to have this?
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
        // (metavar <name> <type>)
        MetaVar(AppliedId, AppliedId) = "metavar",

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

pub(crate) fn equality_saturate(
    sexpr: &str,
    rulesets: Vec<RuleSet>,
    cost_fn: CostFn,
) -> Vec<RecExprFFI> {
    let mut sexprs = split_sexprs(sexpr);

    set_rulesets(rulesets);

    let mut rules = get_rules();
    convert_rules(&mut sexprs, &mut rules);

    match cost_fn {
        CostFn::AstSize => rewrite_sexprs(sexprs, rules, || AstSize),
        _ => panic!("Unknown cost function provided."),
    }
}

pub(crate) fn pretty(sexpr: &str, _line_len: usize) -> String {
    let sexprs = split_sexprs(sexpr);

    let mut res = String::new();
    for (i, sexpr) in sexprs.iter().enumerate() {
        let parsed: RecExpr<MimSlotted> = grow(PARSE_STACK_SIZE, || RecExpr::parse(sexpr).unwrap());
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

fn split_sexprs(sexpr: &str) -> Vec<String> {
    let normalized = sexpr.replace("\r\n", "\n");

    normalized
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn rewrite_sexprs<C, F>(
    sexprs: Vec<String>,
    rules: Vec<Rewrite<MimSlotted, MimSlottedAnalysis>>,
    cost_fn: F,
) -> Vec<RecExprFFI>
where
    C: CostFunction<MimSlotted>,
    F: Fn() -> C,
{
    let mut rewritten_sexprs: Vec<RecExprFFI> = Vec::new();

    let mut roots: Vec<AppliedId> = vec![];
    let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();
    for sexpr in &sexprs {
        let annotated_rec_expr: RecExpr<MimSlotted> =
            grow(PARSE_STACK_SIZE, || RecExpr::parse(sexpr).unwrap());

        let typed_rec_expr: TypedRecExpr = extract_type_annotations(&annotated_rec_expr);
        let root_id = add_expr_typed(&mut eg, typed_rec_expr);
        roots.push(root_id);
    }

    let mut runner = Runner::<MimSlotted, MimSlottedAnalysis>::default();
    runner = runner.with_egraph(eg);
    runner.roots = roots;
    let _report = runner.run(&rules);

    let extractor = Extractor::new(&runner.egraph, cost_fn());
    for i in 0..sexprs.len() {
        let best_expr = extractor.extract(&runner.roots[i], &runner.egraph);
        let best_expr_ffi = best_expr.to_ffi(&runner.egraph);
        rewritten_sexprs.push(best_expr_ffi);
    }

    rewritten_sexprs
}

fn convert_rules(
    sexprs: &mut Vec<String>,
    rules: &mut Vec<Rewrite<MimSlotted, MimSlottedAnalysis>>,
) {
    sexprs.retain(|sexpr| {
        // let parsed: RecExpr<MimSlotted> = RecExpr::parse(sexpr).unwrap();
        // if let MimSlotted::Rule(..) = parsed.node {
        //
        // We initially used the more robust check above, however I realized
        // that the operation of parsing a rec expr with type annotations can
        // be very expensive and so it would be better to use the cheap shortcut
        // below to check for rule sexprs
        //
        // (rule <name> <meta_var> <lhs> <rhs> <guard>)
        if sexpr.trim().starts_with("(rule") {
            let parsed: RecExpr<MimSlotted> = RecExpr::parse(sexpr).unwrap();

            let mut rule_name = "";
            if let MimSlotted::Symbol(s) = parsed.children[0].node {
                rule_name = s.into();
            }

            let mut meta_vars: Vec<String> = Vec::new();
            fn lookup(rec_expr: &RecExpr<MimSlotted>, meta_vars: &mut Vec<String>) {
                if let RecExpr {
                    node: MimSlotted::MetaVar(..),
                    children,
                } = rec_expr
                {
                    let name_expr = children.first().expect("Expected meta var name");
                    if let MimSlotted::Symbol(s) = name_expr.node {
                        meta_vars.push(s.to_string());
                    } else {
                        panic!("Expected meta var name to be a symbol");
                    }
                }
                rec_expr.children.iter().for_each(|c| lookup(c, meta_vars));
            }
            lookup(&parsed, &mut meta_vars);

            let lhs_rexpr = &parsed.children[2];
            let rhs_rexpr = &parsed.children[3];

            let mut pat = format!("{}", re_to_pattern(lhs_rexpr));
            inject_meta_vars(&meta_vars, &mut pat);
            let mut outpat = format!("{}", re_to_pattern(rhs_rexpr));
            inject_meta_vars(&meta_vars, &mut outpat);

            let rule: Rewrite<MimSlotted, MimSlottedAnalysis> =
                Rewrite::new(rule_name, &pat, &outpat);
            rules.push(rule);

            false
        } else {
            true
        }
    });
}

fn inject_meta_vars(meta_vars: &[String], pattern: &mut String) {
    // We differentiate between meta vars with prefix "pat_" and meta vars with prefix "slot_".
    // As the names suggest, the first kind are pattern vars and the second are slots

    let re = Regex::new(r"(pat|slot)_([_A-Za-z0-9]+)").unwrap();

    let res = re.replace_all(pattern, |caps: &regex::Captures| {
        let kind = &caps[1];
        let name = &caps[2];

        let full_name = format!("{}_{}", kind, name);
        if !meta_vars.contains(&full_name) {
            return full_name;
        }

        match kind {
            "pat" => format!("?{}", name),
            "slot" => format!("(var ${})", name),
            _ => unreachable!(),
        }
    });

    *pattern = res.into_owned();
}
