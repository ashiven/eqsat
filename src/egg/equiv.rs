use crate::egg::rules::convert_rules;
use crate::egg::rulesets::get_rules;
use crate::egg::rulesets::set_rulesets;
use crate::egg::types::remove_type_annotations;
use crate::egg::util::split_sexprs;
use crate::ffi::bridge::RuleSet;
use egg::*;

pub fn reaches(
    sexpr: &str,
    rulesets: Vec<RuleSet>,
    start_name: &str,
    end_name: &str,
    max_steps: usize,
) -> bool {
    set_rulesets(rulesets);

    let mut sexprs = split_sexprs(sexpr);
    let mut rules = get_rules();

    convert_rules(&mut sexprs, &mut rules);

    let start_term = sexprs
        .iter()
        .find(|sexpr| {
            sexpr.starts_with(format!("(root extern {}\n", start_name).as_str())
                || sexpr.starts_with(format!("(root intern {}\n", start_name).as_str())
        })
        .expect("Reaches failed to find start term");

    let end_term = sexprs
        .iter()
        .find(|sexpr| {
            sexpr.starts_with(format!("(root extern {}\n", end_name).as_str())
                || sexpr.starts_with(format!("(root intern {}\n", end_name).as_str())
        })
        .expect("Reaches failed to find end term");

    // We want to assert only for the terms inside of the root nodes
    let start_term = start_term
        .strip_prefix(&format!("(root extern {}\n", start_name))
        .or(start_term.strip_prefix(&format!("(root intern {}\n", start_name)))
        .expect("Reaches failed to strip prefix")
        .strip_suffix(")")
        .expect("Reaches failed to strip suffix");

    let end_term = end_term
        .strip_prefix(&format!("(root extern {}\n", end_name))
        .or(end_term.strip_prefix(&format!("(root intern {}\n", end_name)))
        .expect("Reaches failed to strip prefix")
        .strip_suffix(")")
        .expect("Reaches failed to strip suffix");

    // We also don't care about type annotations, so we just remove them.
    let start_term_expr = start_term.parse().unwrap();
    let start_term_expr_unannotated = remove_type_annotations(&start_term_expr);
    let start_term = format!("{}", start_term_expr_unannotated);

    let end_term_expr = end_term.parse().unwrap();
    let end_term_expr_unannotated = remove_type_annotations(&end_term_expr);
    let end_term = format!("{}", end_term_expr_unannotated);

    assert_reaches(&start_term, &end_term, &rules, max_steps)
}

#[allow(clippy::type_complexity)]
fn reach_hook<'a, L, N, IterData>(
    start: &'a RecExpr<L>,
    goal: &'a RecExpr<L>,
    steps: usize,
) -> Box<dyn FnMut(&mut Runner<L, N, IterData>) -> Result<(), String>>
where
    L: Language + 'static,
    N: Analysis<L>,
    IterData: IterationData<L, N>,
{
    let start = start.clone();
    let goal = goal.clone();
    Box::new(move |runner: &mut Runner<L, N, IterData>| {
        if !runner.egraph.equivs(&start, &goal).is_empty() {
            return Err("Reached".to_owned());
        }
        if runner.iterations.len() >= steps - 1 {
            return Err("Failed".to_owned());
        }
        Ok(())
    })
}

pub fn assert_reaches<L, N>(
    start: &str,
    goal: &str,
    rewrites: &[Rewrite<L, N>],
    steps: usize,
) -> bool
where
    L: Language + FromOp + 'static,
    N: Analysis<L> + Default + 'static,
{
    let start: RecExpr<L> = start.parse().unwrap();
    let goal: RecExpr<L> = goal.parse().unwrap();

    let runner: Runner<L, N, ()> = Runner::default()
        .with_expr(&start)
        .with_iter_limit(steps)
        .with_hook(reach_hook(&start, &goal, steps));
    let runner = runner.run(rewrites);
    let report = runner.report();

    matches!(report.stop_reason, StopReason::Other(reason) if reason == "Reached")
}
