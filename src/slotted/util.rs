use crate::slotted::Mim;
use slotted_egraphs::*;

pub fn split_sexprs(sexpr: &str) -> Vec<String> {
    let normalized = sexpr.replace("\r\n", "\n");

    normalized
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn get_literal(lit_expr: &RecExpr<Mim>) -> u64 {
    let lit_val = lit_expr.children.first().expect("Expected literal value");
    if let Mim::Symbol(s) = lit_val.node {
        match s.as_str() {
            "ff" => 0,
            "tt" => 1,
            "i1" => 2,
            "i8" => 0x100,
            "i16" => 0x10000,
            "i32" => 0x100000000,
            _ => panic!("Unknown literal alias"),
        }
    } else if let Mim::Num(n) = lit_val.node {
        n
    } else {
        panic!("Expected literal value to be a symbol or a number");
    }
}

pub fn cons_to_vec(cons_expr: &RecExpr<Mim>) -> Vec<RecExpr<Mim>> {
    let mut res = vec![];

    let mut curr_cons = cons_expr;
    while let RecExpr {
        node: Mim::Cons(..),
        children,
    } = curr_cons
    {
        let curr_elem = children.first().expect("Expected cons elem");
        curr_cons = children.get(1).expect("Expected next cons");
        res.push(curr_elem.clone());
    }

    res
}

pub fn cons_elem_at(cons_expr: &RecExpr<Mim>, index: u64) -> RecExpr<Mim> {
    let mut i = 0;
    let mut curr_cons = cons_expr;
    while let RecExpr {
        node: Mim::Cons(..),
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

pub fn cons_insert_at(cons_expr: &RecExpr<Mim>, value: &RecExpr<Mim>, index: u64) -> RecExpr<Mim> {
    let mut i = 0;
    let mut curr_cons = cons_expr.clone();
    let mut cursor = &mut curr_cons;

    while let RecExpr {
        node: Mim::Cons(..),
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
