use crate::egg::Mim;
use egg::*;

pub fn pretty(sexpr: &str, line_len: usize) -> String {
    let normalized = sexpr.replace("\r\n", "\n");
    let mut sexprs: Vec<&str> = normalized.split("\n\n").collect();
    sexprs.retain(|s| !s.trim().is_empty());
    let mut res = String::new();

    for (i, sexpr) in sexprs.iter().enumerate() {
        let parsed: RecExpr<Mim> = sexpr.parse().unwrap();
        res.push_str(parsed.pretty(line_len).as_str());
        if i < sexprs.len() - 1 {
            res.push_str("\n\n");
        } else {
            res.push('\n');
        }
    }

    res
}
