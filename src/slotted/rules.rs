use crate::slotted::{Mim, analysis::MimAnalysis};
use regex::Regex;
use slotted_egraphs::*;

pub fn convert_rules(sexprs: &mut Vec<String>, rules: &mut Vec<Rewrite<Mim, MimAnalysis>>) {
    sexprs.retain(|sexpr| {
        // let parsed: RecExpr<Mim> = RecExpr::parse(sexpr).unwrap();
        // if let Mim::Rule(..) = parsed.node {
        //
        // We initially used the more robust check above, however I realized
        // that the operation of parsing a rec expr with type annotations can
        // be very expensive and so it would be better to use the cheap shortcut
        // below to check for rule sexprs
        //
        // (rule <name> <meta_var> <lhs> <rhs> <guard>)
        if sexpr.trim().starts_with("(rule") {
            let parsed: RecExpr<Mim> = RecExpr::parse(sexpr).unwrap();

            let mut rule_name = "";
            if let Mim::Symbol(s) = parsed.children[0].node {
                rule_name = s.into();
            }

            let mut meta_vars: Vec<String> = Vec::new();
            fn lookup(rec_expr: &RecExpr<Mim>, meta_vars: &mut Vec<String>) {
                if let RecExpr {
                    node: Mim::MetaVar(..),
                    children,
                } = rec_expr
                {
                    let name_expr = children.first().expect("Expected meta var name");
                    if let Mim::Symbol(s) = name_expr.node {
                        meta_vars.push(s.to_string());
                    } else {
                        panic!("Expected meta var name to be a symbol");
                    }
                }
                rec_expr.children.iter().for_each(|c| lookup(c, meta_vars));
            }
            lookup(&parsed, &mut meta_vars);

            let lhs_rexpr = &parsed.children[2];
            let rhs_rexpr = &parsed.children[3];

            let mut pat = format!("{}", re_to_pattern(lhs_rexpr));
            let mut outpat = format!("{}", re_to_pattern(rhs_rexpr));

            inject_meta_vars(&meta_vars, &mut pat);
            inject_meta_vars(&meta_vars, &mut outpat);

            let mut counter = 1;
            replace_dummy_slots(&mut counter, &mut pat);
            replace_dummy_slots(&mut counter, &mut outpat);

            let rule: Rewrite<Mim, MimAnalysis> = Rewrite::new(rule_name, &pat, &outpat);
            rules.push(rule);

            false
        } else {
            true
        }
    });
}

fn replace_dummy_slots(counter: &mut usize, pattern: &mut String) {
    let mut result = String::with_capacity(pattern.len());
    let mut parts = pattern.split("$dummy");

    if let Some(first) = parts.next() {
        result.push_str(first);
    }

    for part in parts {
        result.push_str(&format!("$dummy{}", counter));
        *counter += 1;
        result.push_str(part);
    }

    *pattern = result;
}

fn inject_meta_vars(meta_vars: &[String], pattern: &mut String) {
    let re = Regex::new(r"[_A-Za-z0-9]+").unwrap();

    let res = re.replace_all(pattern, |caps: &regex::Captures| {
        let mut name = caps[0].to_string();

        if !meta_vars.contains(&name) {
            return name;
        }

        if let Some((base, suffix)) = name.rsplit_once('_')
            && suffix.chars().all(|c| c.is_ascii_digit())
        {
            name = base.to_string();
        }

        format!("?{}", name)
    });

    *pattern = res.into_owned();
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn convert_custom_rule() {
        let rule = "
        (rule 
            foo
            (cons
                (metavar
                    a_22735
                    Nat)
            (cons
                (metavar
                    slot_b_22734
                    Nat)
            nil))
            (app
                %core.nat.add
                (tuple
                    (cons
                        (app
                            %core.nat.sub
                            (tuple
                                (cons
                                    slot_b_22734
                                (cons
                                    a_22735
                                nil))))
                    (cons
                        a_22735
                    nil))))
            slot_b_22734
            (lit tt Bool))";

        let mut sexprs = vec![rule.to_string()];
        let mut rules = Vec::new();
        convert_rules(&mut sexprs, &mut rules);

        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn rule_replace_dummy_slots() {
        let mut before =
            "(app (app (app %rise.o (pack $dummy (scope (lit 3 Nat) (arr $dummy (scope ?n Nat)))))
         (app (app (app %rise.map ?n) (pack $dummy (scope (lit 2 Nat) Nat))) ?a))
         (app (app (app %rise.map ?n) (pack $dummy (scope (lit 2 Nat) Nat))) ?b))"
                .to_string();

        let after =
        "(app (app (app %rise.o (pack $dummy1 (scope (lit 3 Nat) (arr $dummy2 (scope ?n Nat)))))
         (app (app (app %rise.map ?n) (pack $dummy3 (scope (lit 2 Nat) Nat))) ?a))
         (app (app (app %rise.map ?n) (pack $dummy4 (scope (lit 2 Nat) Nat))) ?b))"
            .to_string();

        let mut counter = 1;
        replace_dummy_slots(&mut counter, &mut before);

        assert_eq!(before, after);
    }

    #[test]
    fn rule_inject_meta_vars() {
        let meta_vars = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut before = "(app d (app a (app c b)))".to_string();
        let after = "(app d (app ?a (app ?c ?b)))".to_string();

        inject_meta_vars(&meta_vars, &mut before);

        assert_eq!(before, after);
    }

    #[test]
    fn rule_inject_meta_vars_substr() {
        let meta_vars = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut before = "(app d (app abc (app c b)))".to_string();
        let after = "(app d (app abc (app ?c ?b)))".to_string();

        inject_meta_vars(&meta_vars, &mut before);

        assert_eq!(before, after);
    }
}
