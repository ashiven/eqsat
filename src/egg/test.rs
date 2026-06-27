use std::fs;

use crate::egg::Mim;
use crate::egg::rulesets::{get_rules, set_rulesets};
use crate::egg::util::split_sexprs;
use crate::ffi::bridge::{CostFn, OptionSelected, RuleSet};
use crate::{eqsat_egg, pretty_ffi};
use egg::*;

fn parse_sexprs(sexpr: &str) -> Vec<RecExpr<Mim>> {
    let sexprs = split_sexprs(sexpr);

    let mut res = vec![];
    for sexpr in sexprs {
        res.push(sexpr.parse().expect("Failed to parse RecExpr"));
    }
    res
}

fn eqsat_equals(file: &str, file_rw: &str) {
    let egg = fs::read_to_string(file).expect("Failed to read file.egg");

    let selected = OptionSelected::none();
    let nodes = eqsat_egg(&egg, selected, vec![], CostFn::AstSize);

    let egg = pretty_ffi(nodes, LINE_LEN);
    let egg_rw = fs::read_to_string(file_rw)
        .expect("Failed to read file_rw.egg")
        .replace("\r\n", "\n");

    assert_eq!(egg, egg_rw);
}

const LINE_LEN: usize = 80;

#[test]
fn get_ruleset_core() {
    set_rulesets(vec![RuleSet::Core]);
    let core = get_rules();
    assert_ne!(core.len(), 0);
}

#[test]
fn parse_loop_egg() {
    let loop_egg = fs::read_to_string("examples/loop.egg").expect("Failed to read loop.egg");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&loop_egg);
}

#[test]
fn eqsat_loop_egg() {
    eqsat_equals("examples/loop.egg", "examples/loop_rw.egg");
}

#[test]
fn parse_import_egg() {
    let import_egg = fs::read_to_string("examples/import.egg").expect("Failed to read import.egg");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&import_egg);
}

#[test]
fn eqsat_import_egg() {
    eqsat_equals("examples/import.egg", "examples/import_rw.egg");
}

#[test]
fn parse_fun_egg() {
    let fun_egg = fs::read_to_string("examples/fun.egg").expect("Failed to read fun.egg");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&fun_egg);
}

#[test]
fn eqsat_fun_egg() {
    eqsat_equals("examples/fun.egg", "examples/fun_rw.egg");
}

#[test]
#[ignore = "rec check in sexpr emitter currently bugged"]
fn parse_pow_egg() {
    let pow_egg = fs::read_to_string("examples/pow.egg").expect("Failed to read pow.egg");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&pow_egg);
}

#[test]
#[ignore = "rec check in sexpr emitter currently bugged"]
fn eqsat_pow_egg() {
    eqsat_equals("examples/pow.egg", "examples/pow_rw.egg");
}
