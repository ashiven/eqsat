use crate::{typ, isa};
use crate::mim_slotted::{MimSlotted, analysis::{AnalysisData, MimSlottedAnalysis}};
use slotted_egraphs::{AbstractVecSet, Rewrite, Slot};

type RW = Rewrite<MimSlotted, MimSlottedAnalysis>;

// Ruleset derived from: 
// https://github.com/memoryleak47/slotted-egraphs/blob/main/tests/rise/rewrite.rs

pub fn rules() -> Vec<RW> {
    let rules = vec![
        // EVAL
        beta(),
        eta(),
        eta_expansion(),
        let_unused(),
        let_var_same(),
        let_var_diff(),
        let_app(),
        let_lam_diff(),
        // RISE
        map_fusion(),
        map_fission(),
        remove_transpose_pair(),
        map_slide_before_transpose(),
        map_split_before_transpose(),
        slide_before_map_map_f(),
        split_before_map_map_f(),
        slide_before_map(),
        separate_dot_vh_simplified(),
        separate_dot_hv_simplified(),
    ];

    rules
}

fn beta() -> RW {
    let pat = "(app (lam $x (scope ?filter ?body)) ?e)";
    let outpat = "(let $x (scope ?e ?body))";
    Rewrite::new("beta", pat, outpat)
}

fn eta() -> RW {
    let pat = "(lam $x (scope ?filter (app ?fn (var $x))))";
    let outpat = "?fn";

    Rewrite::new_if("eta", pat, outpat, |subst, _| {
        !subst["fn"].slots().contains(&Slot::named("x"))
    })
}

fn eta_expansion() -> RW {
    let pat = "?fn";
    let outpat = "(lam $x (scope (lit ff Bool) (app ?fn (var $x))))";
    Rewrite::new_if("eta-expansion", pat, outpat, |subst, eg| {
        !isa!(subst, eg, "fn", MimSlotted::Lam(..)) && typ!(subst, eg, "fn", MimSlotted::Pi(..))
    })
}

fn let_unused() -> RW {
    let pat = "(let $name (scope ?def ?expr))";
    let outpat = "?expr";
    Rewrite::new_if("let-unused", pat, outpat, |subst, _| {
        !subst["expr"].slots().contains(&Slot::named("name"))
    })
}

fn let_var_same() -> RW {
    let pat = "(let $name (scope ?def (var $name)))";
    let outpat = "?def";
    Rewrite::new("let-var-same", pat, outpat)
}

fn let_var_diff() -> RW {
    let pat = "(let $name (scope ?def (var $other)))";
    let outpat = "(var $other)";
    Rewrite::new("let-var-diff", pat, outpat)
}

fn let_app() -> RW {
    let pat = "(let $name (scope ?def (app ?a ?b)))";
    let outpat = "(app (let $name (scope ?def ?a)) (let $name (scope ?def ?b)))";
    Rewrite::new_if("let-app", pat, outpat, |subst, _| {
        subst["a"].slots().contains(&Slot::named("name"))
            || subst["b"].slots().contains(&Slot::named("name"))
    })
}

fn let_lam_diff() -> RW {
    let pat = "(let $name (scope ?def (lam $x (scope ?filter ?body))))";
    let outpat = "(lam $x (scope ?filter (let $name (scope ?def ?body))))";
    Rewrite::new_if("let-lam-diff", pat, outpat, |subst, _| {
        subst["body"].slots().contains(&Slot::named("name"))
    })
}

// (map f) ((map g) arg) => (map λx.(f (g x))) arg
fn map_fusion() -> RW {
    let pat = "(app (app %rise.map ?f) (app (app %rise.map ?g) ?arg))";
    let outpat = "(app (app %rise.map (lam $x (scope (lit ff Bool) (app ?f (app ?g (var $x)))))) ?arg)";
    Rewrite::new("map-fusion", pat, outpat)
}

// map λx.(f (g x)) => λy.(map f) ((map λx.(g x)) y)
fn map_fission() -> RW {
    let pat = "(app %rise.map (lam $x (scope ?filter (app ?f ?gx))))";
    let outpat = " (lam $y (scope (lit ff Bool) (app (app %rise.map ?f) (app (app %rise.map (lam $x (scope ?filter ?gx))) (var $y)))))";
    Rewrite::new_if("map-fission", pat, outpat, |subst, _| {
        !subst["f"].slots().contains(&Slot::named("x"))
    })
}

