use crate::ffi::bridge::{CostFn, OptionSelected, RecExprFFI, RuleSet};
use crate::mim_egg::equality_saturate;

const LINE_LEN: usize = 80;

fn first(rewrites: Vec<RecExprFFI>) -> String {
    rewrites
        .first()
        .expect("rewrites was empty")
        .pretty(LINE_LEN)
}

#[test]
fn fold_core_nat_simple() {
    let sexpr = "(app %core.nat.add (tuple (lit 1 Nat) (lit 1 Nat)))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit 2 Nat)");
}

#[test]
fn fold_core_nat_complex() {
    let sexpr = "(app %core.nat.mul (tuple (app %core.nat.add (tuple (lit 1 Nat) (lit 1 Nat))) (app %core.nat.sub (tuple (lit 5 Nat) (lit 4 Nat)))))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit 2 Nat)");
}

#[test]
fn fold_core_ncmp_simple() {
    let sexpr = "(app %core.ncmp.e (tuple (lit 4 Nat) (lit 2 Nat)))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit ff Bool)");
}

#[test]
fn fold_core_ncmp_complex() {
    let sexpr = "(app %core.ncmp.ne (tuple (app %core.nat.add (tuple (lit 1 Nat) (lit 1 Nat))) (app %core.nat.sub (tuple (lit 5 Nat) (lit 4 Nat)))))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit tt Bool)");
}

#[test]
fn fold_core_icmp_plusminus() {
    let sexpr = "(app (app %core.icmp.Xygle (lit i32 Nat)) (tuple (lit 4 I32) (lit 2 I32)))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit ff Bool)");
}

#[test]
fn fold_core_icmp_minusplus() {
    let sexpr =
        "(app (app %core.icmp.xYgle (lit i32 Nat)) (tuple (lit 4171510507 I32) (lit 2 I32)))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit tt Bool)");
}

#[test]
fn fold_core_icmp_greater() {
    let sexpr = "(app (app %core.icmp.xyGle (lit i32 Nat)) (tuple (lit 4 I32) (lit 2 I32)))";
    let rewrites = equality_saturate(
        sexpr,
        OptionSelected::none(),
        vec![RuleSet::Core],
        CostFn::AstSize,
    );
    let res = first(rewrites);
    assert_eq!(res, "(lit tt Bool)");
}
