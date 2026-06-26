use crate::ffi::bridge::{CostFn, OptionSelected, RecExprFFI, RuleSet};
// AUTOGEN START: egg-cost-rust-import
// AUTOGEN END: egg-cost-rust-import
use crate::egg::rewrite::{filter_selected, rewrite_sexprs};
use crate::egg::rules::convert_rules;
use crate::egg::rulesets::get_rules;
use crate::egg::util::split_sexprs;
use egg::*;
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

thread_local! {
    pub static RULESETS: RefCell<Vec<RuleSet>> = const { RefCell::new(vec![]) };
}

define_language! {
    pub enum Mim {
        // TERMS

        // (let <var> <definition> <expression>)
        "let" = Let([Id; 3]),
        // (lam <var> [<filter> <body>])
        "lam" = Lam(Box<[Id]>),
        // (con <var> [<filter> <body>])
        "con" = Con(Box<[Id]>),
        // (fun <var> [<filter> <body>])
        "fun" = Fun(Box<[Id]>),
        // (app <callee> <arg>)
        "app" = App([Id; 2]),
        // (var <name>)
        "var" = Var(Id),
        // (lit <value> <type>)
        "lit" = Lit([Id; 2]),
        // (pack <var> <arity> <body>)
        "pack" = Pack([Id; 3]),
        // (tuple <elems>...)
        "tuple" = Tuple(Box<[Id]>),
        // (extract <tuple> <index>)
        "extract" = Extract([Id; 2]),
        // (ins <tuple> <index> <value>)
        "insert" = Insert([Id; 3]),
        // (rule <name> <meta-var> <lhs> <rhs> <guard>)
        "rule" = Rule([Id; 5]),
        // (inj <type> <value>)
        "inj" = Inj([Id; 2]),
        // (merge <type> <values>...)
        "merge" = Merge(Box<[Id]>),
        // (axm <name>)
        "axm" = Axm(Id),
        // (match <scrutinee> <arms>...)
        "match" = Match(Box<[Id]>),
        // (proxy <type> <pass> <tag> <ops>...)
        "proxy" = Proxy(Box<[Id]>),


        // TYPES

        // (join <types>...)
        "join" = Join(Box<[Id]>),
        // (meet <types>...)
        "meet" = Meet(Box<[Id]>),
        // (bot <type>)
        "bot" = Bot(Id),
        // (top <type>)
        "top" = Top(Id),
        // (arr <var> <arity> <body>)
        "arr" = Arr([Id; 3]),
        // (sigma <var> <types>...)
        "sigma" = Sigma(Box<[Id]>),
        // (fn <var> <domain> <codomain>)
        "fn" = Fn_([Id; 3]),
        // (cn <var> <domain> <codomain>)
        "cn" = Cn([Id; 3]),
        // (pi <var> <domain> <codomain>)
        "pi" = Pi([Id; 3]),
        // (pi* <var> <domain> <codomain>)
        "pi*" = ImplicitPi([Id; 3]),
        // (idx <size>)
        "idx" = Idx(Id),
        // (hole <type>)
        "hole" = Hole(Id),
        // (type <level>)
        "type" = Type(Id),
        // (reform <meta_type>)
        "reform" = Reform(Id),

        // (@ <type> <value>)
        "@" = TypeWrap([Id; 2]),
        // (root <extern> <name> <definition>)
        "root" = Root([Id; 3]),
        // (metavar <name> [<projs>...])
        "metavar" = MetaVar(Box<[Id]>),

        Num(u64), Symbol(String),
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
        CostFn::AstDepth => rewrite_sexprs(&sexprs, &selected, rules, || AstDepth),
        // AUTOGEN START: egg-cost-rust-match
        // AUTOGEN END: egg-cost-rust-match
        _ => panic!("Unknown cost function provided."),
    }
}

pub fn set_rulesets(rulesets: Vec<RuleSet>) {
    RULESETS.with(|rulesets_global| {
        let mut rulesets_global = rulesets_global.borrow_mut();
        *rulesets_global = rulesets;
    });
}

pub fn pretty(sexpr: &str, line_len: usize) -> String {
    let normalized = sexpr.replace("\r\n", "\n");
    let mut sexprs: Vec<&str> = normalized.split("\n\n").collect();
    sexprs.retain(|s| !s.trim().is_empty());
    let mut res = String::new();

    for (i, sexpr) in sexprs.iter().enumerate() {
        let parsed: RecExpr<Mim> = sexpr.parse().unwrap();
        res.push_str(parsed.pretty(line_len).as_str());
        if i < sexprs.len() - 1 {
            res.push_str("\n\n");
        } else {
            res.push('\n');
        }
    }

    res
}