fn remove_transpose_pair() -> RW {
    let pat = "(app %rise.transpose (app %rise.transpose ?arg))";
    let outpat = "?arg";
    Rewrite::new("remove-transpose-pair", pat, outpat)
}

fn map_slide_before_transpose() -> RW {
    let pat = "(app %rise.transpose (app (app %rise.map (app (app %rise.slide ?sz) ?sp)) ?y))";
    let outpat = "(app (app %rise.map %rise.transpose) (app (app (app %rise.slide ?sz) ?sp) (app %rise.transpose ?y)))";
    Rewrite::new("map-slide-before-transpose", pat, outpat)
}

fn map_split_before_transpose() -> RW {
    let pat = "(app %rise.transpose (app (app %rise.map (app %rise.split ?n)) ?y))";
    let outpat = "(app (app %rise.map %rise.transpose) (app (app %rise.split ?n) (app %rise.transpose ?y)))";
    Rewrite::new("map-split-before-transpose", pat, outpat)
}

fn slide_before_map_map_f() -> RW {
    let pat = "(app (app %rise.map (app %rise.map ?f)) (app (app (app %rise.slide ?sz) ?sp) ?y))";
    let outpat = "(app (app (app %rise.slide ?sz) ?sp) (app (app %rise.map ?f) ?y))";
    Rewrite::new("slide-before-map-map-f", pat, outpat)
}

fn split_before_map_map_f() -> RW {
    let pat = "(app (app %rise.map (app %rise.map ?f)) (app (app %rise.split ?n) ?y))";
    let outpat = "(app (app %rise.split ?n) (app (app %rise.map ?f) ?y))";
    Rewrite::new("slide-before-map-map-f", pat, outpat)
}

fn slide_before_map() -> RW {
    let pat = "(app (app (app %rise.slide ?sz) ?sp) (app (app %rise.map ?f) ?y))";
    let outpat =
        "(app (app %rise.map (app %rise.map ?f)) (app (app (app %rise.slide ?sz) ?sp) ?y))";
    Rewrite::new("slide-before-map", pat, outpat)
}

fn separate_dot_vh_simplified() -> RW {
    let pat = 
        "(app (app (app %rise.reduce %rise.add) (lit 0 Nat)) (app (app %rise.map (lam $x (app (app %rise.mul (app %rise.fst (var $x))) (app %rise.snd (var $x)))))
         (app (app %rise.zip (app %rise.join %rise.weights2d)) (app %rise.join ?nbh))))";
    let outpat = 
        "(app (app (app %rise.reduce %rise.add) (lit 0 Nat)) (app (app %rise.map (lam $x (app (app %rise.mul (app %rise.fst (var $x))) (app %rise.snd (var $x)))))
         (app (app %rise.zip %rise.weightsH) (app (app %rise.map (lam $sdvh (app (app (app %rise.reduce %rise.add) (lit 0 Nat)) (app (app %rise.map (lam $x (app (app %rise.mul (app %rise.fst (var $x))) (app %rise.snd (var $x)))))
         (app (app %rise.zip %rise.weightsV) (var $sdvh)))))) (app %rise.transpose ?nbh)))))";
    Rewrite::new("separate-dot-vh-simplified", pat, outpat)
}

fn separate_dot_hv_simplified() -> RW {
    let pat = 
        "(app (app (app %rise.reduce %rise.add) (lit 0 Nat)) (app (app %rise.map (lam $x (app (app %rise.mul (app %rise.fst (var $x))) (app %rise.snd (var $x)))))
         (app (app %rise.zip (app %rise.join %rise.weights2d)) (app %rise.join ?nbh))))";
    let outpat = 
        "(app (app (app %rise.reduce %rise.add) (lit 0 Nat)) (app (app %rise.map (lam $x (app (app %rise.mul (app %rise.fst (var $x))) (app %rise.snd (var $x)))))
         (app (app %rise.zip %rise.weightsV) (app (app %rise.map (lam $sdhv (app (app (app %rise.reduce %rise.add) (lit 0 Nat)) (app (app %rise.map (lam $x (app (app %rise.mul (app %rise.fst (var $x))) (app %rise.snd (var $x)))))
         (app (app %rise.zip %rise.weightsH) (var $sdhv)))))) (app %rise.transpose ?nbh)))))";
    Rewrite::new("separate-dot-hv-simplified", pat, outpat)
}

