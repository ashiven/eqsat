use ffi::bridge::{CostFn, NodeFFI, RecExprFFI, RuleSet};

use crate::ffi::bridge::OptionSelected;

pub mod ffi;
mod mim_egg;
mod mim_slotted;

pub fn eqsat_egg(sexpr: &str, rulesets: Vec<RuleSet>, cost_fn: CostFn) -> Vec<RecExprFFI> {
    mim_egg::equality_saturate(sexpr, rulesets, cost_fn)
}

pub fn reaches_egg(
    _sexpr: &str,
    _rulesets: Vec<RuleSet>,
    _start_name: &str,
    _end_name: &str,
    _max_steps: usize,
) -> bool {
    // mim_egg::reaches(sexpr, rulesets, start_name, end_name, max_steps)
    true
}

pub fn pretty_egg(sexpr: &str, line_len: usize) -> String {
    mim_egg::pretty(sexpr, line_len)
}

pub fn eqsat_slotted(
    sexpr: &str,
    selected: OptionSelected,
    rulesets: Vec<RuleSet>,
    cost_fn: CostFn,
) -> Vec<RecExprFFI> {
    mim_slotted::equality_saturate(sexpr, selected, rulesets, cost_fn)
}

pub fn reaches_slotted(
    sexpr: &str,
    rulesets: Vec<RuleSet>,
    start_name: &str,
    end_name: &str,
    max_steps: usize,
) -> bool {
    mim_slotted::reaches(sexpr, rulesets, start_name, end_name, max_steps)
}

pub fn pretty_slotted(sexpr: &str, line_len: usize) -> String {
    mim_slotted::pretty(sexpr, line_len)
}

pub fn pretty_ffi(sexprs: Vec<RecExprFFI>, line_len: usize) -> String {
    ffi::pretty_ffi(sexprs, line_len)
}

pub fn node_ffi_str(mut node: NodeFFI) -> String {
    // Printing types along with nodes becomes too bloated
    node.type_ = RecExprFFI { nodes: vec![] };
    format!("{:?}", node)
}

pub fn type_str(type_: RecExprFFI, line_len: usize) -> String {
    ffi::pretty_ffi(vec![type_], line_len)
}
