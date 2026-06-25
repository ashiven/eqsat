use crate::egg::Mim;
use egg::*;

pub(crate) fn get_literal(lit_expr: &RecExpr<Mim>) -> u64 {
    let node = lit_expr.iter().last().unwrap();
    let lit_val_id = usize::from(*node.children().first().unwrap());
    let lit_val = lit_expr.get(lit_val_id).unwrap();

    if let Mim::Symbol(s) = lit_val {
        match s.as_str() {
            "ff" => 0,
            "tt" => 1,
            "i1" => 2,
            "i8" => 0x100,
            "i16" => 0x10000,
            "i32" => 0x100000000,
            _ => panic!("Unknown literal alias"),
        }
    } else if let Mim::Num(n) = lit_val {
        *n
    } else {
        panic!("Expected literal value to be a symbol or a number");
    }
}

// Source: https://github.com/memoryleak47/slotted-egraphs/blob/main/tests/entry.rs
// Had to copy-paste the code below since it didn't seem to be exposed as part of the library.

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

pub(crate) fn assert_reaches<L, N>(
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
