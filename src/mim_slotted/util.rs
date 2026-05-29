use crate::mim_slotted::MimSlotted;
use slotted_egraphs::*;

pub(crate) fn get_literal(lit_expr: &RecExpr<MimSlotted>) -> u64 {
    let lit_val = lit_expr.children.first().expect("Expected literal value");
    if let MimSlotted::Symbol(s) = lit_val.node {
        match s.as_str() {
            "ff" => 0,
            "tt" => 1,
            "i1" => 2,
            "i8" => 0x100,
            "i16" => 0x10000,
            "i32" => 0x100000000,
            _ => panic!("Unknown literal alias"),
        }
    } else if let MimSlotted::Num(n) = lit_val.node {
        n
    } else {
        panic!("Expected literal value to be a symbol or a number");
    }
}

pub(crate) fn cons_elem_at(cons_expr: &RecExpr<MimSlotted>, index: u64) -> RecExpr<MimSlotted> {
    let mut i = 0;
    let mut curr_cons = cons_expr;
    while let RecExpr {
        node: MimSlotted::Cons(..),
        children,
    } = curr_cons
    {
        let curr_elem = children.first().expect("Expected cons elem");
        if i == index {
            return curr_elem.clone();
        }
        curr_cons = children.get(1).expect("Expected next cons");
        i += 1;
    }
    panic!("Cons index out of bounds");
}

pub(crate) fn cons_insert_at(
    cons_expr: &RecExpr<MimSlotted>,
    value: &RecExpr<MimSlotted>,
    index: u64,
) -> RecExpr<MimSlotted> {
    let mut i = 0;
    let mut curr_cons = cons_expr.clone();
    let mut cursor = &mut curr_cons;

    while let RecExpr {
        node: MimSlotted::Cons(..),
        children,
    } = cursor
    {
        if i == index {
            children[0] = value.clone();
            return curr_cons;
        }
        cursor = &mut children[1];
        i += 1;
    }
    panic!("Cons index out of bounds");
}

#[macro_export]
macro_rules! typ {
    ($subst: expr, $eg: expr, $name: expr, $type: pat) => {{
        let id = $subst[$name].id;
        let analysis_data: &AnalysisData = $eg.analysis_data(id);
        if let Some(type_) = &analysis_data.type_ {
            matches!(type_.node, $type)
        } else {
            false
        }
    }};
}

#[macro_export]
macro_rules! isa {
    ($subst: expr, $eg: expr, $name: expr, $node: pat) => {{
        let id = &$subst[$name];
        let id = $eg.find_applied_id(id);
        let enodes = $eg.enodes_applied(&id);

        enodes.iter().any(|n| matches!(n, $node))
    }};
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

pub(crate) fn assert_reaches<L, N>(
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
