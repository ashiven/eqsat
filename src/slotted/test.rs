use regex::Regex;
use std::fs;

use crate::ffi::bridge::{CostFn, OptionSelected, RuleSet};
use crate::slotted::{Mim, get_rules, set_rulesets, split_sexprs};
use crate::{eqsat_slotted, pretty_ffi};
use slotted_egraphs::*;

fn parse_sexprs(sexpr: &str) -> Vec<RecExpr<Mim>> {
    let sexprs = split_sexprs(sexpr);

    let mut res = vec![];
    for sexpr in sexprs {
        res.push(RecExpr::parse(&sexpr).expect("Failed to parse RecExpr"));
    }
    res
}

fn eqsat_equals(file: &str, file_rw: &str) {
    let slotted = fs::read_to_string(file).expect("Failed to read file.slotted");

    let selected = OptionSelected::none();
    let nodes = eqsat_slotted(&slotted, selected, vec![], CostFn::AstSize);

    let slotted = pretty_ffi(nodes, LINE_LEN);
    let slotted_rw = fs::read_to_string(file_rw)
        .expect("Failed to read file_rw.slotted")
        .replace("\r\n", "\n");

    let slot_re = Regex::new(r"\$[_A-Za-z0-9]+").unwrap();
    let slotted = slot_re.replace_all(&slotted, "slot");
    let slotted_rw = slot_re.replace_all(&slotted_rw, "slot");

    assert_eq!(slotted, slotted_rw);
}

const LINE_LEN: usize = 80;

#[test]
fn get_ruleset_standard() {
    set_rulesets(vec![RuleSet::Standard]);
    let standard = get_rules();
    assert_ne!(standard.len(), 0);
}

#[test]
fn parse_loop_slotted() {
    let loop_slotted =
        fs::read_to_string("examples/loop.slotted").expect("Failed to read loop.slotted");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&loop_slotted);
}

#[test]
fn eqsat_loop_slotted() {
    eqsat_equals("examples/loop.slotted", "examples/loop_rw.slotted");
}

#[test]
fn parse_import_slotted() {
    let import_slotted =
        fs::read_to_string("examples/import.slotted").expect("Failed to read import.slotted");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&import_slotted);
}

#[test]
fn eqsat_import_slotted() {
    eqsat_equals("examples/import.slotted", "examples/import_rw.slotted");
}

#[test]
fn parse_fun_slotted() {
    let fun_slotted =
        fs::read_to_string("examples/fun.slotted").expect("Failed to read fun.slotted");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&fun_slotted);
}

#[test]
fn eqsat_fun_slotted() {
    eqsat_equals("examples/fun.slotted", "examples/fun_rw.slotted");
}

#[test]
#[ignore = "rec check in sexpr emitter currently bugged"]
fn parse_pow_slotted() {
    let pow_slotted =
        fs::read_to_string("examples/pow.slotted").expect("Failed to read pow.slotted");
    let _parsed: Vec<RecExpr<Mim>> = parse_sexprs(&pow_slotted);
}

#[test]
#[ignore = "rec check in sexpr emitter currently bugged"]
fn eqsat_pow_slotted() {
    eqsat_equals("examples/pow.slotted", "examples/pow_rw.slotted");
}
