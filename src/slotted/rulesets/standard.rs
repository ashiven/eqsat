use crate::slotted::{MimSlotted, analysis::MimSlottedAnalysis};
use slotted_egraphs::Rewrite;

pub fn rules() -> Vec<Rewrite<MimSlotted, MimSlottedAnalysis>> {
    let rules = vec![let_var_same(), core_nat_add0()];

    rules
}

fn let_var_same() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(let $1 (scope ?def (var $1)))";
    let outpat = "?def";
    Rewrite::new("let_var_same", pat, outpat)
}

fn core_nat_add0() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(app %core.nat.add (tuple (cons (var $1) (cons (lit 0 Nat) nil))))";
    let outpat = "(var $1)";
    Rewrite::new("core_nat_add0", pat, outpat)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ffi::bridge::bridge::RuleSet;
    use crate::slotted::set_rulesets;
    use crate::slotted::util::assert_reaches;

    #[test]
    fn let_var_same() {
        set_rulesets(vec![RuleSet::Standard]);
        let a = "(let $foo (scope (lit 1 Nat) (var $foo)))";
        let b = "(lit 1 Nat)";
        let reached = assert_reaches(a, b, &rules(), 1);
        assert!(reached);
    }

    #[test]
    fn lam_var_add0() {
        set_rulesets(vec![RuleSet::Standard]);
        let a = "(root extern foo (lam $x (scope (lit ff Bool) (app %core.nat.add (tuple (cons (var $x) (cons (lit 0 Nat) nil)))))))";
        let b = "(root extern foo (lam $x (scope (lit ff Bool) (var $x))))";
        let reached = assert_reaches(a, b, &rules(), 1);
        assert!(reached);
    }
}
