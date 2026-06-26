use crate::egg::rules::convert_rules;
use crate::egg::rulesets::{get_rules, set_rulesets};
use crate::egg::types::{
    TypedRecExpr, add_expr_typed, extract_type_annotations, remove_type_annotations,
};
use crate::egg::util::split_sexprs;
use crate::egg::{Mim, analysis::MimAnalysis};
use crate::ffi::FFI;
use crate::ffi::bridge::{CostFn, OptionSelected, RecExprFFI, RuleSet};
use egg::*;
use regex::Regex;

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

pub fn filter_selected(sexprs: &[String], selected: OptionSelected) -> Vec<bool> {
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

pub fn rewrite_sexprs<C, F>(
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

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn select_axiom() {
        let axm = "(@ (pi* _38960 (sigma dummy Nat Nat (type (lit 0 Univ))) (pi dummy (arr dummy (extract _38960 (lit 0 (idx (lit 3 Nat)))) (arr dummy (extract _38960 
        (lit 1 (idx (lit 3 Nat)))) (extract _38960 (lit 2 (idx (lit 3 Nat)))))) (arr dummy (extract _38960 (lit 1 (idx (lit 3 Nat)))) (arr dummy (extract _38960 (lit 0 (idx (lit 3 Nat)))) 
        (extract _38960 (lit 2 (idx (lit 3 Nat)))))))) (axm %rise.transpose))";

        let axm_regex = Regex::new(r"(?s)^\(@\s+.+\s+\(axm\s+([^)]+)\)\)$").unwrap();
        assert!(axm_regex.is_match(axm));
    }
}
