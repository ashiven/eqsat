use crate::ffi::bridge::{MimKind, NodeFFI, RecExprFFI};
use std::fmt;

pub fn pretty_ffi(sexprs: Vec<RecExprFFI>, line_len: usize) -> String {
    let mut res = String::new();

    for (i, sexpr) in sexprs.iter().enumerate() {
        res.push_str(&sexpr.pretty(line_len));
        if i < sexprs.len() - 1 {
            res.push_str("\n\n");
        } else {
            res.push('\n');
        }
    }

    res
}

impl fmt::Display for NodeFFI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            MimKind::Let => f.write_str("let"),
            MimKind::Lam => f.write_str("lam"),
            MimKind::Con => f.write_str("con"),
            MimKind::Fun => f.write_str("fun"),
            MimKind::App => f.write_str("app"),
            MimKind::Var => f.write_str("var"),
            MimKind::Lit => f.write_str("lit"),
            MimKind::Pack => f.write_str("pack"),
            MimKind::Tuple => f.write_str("tuple"),
            MimKind::Extract => f.write_str("extract"),
            MimKind::Insert => f.write_str("insert"),
            MimKind::Rule => f.write_str("rule"),
            MimKind::Inj => f.write_str("inj"),
            MimKind::Merge => f.write_str("merge"),
            MimKind::Axm => f.write_str("axm"),
            MimKind::Match => f.write_str("match"),
            MimKind::Proxy => f.write_str("proxy"),
            MimKind::Join => f.write_str("join"),
            MimKind::Meet => f.write_str("meet"),
            MimKind::Bot => f.write_str("bot"),
            MimKind::Top => f.write_str("top"),
            MimKind::Arr => f.write_str("arr"),
            MimKind::Sigma => f.write_str("sigma"),
            MimKind::ImplicitPi => f.write_str("pi*"),
            MimKind::Pi => f.write_str("pi"),
            MimKind::Cn => f.write_str("cn"),
            MimKind::Fn => f.write_str("fn"),
            MimKind::Idx => f.write_str("idx"),
            MimKind::Hole => f.write_str("hole"),
            MimKind::Type => f.write_str("type"),
            MimKind::Reform => f.write_str("reform"),
            MimKind::TypeWrap => f.write_str("@"),
            MimKind::MetaVar => f.write_str("metavar"),
            MimKind::Root => f.write_str("root"),
            MimKind::Scope => f.write_str("scope"),
            MimKind::Cons => f.write_str("cons"),
            MimKind::Nil => f.write_str("nil"),
            MimKind::Num => f.write_str(&self.num.to_string()),
            MimKind::Symbol => f.write_str(&self.symbol),
            _ => todo!(),
        }
    }
}

// Source: https://github.com/egraphs-good/egg/blob/main/src/sexp.rs
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum Sexpr {
    String(String),
    List(Vec<Sexpr>),
    Empty,
}

impl fmt::Display for Sexpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sexpr::String(s) => {
                if s.contains(' ') || s.contains('(') || s.contains(')') || s.is_empty() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "{}", s)
                }
            }
            Sexpr::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
            Sexpr::Empty => write!(f, "()"),
        }
    }
}

// Source: https://github.com/egraphs-good/egg/blob/main/src/util.rs
fn pretty_print(buf: &mut String, sexpr: &Sexpr, width: usize, level: usize) -> std::fmt::Result {
    use std::fmt::Write;
    if let Sexpr::List(list) = sexpr {
        let indent = sexpr.to_string().len() > width;
        write!(buf, "(")?;

        for (i, val) in list.iter().enumerate() {
            if indent && i > 0 {
                writeln!(buf)?;
                for _ in 0..level {
                    write!(buf, "  ")?;
                }
            }
            pretty_print(buf, val, width, level + 1)?;
            if !indent && i < list.len() - 1 {
                write!(buf, " ")?;
            }
        }

        write!(buf, ")")?;
        Ok(())
    } else {
        write!(buf, "{}", sexpr.to_string().trim_matches('"'))
    }
}

// Source: https://github.com/egraphs-good/egg/blob/main/src/language.rs
impl RecExprFFI {
    fn to_sexpr(&self) -> Sexpr {
        let last = self.nodes.len() - 1;
        self.to_sexpr_rec(last, &mut |_| None)
    }

    fn to_sexpr_rec(&self, i: usize, f: &mut impl FnMut(u32) -> Option<String>) -> Sexpr {
        let node = &self.nodes[i];
        let op = Sexpr::String(node.to_string());
        if node.children.is_empty() && node.slot.is_empty() {
            op
        } else {
            let mut vec = vec![op];
            for child in node.children.iter() {
                vec.push(if let Some(s) = f(*child) {
                    return Sexpr::String(s);
                } else if (*child as usize) < i {
                    self.to_sexpr_rec(*child as usize, f)
                } else {
                    Sexpr::String(format!("<<<< CYCLE to {} = {:?} >>>>", i, node))
                })
            }
            // Some nodes introduce or use slots which don't
            // have their own nodes so we insert them manually.
            match node.kind {
                MimKind::Let => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Lam => {
                    if !node.slot.is_empty() {
                        vec.insert(vec.len() - 1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Con => {
                    if !node.slot.is_empty() {
                        vec.insert(vec.len() - 1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Fun => {
                    if !node.slot.is_empty() {
                        vec.insert(vec.len() - 1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Pack => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Var => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::ImplicitPi => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Pi => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Cn => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Fn => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Sigma => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                MimKind::Arr => {
                    if !node.slot.is_empty() {
                        vec.insert(1, Sexpr::String(node.slot.clone()))
                    }
                }
                _ => (),
            }
            Sexpr::List(vec)
        }
    }

    pub fn pretty(&self, width: usize) -> String {
        let sexp = self.to_sexpr();

        let mut buf = String::new();
        pretty_print(&mut buf, &sexp, width, 1).unwrap();
        buf
    }
}
