use crate::expect;
use crate::ffi::FFI;
use crate::ffi::bridge::{CostFn, OptionSelected, RecExprFFI, RuleSet};
// AUTOGEN START: egg-cost-rust-import
// AUTOGEN END: egg-cost-rust-import
use crate::mim_egg::analysis::MimAnalysis;
use crate::mim_egg::rulesets::get_rules;
use crate::mim_egg::types::{
    TypedRecExpr, add_expr_typed, extract_type_annotations, remove_type_annotations,
};
use egg::*;
use regex::Regex;
use std::cell::RefCell;

pub mod analysis;
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
        // (var <name> [<proj1> <proj2> ...] <type>)
        "var" = Var(Box<[Id]>),
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
        // (axm <name> <type>)
        "axm" = Axm([Id; 2]),
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

        Num(u64), Symbol(String),
    }
}

pub(crate) fn equality_saturate(
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

pub(crate) fn pretty(sexpr: &str, line_len: usize) -> String {
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

fn filter_selected(sexprs: &[String], selected: OptionSelected) -> Vec<bool> {
    let selected = unsafe { selected.option.as_mut() };
    let mut selected_mask: Vec<bool> = vec![true; sexprs.len()];

    let axm_regex = Regex::new(r"(?s)^\(@\s+.+\s+\(axm\s+([^)]+)\)\)$").unwrap();

    if let Some(names) = selected {
        for (i, sexpr) in sexprs.iter().enumerate() {
            let mut is_selected = false;

            if axm_regex.is_match(sexpr) {
                is_selected = true;
            }

            for name in names.iter() {
                if sexpr.starts_with(&format!("(root extern {}", name))
                    || sexpr.starts_with(&format!("(root intern {}", name))
                {
                    is_selected = true;
                    break;
                }
            }

            selected_mask[i] = is_selected;
        }
    }

    selected_mask
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
    sexprs: &[String],
    selected: &[bool],
    rules: Vec<Rewrite<Mim, MimAnalysis>>,
    cost_fn: F,
) -> Vec<RecExprFFI>
where
    C: CostFunction<Mim>,
    F: Fn() -> C,
{
    let mut rewritten_sexprs: Vec<RecExprFFI> = Vec::new();

    let mut roots: Vec<Id> = vec![];
    let mut eg = EGraph::<Mim, MimAnalysis>::default();
    for (i, is_selected) in selected.iter().enumerate() {
        if *is_selected {
            let sexpr = &sexprs[i];
            let annotated_rec_expr: RecExpr<Mim> = sexpr.parse().unwrap();

            let typed_rec_expr: TypedRecExpr = extract_type_annotations(&annotated_rec_expr);
            let root_id = add_expr_typed(&mut eg, typed_rec_expr);
            roots.push(root_id);
        }
    }

    let mut runner = Runner::<Mim, MimAnalysis>::default();
    runner = runner.with_egraph(eg);
    runner.roots = roots;
    let runner = runner.run(&rules);

    let extractor = Extractor::new(&runner.egraph, cost_fn());
    let mut root_idx = 0;
    for (i, is_selected) in selected.iter().enumerate() {
        if *is_selected {
            let (_best_cost, best_expr) = extractor.find_best(runner.roots[root_idx]);
            let best_expr_ffi = best_expr.to_ffi(Some(&runner.egraph));
            rewritten_sexprs.push(best_expr_ffi);
            root_idx += 1;
        } else {
            let sexpr = &sexprs[i];
            let annotated_rec_expr: RecExpr<Mim> = sexpr.parse().unwrap();

            // TODO: The below is also not very robust - maybe find some better way to do this

            // We first extract the types from the annotated rec expr to then be able
            // to extract the pi-type of the root-level lambda and manually set this
            // as the type of the unannotated_rec_expr_ffi. The reason we do this instead
            // of just adding the typed expr to the egraph and then hoping that rec_expr.to_ffi(eg)
            // will lookup and set the type of the rec expr for us, is that the lookup somehow fails.
            let typed_rec_expr: TypedRecExpr = extract_type_annotations(&annotated_rec_expr);
            let lam_type = {
                let lam = typed_rec_expr
                    .children
                    .get(2)
                    .expect("Expected root-level lambda");
                lam.type_.clone()
            };

            let unannotated_rec_expr = remove_type_annotations(&annotated_rec_expr);
            let mut unannotated_rec_expr_ffi = unannotated_rec_expr.to_ffi(Some(&runner.egraph));

            let lam_idx = unannotated_rec_expr_ffi.nodes.len() - 2;
            unannotated_rec_expr_ffi.nodes[lam_idx].type_ =
                lam_type.unwrap().to_ffi(Some(&runner.egraph));

            rewritten_sexprs.push(unannotated_rec_expr_ffi);
        }
    }

    rewritten_sexprs
}

fn convert_rules(sexprs: &mut Vec<String>, rules: &mut Vec<Rewrite<Mim, MimAnalysis>>) {
    // Converts rewrite rules in sexpr form into rewrite rules usable in egg and then
    // filters them out so we only have proper sexprs remaining to equality saturate in the next loop
    sexprs.retain(|sexpr| {
        if sexpr.trim().starts_with("(rule") {
            let rule: RecExpr<Mim> = sexpr.parse().unwrap();

            let [name, meta_var, lhs, rhs, _guard] = expect!(rule[rule.root()], Mim::Rule([name, meta_var, lhs, rhs, guard]) => [name,meta_var,lhs, rhs, guard] );


            let rule_name = if let Mim::Symbol(s) = &rule[name] {
                s
            } else {
                panic!("Failed to parse rule name.")
            };

            let mut meta_vars: Vec<String> = Vec::new();
            let nth_node = |id: Id| rule[id].clone();

            let meta_var_rexpr = rule[meta_var].build_recexpr(nth_node);
            for node in meta_var_rexpr.iter() {
                if let Mim::Var(ids) = node
                    && let [var_name, ..] = &**ids
                {
                    if let Mim::Symbol(s) =
                        &meta_var_rexpr[*var_name]
                    {
                        meta_vars.push(s.clone());
                    } else {
                        panic!("Failed to parse meta variable name.")
                    };
                }
            }

            let mut lhs_rexpr = rule[lhs].build_recexpr(nth_node);
            for (_id, node) in lhs_rexpr.items_mut() {
                if let Mim::Symbol(s) = node
                    && meta_vars.contains(s)
                {
                    s.insert(0, '?')
                }
            }

            let mut rhs_rexpr = rule[rhs].build_recexpr(nth_node);
            for (_id, node) in rhs_rexpr.items_mut() {
                if let Mim::Symbol(s) = node
                    && meta_vars.contains(s)
                {
                    s.insert(0, '?')
                }
            }

            let pat: Pattern<Mim> = lhs_rexpr.pretty(80).parse().unwrap();
            let outpat: Pattern<Mim> = rhs_rexpr.pretty(80).parse().unwrap();
            let rule: Rewrite<Mim, MimAnalysis> = Rewrite::new(rule_name, pat, outpat).unwrap();
            rules.push(rule);
            false
        } else {
            true
        }
    });
}