#[cfg(test)]
mod test {
    use slotted_egraphs::rw;

    use super::*;
    use crate::mim_slotted::util::assert_reaches;

    #[test]
    #[ignore]
    fn reduction() {
        let a = "
        (app 
            (lam $0 (scope (lit ff Bool)
                (app 
                    (lam $1 (scope (lit ff Bool)
                        (app (app (var $0) (var $1)) (app (app (var $0) (var $1)) (app (app (var $0) (var $1)) (app (app (var $0) (var $1)) (app (app (var $0) (var $1)) (app (app (var $0) (var $1)) (var $1))))))))) 
                    (lam $2 (scope (lit ff Bool)
                        (app (app %rise.add (var $2)) 1)))))) 
            (lam $3 (scope (lit ff Bool)
                (lam $4 (scope (lit ff Bool)
                    (lam $5 (scope (lit ff Bool)
                        (app (var $3) (app (var $4) (var $5))))))))))";

        let b = "
        (lam $0 (scope (lit ff Bool)
            (app (app %rise.add (app (app %rise.add (app (app %rise.add (app (app %rise.add (app (app %rise.add (app (app %rise.add (app (app %rise.add (var $0)) 1)) 1)) 1)) 1)) 1)) 1)) 1)))";

        let reached = assert_reaches(a, b, &rules(), 40);
        assert!(reached);
    }

    #[test]
    #[ignore]
    fn fission() {
        let a = "(app %rise.map (lam $42 (scope (lit ff Bool) (app f5 (app f4 (app f3 (app f2 (app f1 (var $42)))))))))";
        let b = "(lam $1 (scope (lit ff Bool) (app (app %rise.map (lam $42 (scope (lit ff Bool) (app f5 (app f4 (app f3 (var $42))))))) (app (app %rise.map (lam $42 (scope (lit ff Bool) (app f2 (app f1 (var $42)))))) (var $1)))))";

        let reached = assert_reaches(a, b, &rules(), 40);
        assert!(reached);
    }

    #[test]
    #[ignore = "doesn't work yet"]
    fn guided() {
        let transpose_mm: Rewrite<MimSlotted> = rw!("transpose-mm"; "(app (app %rise.o %rise.transpose) (app %rise.map (app %rise.map ?a)))" => "(app (app %rise.o (app %rise.map (app %rise.map ?a))) %rise.transpose)");
        let compose_assoc: Rewrite<MimSlotted> = rw!("compose-assoc"; "(app (app %rise.o ?a) (app (app %rise.o ?b) ?c))" => "(app (app %rise.o (app (app %rise.o a) ?b)) ?c)");
        let map_fuse: Rewrite<MimSlotted> = rw!("map-fuse"; "(app (app %rise.o (app %rise.map ?a)) (app %rise.map ?b))" => "(app %rise.map (app (app %rise.o ?a) ?b))");

        let a = "(fun $_11870 (scope (lit ff Bool) 
                            (let $return_11877 (scope (extract (var $_11870) (lit tt Bool)) 
                            (let $mapper_12055 (scope (app (app %rise.o (app %rise.map (app %rise.map f_11814))) (app (app %rise.o %rise.transpose) (app %rise.map (app %rise.map g_11838)))) 
                            (let $arg_11871 (scope (extract (var $_11870) (lit ff Bool)) 
                                (app (var $return_11877) (app (var $mapper_12055) (var $arg_11871)))))))))))";
        let b =  "(fun $_12079 (scope (lit ff Bool) 
                            (let $return_12086 (scope (extract (var $_12079) (lit tt Bool)) 
                            (let $mapper_12183 (scope (app (app %rise.o (app %rise.map (app %rise.map (app (app %rise.o f_11814) g_11838)))) %rise.transpose) 
                            (let $arg_12080 (scope (extract (var $_12079) (lit ff Bool)) 
                                (app (var $return_12086) (app (var $mapper_12183) (var $arg_12080)))))))))))";

        let reached = assert_reaches(a, b, &[transpose_mm, compose_assoc, map_fuse], 100);
        assert!(reached);
    }
}
