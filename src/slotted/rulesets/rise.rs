use crate::{typ, isa};
use crate::slotted::{Mim, analysis::{AnalysisData, MimAnalysis}};
use slotted_egraphs::{AbstractVecSet, Rewrite, Slot};

type RW = Rewrite<Mim, MimAnalysis>;

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
        !isa!(subst, eg, "fn", Mim::Lam(..)) && typ!(subst, eg, "fn", Mim::Pi(..))
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
    use crate::{ffi::bridge::RuleSet, slotted::{rulesets::get_rules, set_rulesets, util::assert_reaches}};

    #[test]
    #[ignore = "works but is slow"]
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
    #[ignore = "works but is slow"]
    fn fission() {
        let a = "(app %rise.map (lam $42 (scope (lit ff Bool) (app f5 (app f4 (app f3 (app f2 (app f1 (var $42)))))))))";
        let b = "(lam $1 (scope (lit ff Bool) (app (app %rise.map (lam $42 (scope (lit ff Bool) (app f5 (app f4 (app f3 (var $42))))))) (app (app %rise.map (lam $42 (scope (lit ff Bool) (app f2 (app f1 (var $42)))))) (var $1)))))";

        let reached = assert_reaches(a, b, &rules(), 40);
        assert!(reached);
    }

    #[test]
    fn guided() {
        let transpose_mm: Rewrite<Mim> = rw!("transpose-mm"; "(app (app %rise.o %rise.transpose) (app %rise.map (app %rise.map ?a)))" => "(app (app %rise.o (app %rise.map (app %rise.map ?a))) %rise.transpose)");
        let compose_assoc: Rewrite<Mim> = rw!("compose-assoc"; "(app (app %rise.o ?a) (app (app %rise.o ?b) ?c))" => "(app (app %rise.o (app (app %rise.o ?a) ?b)) ?c)");
        let map_fuse: Rewrite<Mim> = rw!("map-fuse"; "(app (app %rise.o (app %rise.map ?a)) (app %rise.map ?b))" => "(app %rise.map (app (app %rise.o ?a) ?b))");

        // (map (map f)) o (transpose o (map (map g)))
        // 1) -> (map (map f)) o ((map (map g)) o transpose)
        // 2) -> ((map (map f)) o (map (map g))) o transpose
        // 3) -> (map (map f o map g)) o transpose
        // 3) -> (map (map (f o g))) o transpose

        let a = "(fun $1 (scope (lit ff Bool)
                            (let $return (scope (extract (var $1) (lit tt Bool))
                            (let $mapper (scope (app (app %rise.o (app %rise.map (app %rise.map f))) (app (app %rise.o %rise.transpose) (app %rise.map (app %rise.map g))))
                            (let $arg (scope (extract (var $1) (lit ff Bool))
                                (app (var $return) (app (var $mapper) (var $arg)))))))))))";
        let b = "(fun $1 (scope (lit ff Bool)
                            (let $return (scope (extract (var $1) (lit tt Bool))
                            (let $mapper (scope (app (app %rise.o (app %rise.map (app %rise.map (app (app %rise.o f) g)))) %rise.transpose)
                            (let $arg (scope (extract (var $1) (lit ff Bool))
                                (app (var $return) (app (var $mapper) (var $arg)))))))))))";

        let reached = assert_reaches(a, b, &[transpose_mm, compose_assoc, map_fuse], 100);
        assert!(reached);
    }

    #[test]
    fn rule_fuse() {
        let _map_fuse: Rewrite<Mim, MimAnalysis> = rw!("map-fuse";
        "(app (app (app %rise.o (tuple (cons ?A (cons ?B (cons ?C nil)))))
         (app (app (app %rise.map ?n) (tuple (cons ?D (cons ?E nil)))) ?a))
         (app (app (app %rise.map ?n) (tuple (cons ?F (cons ?D nil)))) ?b))"
        => "(app (app (app %rise.map ?n) (tuple (cons ?F (cons ?E nil))))
            (app (app (app %rise.o (tuple (cons ?F (cons ?D (cons ?E nil))))) ?a) ?b))");

        let map_fuse_gen: Rewrite<Mim, MimAnalysis> = rw!("map-fuse-gen";
        "(app (app (app %rise.o (tuple (cons (arr $1 (scope ?n ?A)) (cons (arr $2 (scope ?n ?B)) (cons (arr $3 (scope ?n ?C)) nil)))))
         (app (app (app %rise.map ?n) (tuple (cons ?B (cons ?C nil)))) ?a))
         (app (app (app %rise.map ?n) (tuple (cons ?A (cons ?B nil)))) ?b))"
        => "(app (app (app %rise.map ?n) (tuple (cons ?A (cons ?C nil))))
            (app (app (app %rise.o (tuple (cons ?A (cons ?B (cons ?C nil))))) ?a) ?b))");

        set_rulesets(vec![RuleSet::Normalize]);
        let mut rules = get_rules();
        rules.push(map_fuse_gen);

        let a = "(fun $_82673 (scope (lit ff Bool)
                            (let $return_82779 (scope (extract (var $_82673) (lit tt Bool))
                            (let $mapper_82777 (scope
                                (app (app (app %rise.o (tuple (cons (arr $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) nil)))))
                                    (app (app (app %rise.o (pack $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))))
                                        (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) f_82746)))
                                        (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) g_82699))))
                                    (app (app %rise.transpose (tuple (cons (lit 3 Nat) (cons (lit 4 Nat) nil)))) Nat))
                            (let $arg_82674 (scope (extract (var $_82673) (lit ff Bool))
                                (app (var $return_82779) (app (var $mapper_82777) (var $arg_82674)))))))))))";

        let _i = "(fun $_83012 (scope (lit ff Bool)
                            (let $return_83024 (scope (extract (var $_83012) (lit tt Bool))
                            (let $mapper_83022 (scope
                                (app (app (app %rise.o (tuple (cons (arr $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) nil)))))
                                    (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                        (app (app (app %rise.o (pack $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) f_82746))
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) g_82699))))
                                    (app (app %rise.transpose (tuple (cons (lit 3 Nat) (cons (lit 4 Nat) nil)))) Nat))
                            (let $arg_83013 (scope (extract (var $_83012) (lit ff Bool))
                                (app (var $return_83024) (app (var $mapper_83022) (var $arg_83013)))))))))))";


        let b = " (fun $_83027 (scope (lit ff Bool) 
                            (let $return_83040 (scope (extract (var $_83027) (lit tt Bool)) 
                            (let $mapper_83038 (scope 
                                (app (app (app %rise.o (tuple (cons (arr $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) nil))))) 
                                    (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat))))) 
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) 
                                            (app (app (app %rise.o (pack $dummy (scope (lit 3 Nat) Nat))) f_82746) g_82699)))) 
                                    (app (app %rise.transpose (tuple (cons (lit 3 Nat) (cons (lit 4 Nat) nil)))) Nat)) 
                            (let $arg_83028 (scope (extract (var $_83027) (lit ff Bool)) 
                                (app (var $return_83040) (app (var $mapper_83038) (var $arg_83028)))))))))))";

        let reached = assert_reaches(a, b, &rules, 5);
        assert!(reached);
    }

    #[test]
    fn rule_assoc() {
        let compose_assoc: Rewrite<Mim> = rw!("compose-assoc";
        "(app (app (app %rise.o (tuple (cons ?A (cons ?C (cons ?D nil))))) ?a)
         (app (app (app %rise.o (tuple (cons ?A (cons ?B (cons ?C nil))))) ?b) ?c))"
        => "(app (app (app %rise.o (tuple (cons ?A (cons ?B (cons ?D nil)))))
            (app (app (app %rise.o (tuple (cons ?B (cons ?C (cons ?D nil))))) ?a) ?b)) ?c)");

        // We need to consider the normalization where a tuple of three equivalent terms
        // gets reduced to a pack in the expected term.
        let normalize_three_tuple: Rewrite<Mim> = rw!("normalize-three-tuple";
        "(tuple (cons ?a (cons ?a (cons ?a nil))))"
        => "(pack $dummy (scope (lit 3 Nat) ?a))");

        let a = "(fun $_39665 (scope (lit ff Bool)
                            (let $return_39672 (scope (extract (var $_39665) (lit tt Bool))
                            (let $mapper_39891 (scope
                                (app (app (app %rise.o (tuple (cons (arr $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) nil)))))
                                    (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                    (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) f_39609)))
                                        (app (app (app %rise.o (tuple (cons (arr $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) nil)))))
                                            (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                            (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) g_39633)))
                                            (app (app %rise.transpose (tuple (cons (lit 3 Nat) (cons (lit 4 Nat) nil)))) Nat)))
                            (let $arg_39666 (scope (extract (var $_39665) (lit ff Bool)) 
                                (app (var $return_39672) (app (var $mapper_39891) (var $arg_39666)))))))))))";


        let b = "(fun $_39915 (scope (lit ff Bool)
                            (let $return_39922 (scope (extract (var $_39915) (lit tt Bool))
                            (let $mapper_40106 (scope
                                (app (app (app %rise.o (tuple (cons (arr $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) (cons (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))) nil)))))
                                    (app (app (app %rise.o (pack $dummy (scope (lit 3 Nat) (arr $dummy (scope (lit 4 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))))
                                        (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) f_39609)))
                                        (app (app (app %rise.map (lit 4 Nat)) (pack $dummy (scope (lit 2 Nat) (arr $dummy (scope (lit 3 Nat) Nat)))))
                                        (app (app (app %rise.map (lit 3 Nat)) (pack $dummy (scope (lit 2 Nat) Nat))) g_39633))))
                                    (app (app %rise.transpose (tuple (cons (lit 3 Nat) (cons (lit 4 Nat) nil)))) Nat))
                            (let $arg_39916 (scope (extract (var $_39915) (lit ff Bool))
                                (app (var $return_39922) (app (var $mapper_40106) (var $arg_39916)))))))))))";


        let reached = assert_reaches(a, b, &[compose_assoc, normalize_three_tuple], 5);
        assert!(reached);
    }
}
