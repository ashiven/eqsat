use crate::ffi::bridge::RuleSet;
use crate::slotted::rules::convert_rules;
use crate::slotted::rulesets::{get_rules, set_rulesets};
use crate::slotted::types::remove_type_annotations;
use crate::slotted::util::split_sexprs;
use crate::slotted::{Mim, PARSE_STACK_SIZE};
use slotted_egraphs::*;
use stacker::grow;

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

    // TODO: More robust checks would be good because this string comparison
    // can break way too easily (just takes an extra space or tab somewhere)
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
    let start_term_expr: RecExpr<Mim> =
        grow(PARSE_STACK_SIZE, || RecExpr::parse(start_term).unwrap());
    let start_term_expr_unannotated = remove_type_annotations(&start_term_expr);
    let start_term = format!("{}", start_term_expr_unannotated);

    let end_term_expr: RecExpr<Mim> = grow(PARSE_STACK_SIZE, || RecExpr::parse(end_term).unwrap());
    let end_term_expr_unannotated = remove_type_annotations(&end_term_expr);
    let end_term = format!("{}", end_term_expr_unannotated);

    assert_reaches(&start_term, &end_term, &rules, max_steps)
}

// Source: https://github.com/memoryleak47/slotted-egraphs/blob/main/tests/entry.rs
// Had to copy-paste the code below since it didn't seem to be exposed as part of the library.

#[derive(Clone, Debug)]
enum ReachError {
    Reached,
    Failed,
}

#[allow(clippy::type_complexity)]
fn reach_hook<'a, L, N, IterData>(
    start: &'a RecExpr<L>,
    goal: &'a RecExpr<L>,
    steps: usize,
) -> Box<dyn FnMut(&mut Runner<L, N, IterData, ReachError>) -> Result<(), ReachError>>
where
    L: Language + 'static,
    N: Analysis<L>,
    IterData: IterationData<L, N>,
{
    let start = start.clone();
    let goal = goal.clone();
    Box::new(move |runner: &mut Runner<L, N, IterData, ReachError>| {
        if let Some(i2) = lookup_rec_expr(&goal, &runner.egraph) {
            let i1 = lookup_rec_expr(&start, &runner.egraph).unwrap();

            if runner.egraph.eq(&i1, &i2) {
                return Err(ReachError::Reached);
            }
        }
        if runner.iterations.len() >= steps - 1 {
            return Err(ReachError::Failed);
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
    L: Language + 'static,
    N: Analysis<L> + Default + 'static,
{
    let start: RecExpr<L> = RecExpr::parse(start).unwrap();
    let goal: RecExpr<L> = RecExpr::parse(goal).unwrap();

    let mut runner: Runner<L, N, (), ReachError> = Runner::default()
        .with_expr(&start)
        .with_iter_limit(60)
        .with_iter_limit(steps)
        .with_hook(reach_hook(&start, &goal, steps));
    let report = runner.run(rewrites);

    matches!(report.stop_reason, StopReason::Other(ReachError::Reached))
}
