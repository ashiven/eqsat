use crate::egg::Mim;
use crate::egg::analysis::MimAnalysis;
use egg::{Applier, EGraph, Id, PatternAst, Rewrite, Subst, Var, rewrite};

pub fn rules() -> Vec<Rewrite<Mim, MimAnalysis>> {
    let rules = vec![
        beta(),
        eta(),
        eta_expansion(),
        let_unused(),
        let_var_same(),
        let_var_diff(),
        let_app(),
        let_lam_diff(),
    ];

    rules
}

type RW = Rewrite<Mim, MimAnalysis>;

fn beta() -> RW {
    rewrite!("beta"; "(app (lam ?x ?filter ?body) ?expr)" => "(let ?x ?expr ?body)")
}

fn not_contains_var(
    term: &'static str,
    var: &'static str,
) -> impl Fn(&mut EGraph<Mim, MimAnalysis>, Id, &Subst) -> bool {
    let _term: Var = term.parse().unwrap();
    let _var: Var = var.parse().unwrap();
    // TODO: Implement check - needs free variable analysis?
    move |_egraph, _, _subst| true
}

fn eta() -> RW {
    rewrite!("eta"; "(lam ?x ?filter (app ?f (var ?x)))" => "?f" if not_contains_var("?f", "?x"))
}

// TODO: Without check for ?f is not Lam and typeof(?f) is Pi -> stack overflow
fn eta_expansion() -> RW {
    rewrite!("eta-expansion"; "?f" => { EtaExpand  {
        f: "?f".parse().unwrap(),
        ast: "(lam x_uid (lit ff Bool) (app ?f (var x_uid)))".parse().unwrap(),
    }})
}

#[allow(dead_code)]
struct EtaExpand {
    f: Var,
    ast: PatternAst<Mim>,
}

impl Applier<Mim, MimAnalysis> for EtaExpand {
    fn apply_one(
        &self,
        egraph: &mut EGraph<Mim, MimAnalysis>,
        eclass: Id,
        subst: &Subst,
        _searcher_ast: Option<&PatternAst<Mim>>,
        _rule_name: egg::Symbol,
    ) -> Vec<Id> {
        // TODO: Regex over sexprs and extract largest uid suffix - store in global mut var, then add +1 on each rule application
        let uid = 12345;
        let x = egraph.add(Mim::Symbol(format!("x_{}", uid)));
        let var_x = egraph.add(Mim::Var(x));
        let f = subst[self.f];
        let app = egraph.add(Mim::App([f, var_x]));
        let ff = egraph.add(Mim::Symbol("ff".to_string()));
        let bl = egraph.add(Mim::Symbol("Bool".to_string()));
        let lit_ff = egraph.add(Mim::Lit([ff, bl]));
        let lam = egraph.add(Mim::Lam(Box::new([x, lit_ff, app])));
        if egraph.union(eclass, lam) {
            vec![lam]
        } else {
            vec![]
        }
    }
}

fn let_unused() -> RW {
    rewrite!("let-unused"; "(let ?name ?def ?expr)" => "?expr" if not_contains_var("?expr", "?name"))
}

fn let_var_same() -> RW {
    rewrite!("let-var-same"; "(let ?name ?def (var ?name))" => "?def")
}

fn let_var_diff() -> RW {
    rewrite!("let-var-diff"; "(let ?name ?def (var ?other))" => "(var ?other)")
}

fn let_app() -> RW {
    rewrite!("let-app"; "(let ?name ?def (app ?a ?b))" => "(app (let ?name ?def ?a) (let ?name ?def ?b))")
}

fn let_lam_diff() -> RW {
    rewrite!("let-lam-diff"; "(let ?name ?def (lam ?x ?filter ?body))" => "(lam ?x ?filter (let ?name ?def ?body))")
}

#[cfg(test)]
mod test {
    use egg::Runner;

    use super::*;

    #[test]
    fn eta_expand() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let f = "f".parse().unwrap();
        let f_id = eg.add_expr(&f);

        let runner = Runner::<Mim, MimAnalysis>::default()
            .with_egraph(eg)
            .with_iter_limit(1);
        let rules = rules();

        let runner = runner.run(&rules);

        let nodes = &runner.egraph[f_id].nodes;

        assert!(nodes.len() == 2);
    }
}
