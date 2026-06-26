use crate::slotted::util::split_sexprs;
use crate::slotted::{Mim, PARSE_STACK_SIZE};
use slotted_egraphs::*;
use stacker::grow;

pub fn pretty(sexpr: &str, _line_len: usize) -> String {
    let sexprs = split_sexprs(sexpr);

    let mut res = String::new();
    for (i, sexpr) in sexprs.iter().enumerate() {
        let parsed: RecExpr<Mim> = grow(PARSE_STACK_SIZE, || RecExpr::parse(sexpr).unwrap());
        res.push_str(&parsed.to_string());
        if i < sexprs.len() - 1 {
            res.push_str("\n\n");
        } else {
            res.push('\n');
        }
    }

    res
}
