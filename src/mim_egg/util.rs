use crate::mim_egg::Mim;
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
