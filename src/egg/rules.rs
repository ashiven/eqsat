use crate::egg::{Mim, analysis::MimAnalysis};
use crate::expect;
use egg::*;

pub fn convert_rules(sexprs: &mut Vec<String>, rules: &mut Vec<Rewrite<Mim, MimAnalysis>>) {
    sexprs.retain(|sexpr| {
        if sexpr.trim().starts_with("(rule") {
            let rule: RecExpr<Mim> = sexpr.parse().unwrap();

            let [name, meta_var, lhs, rhs, _guard] = expect!(rule[rule.root()], Mim::Rule([name, meta_var, lhs, rhs, guard]) => [name,meta_var,lhs, rhs, guard] );


            let rule_name = if let Mim::Symbol(s) = &rule[name] {
                s
            } else {
                panic!("Failed to parse rule name.")
            };

            let mut meta_vars: Vec<String> = Vec::new();
            let nth_node = |id: Id| rule[id].clone();

            let meta_var_rexpr = rule[meta_var].build_recexpr(nth_node);
            for node in meta_var_rexpr.iter() {
                if let Mim::MetaVar(ids) = node
                    && let [var_name, ..] = &**ids
                {
                    if let Mim::Symbol(s) =
                        &meta_var_rexpr[*var_name]
                    {
                        meta_vars.push(s.clone());
                    } else {
                        panic!("Failed to parse meta variable name.")
                    };
                }
            }

            let mut lhs_rexpr = rule[lhs].build_recexpr(nth_node);
            for (_id, node) in lhs_rexpr.items_mut() {
                if let Mim::Symbol(s) = node
                    && meta_vars.contains(s)
                {
                    s.insert(0, '?')
                }
            }

            let mut rhs_rexpr = rule[rhs].build_recexpr(nth_node);
            for (_id, node) in rhs_rexpr.items_mut() {
                if let Mim::Symbol(s) = node
                    && meta_vars.contains(s)
                {
                    s.insert(0, '?')
                }
            }

            let pat: Pattern<Mim> = lhs_rexpr.pretty(80).parse().unwrap();
            let outpat: Pattern<Mim> = rhs_rexpr.pretty(80).parse().unwrap();
            let rule: Rewrite<Mim, MimAnalysis> = Rewrite::new(rule_name, pat, outpat).unwrap();
            rules.push(rule);
            false
        } else {
            true
        }
    });
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn convert_custom_rule() {
        let rule = "
        (rule foo
            (metavar foo
                (metavar a_22735)
                (metavar b_22734))
            (app
                %core.nat.add
                (tuple
                    (app
                        %core.nat.sub
                        (tuple
                            b_22734
                            a_22735))
                    a_22735))
            b_22734
            (lit tt Bool))";

        let mut sexprs = vec![rule.to_string()];
        let mut rules = Vec::new();
        convert_rules(&mut sexprs, &mut rules);

        assert_eq!(rules.len(), 1);
        assert_eq!(
            format!("{:#?}", rules[0]),
            "Rewrite {\n    name: \"foo\",\n    searcher: (app \"%core.nat.add\" (tuple (app \"%core.nat.sub\" (tuple ?b_22734 ?a_22735)) ?a_22735)),\n    applier: ?b_22734,\n}"
        );
    }
}
