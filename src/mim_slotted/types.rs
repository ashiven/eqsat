use crate::mim_slotted::MimSlotted;
use crate::mim_slotted::analysis::{AnalysisData, MimSlottedAnalysis};
use slotted_egraphs::*;

/***********************************************************/
/* Conversion from type-annotated RecExpr to TypedRecExpr  */
/***********************************************************/

pub type TypeExpr = RecExpr<MimSlotted>;

#[derive(Debug, Clone)]
pub struct TypedRecExpr {
    node: MimSlotted,
    children: Vec<TypedRecExpr>,
    type_: TypeExpr,
}

pub(crate) fn extract_type_annotations(rec_expr: &RecExpr<MimSlotted>) -> TypedRecExpr {
    if let MimSlotted::TypeWrap(..) = rec_expr.node {
        let type_expr = rec_expr.children[0].clone();
        let expr = &rec_expr.children[1];
        let mut stripped = extract_type_annotations(expr);
        stripped.type_ = type_expr;

        // Instead of the actual type, we give var nodes a hole type
        // to be inferred later on by the mim compiler. This is because
        // all vars are represented with the same singleton var eclass
        // and we can't store different vars' types on this single eclass.
        if let MimSlotted::Var(_slot) = expr.node {
            stripped.type_ = TypeExpr::hole();
        }

        return stripped;
    }

    let mut res = TypedRecExpr {
        node: rec_expr.node.clone(),
        children: rec_expr
            .children
            .iter()
            .map(extract_type_annotations)
            .collect(),
        type_: TypeExpr::nil(),
    };

    // Since it was too difficult to correctly type-annotate let
    // nodes in the sexpr backend, we just infer the type of the let
    // node via the type annotation of the expression it binds into
    if let MimSlotted::Let(..) = rec_expr.node {
        let let_scope = &res.children[0];
        let let_expr = &let_scope.children[1];
        res.type_ = let_expr.type_.clone();
    }

    res
}

pub(crate) fn add_expr_typed(
    eg: &mut EGraph<MimSlotted, MimSlottedAnalysis>,
    rec_expr: TypedRecExpr,
) -> AppliedId {
    let mut node = rec_expr.node;
    let mut child_ids = node.applied_id_occurrences_mut();

    for (i, child) in rec_expr.children.into_iter().enumerate() {
        *(child_ids[i]) = add_expr_typed(eg, child);
    }

    let eclass_applied_id = eg.add(node);

    let eclass_id = eclass_applied_id.id;
    let analysis_data = eg.analysis_data_mut(eclass_id);
    analysis_data.type_ = rec_expr.type_;

    eclass_applied_id
}

/***********************************************************/
/*  Analysis maintaining type information on eclasses      */
/***********************************************************/

pub type TypeData = TypeExpr;

// Returns the ast size of a term, useful for comparing type expr sizes in merge
pub(crate) fn term_size(type_expr: &TypeExpr) -> usize {
    fn size(type_expr: &TypeExpr) -> usize {
        1 + type_expr.children.iter().map(size).sum::<usize>()
    }
    size(type_expr)
}

// Returns the number of holes in a type expr, also useful for comparison in merge
pub(crate) fn hole_amount(type_expr: &TypeExpr) -> usize {
    fn holes(type_expr: &TypeExpr) -> usize {
        if let MimSlotted::Hole(..) = type_expr.node {
            1 + type_expr.children.iter().map(holes).sum::<usize>()
        } else {
            type_expr.children.iter().map(holes).sum::<usize>()
        }
    }
    holes(type_expr)
}

trait TypeConstructors {
    fn hole() -> Self;
    fn nil() -> Self;
    fn cons(elem: TypeExpr, next: TypeExpr) -> Self;
    fn arr(arity: TypeExpr, body: TypeExpr) -> Self;
    fn sigma(elem_cons: TypeExpr) -> Self;
    fn pi(dom: TypeExpr, codom: TypeExpr) -> Self;
}

impl TypeConstructors for TypeExpr {
    // This is a placeholder for a type that is as of yet unknown.
    // The type inference built into the mim compiler is able to later
    // infer the types of these holes from the context they appear in.
    //
    // We could also leave the level of the type as a hole and
    // then use world.mut_hole_type() which leaves the level as a hole
    // as well and infers it later on. (not sure if we need this though)
    //
    // (hole (type (lit 0 Univ)))  --  Hole(*)
    fn hole() -> Self {
        TypeExpr {
            node: MimSlotted::Hole(AppliedId::null()),
            children: vec![TypeExpr {
                node: MimSlotted::Type(AppliedId::null()),
                children: vec![TypeExpr {
                    node: MimSlotted::Lit(AppliedId::null(), AppliedId::null()),
                    children: vec![
                        TypeExpr {
                            node: MimSlotted::Num(0),
                            children: vec![],
                        },
                        TypeExpr {
                            node: MimSlotted::Symbol("Univ".into()),
                            children: vec![],
                        },
                    ],
                }],
            }],
        }
    }

    fn nil() -> Self {
        TypeExpr {
            node: MimSlotted::Nil(),
            children: vec![],
        }
    }

    fn cons(elem: TypeExpr, next: TypeExpr) -> Self {
        TypeExpr {
            node: MimSlotted::Cons(AppliedId::null(), AppliedId::null()),
            children: vec![elem, next],
        }
    }

    fn arr(arity: TypeExpr, body: TypeExpr) -> Self {
        TypeExpr {
            node: MimSlotted::Arr(Bind {
                slot: Slot::named("dummy"),
                elem: AppliedId::null(),
            }),
            children: vec![TypeExpr {
                node: MimSlotted::Scope(AppliedId::null(), AppliedId::null()),
                children: vec![arity, body],
            }],
        }
    }

    fn sigma(elem_cons: TypeExpr) -> Self {
        TypeExpr {
            node: MimSlotted::Sigma(Bind {
                slot: Slot::named("dummy"),
                elem: AppliedId::null(),
            }),
            children: vec![TypeExpr {
                node: MimSlotted::Scope(AppliedId::null(), AppliedId::null()),
                children: vec![elem_cons, TypeExpr::nil()],
            }],
        }
    }

    fn pi(dom: TypeExpr, codom: TypeExpr) -> Self {
        TypeExpr {
            node: MimSlotted::Pi(Bind {
                slot: Slot::named("dummy"),
                elem: AppliedId::null(),
            }),
            children: vec![TypeExpr {
                node: MimSlotted::Scope(AppliedId::null(), AppliedId::null()),
                children: vec![dom, codom],
            }],
        }
    }
}

pub(crate) fn make_type(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    enode: &MimSlotted,
) -> AnalysisData {
    match enode {
        // typeof[(let $name (scope <definition> <expr>))]  = typeof(<expr>)
        MimSlotted::Let(..) => make_let_type(eg, enode),
        // typeof[(lam $x (scope <filter> <body>))]         = Pi(Hole(*), typeof(<body>))
        MimSlotted::Lam(..) => make_lam_type(eg, enode),
        // typeof[(app <callee> <arg>)]                     = typeof(<callee-codomain>)
        MimSlotted::App(..) => make_app_type(eg, enode),
        // typeof[(var $x)]                                 = Hole(*)
        MimSlotted::Var(..) => make_var_type(eg, enode),
        // typeof[(lit <val> <type>)]                       = <type>
        MimSlotted::Lit(..) => make_lit_type(eg, enode),
        // typeof[(pack <arity> <body>)]                    = Arr(<arity>, typeof(<body>))
        MimSlotted::Pack(..) => make_pack_type(eg, enode),
        // typeof[(tuple <elem-cons>)]                      = Sigma(<elem-type-cons>)
        MimSlotted::Tuple(..) => make_tuple_type(eg, enode),
        // typeof[(extract <tuple> <index>)]                = typeof(<extracted-elem>)
        MimSlotted::Extract(..) => make_extract_type(eg, enode),
        // typeof[(insert <tuple> <index> <value>)]         = typeof(<inserted-tuple>)
        MimSlotted::Insert(..) => make_insert_type(eg, enode),

        // TODO:
        // MimSlotted::Con(..) = make_con_type(eg, enode),
        // MimSlotted::Fun(..) = make_fun_type(eg, enode),
        // MimSlotted::Inj(..) = make_inj_type(eg, enode),
        // MimSlotted::Merge(..) = make_merge_type(eg, enode),

        // Num terminals and structural nodes should not get a type at all
        MimSlotted::Num(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },
        MimSlotted::MetaVar(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },
        MimSlotted::Scope(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },
        MimSlotted::Root(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },
        MimSlotted::Cons(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },
        MimSlotted::Nil(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },
        MimSlotted::TypeWrap(..) => AnalysisData {
            type_: TypeExpr::nil(),
        },

        _ => AnalysisData {
            type_: TypeExpr::hole(),
        },
    }
}

// We are making the assumption here that terms are already
// well-typed. (terms emitted from the mim compiler are already well-typed
// and we do our best to correctly type terms that are newly introduced by
// rewrite-rules) So whenever we are merging two eclasses associated with type-data,
// we assume they are equivalent representations of the same type and just
// merge the type with fewer holes and smaller term-size into the eclass.
pub(crate) fn merge_type(l: AnalysisData, r: AnalysisData) -> AnalysisData {
    let l_holes = hole_amount(&l.type_);
    let l_size = term_size(&l.type_);

    let r_holes = hole_amount(&r.type_);
    let r_size = term_size(&r.type_);

    if l_holes < r_holes {
        AnalysisData { type_: l.type_ }
    } else if l_holes > r_holes {
        AnalysisData { type_: r.type_ }
    } else if l_size < r_size {
        AnalysisData { type_: l.type_ }
    } else if l_size > r_size {
        AnalysisData { type_: r.type_ }
    } else {
        AnalysisData { type_: l.type_ }
    }
}

macro_rules! expect {
    ($value:expr, $pat:pat => $result:expr) => {
        match $value {
            $pat => $result,
            _ => panic!("Pattern mismatch"),
        }
    };
}

macro_rules! find {
    ($eg:expr, $id:expr, $pat:pat) => {{
        let found_id = $eg.find_applied_id(&$id);
        let enodes = $eg.enodes_applied(&found_id);
        let node = enodes
            .iter()
            .find(|n| matches!(n, $pat))
            .expect("Expected pattern in find")
            .clone();
        node
    }};
}

fn make_let_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let var_bind = expect!(enode, MimSlotted::Let(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, MimSlotted::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let expr_id = var_scope_childs.get(1).expect("Expected let expr id");
    let expr_type = eg.analysis_data(expr_id.id).type_.clone();

    AnalysisData { type_: expr_type }
}

fn make_lam_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let var_bind = expect!(enode, MimSlotted::Lam(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, MimSlotted::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let body_id = var_scope_childs.get(1).expect("Expected lam body id");
    let body_type = eg.analysis_data(body_id.id).type_.clone();

    AnalysisData {
        type_: TypeExpr::pi(TypeExpr::hole(), body_type),
    }
}

fn make_app_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let (callee, _arg) = expect!(enode, MimSlotted::App(callee, arg) => (callee, arg));
    let callee_type = &eg.analysis_data(callee.id).type_;

    if let TypeExpr {
        node: MimSlotted::Pi(..),
        children: pi_childs,
    } = callee_type
    {
        let pi_scope = pi_childs.first().expect("Expected pi var scope");
        let codom_type = pi_scope.children.get(1).expect("Expected pi codom");
        AnalysisData {
            type_: codom_type.clone(),
        }
    } else {
        AnalysisData {
            type_: TypeExpr::hole(),
        }
    }
}

// I guess we should always give var a hole type because all vars
// are represented with the same var eclass and therefore we can't
// associate the different variables' types with this eclass.
// So we should just hope that the mim compiler will be able to
// resolve all of these var holes itself.
fn make_var_type(
    _eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    _enode: &MimSlotted,
) -> AnalysisData {
    AnalysisData {
        type_: TypeExpr::hole(),
    }
}

fn make_lit_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let (_val, type_) = expect!(enode, MimSlotted::Lit(val, type_) => (val, type_));

    let type_id = eg.find_applied_id(type_);
    let type_ = eg.get_syn_expr(&type_id);
    AnalysisData { type_ }
}

fn make_pack_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let var_bind = expect!(enode, MimSlotted::Pack(var_bind) => var_bind);
    let var_scope = find!(eg, var_bind.elem, MimSlotted::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let arity_id = var_scope_childs.first().expect("Expected pack arity");
    let body_id = var_scope_childs.get(1).expect("Expected pack body");
    let arity = eg.get_syn_expr(arity_id);
    let body_type = eg.analysis_data(body_id.id).type_.clone();

    AnalysisData {
        type_: TypeExpr::arr(arity, body_type),
    }
}

fn make_tuple_type(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    enode: &MimSlotted,
) -> AnalysisData {
    let elem_cons = expect!(enode, MimSlotted::Tuple(elem_cons)=> elem_cons);
    let elem_cons = find!(eg, elem_cons, MimSlotted::Cons(..) | MimSlotted::Nil());

    let mut elem_types: Vec<TypeExpr> = Vec::new();
    let mut curr_cons = elem_cons;
    while let MimSlotted::Cons(elem, next) = curr_cons {
        let curr_elem_id = eg.find_applied_id(&elem);
        let curr_elem_type = eg.analysis_data(curr_elem_id.id).type_.clone();
        elem_types.push(curr_elem_type);
        curr_cons = find!(eg, next, MimSlotted::Cons(..) | MimSlotted::Nil());
    }

    if elem_types.is_empty() {
        AnalysisData {
            type_: TypeExpr::sigma(TypeExpr::nil()),
        }
    } else {
        let mut elem_type_cons = TypeExpr::nil();
        while let Some(elem_type) = elem_types.pop() {
            elem_type_cons = TypeExpr::cons(elem_type, elem_type_cons);
        }
        AnalysisData {
            type_: TypeExpr::sigma(elem_type_cons),
        }
    }
}

fn get_literal(lit_expr: &RecExpr<MimSlotted>) -> u64 {
    let lit_val = lit_expr.children.first().expect("Expected literal value");
    if let MimSlotted::Symbol(s) = lit_val.node {
        match s.as_str() {
            "ff" => 0,
            "tt" => 1,
            "i1" => 2,
            "i8" => 0x100,
            "i16" => 0x10000,
            "i32" => 0x100000000,
            _ => panic!("Unknown literal alias"),
        }
    } else if let MimSlotted::Num(n) = lit_val.node {
        n
    } else {
        panic!("Expected literal value to be a symbol or a number");
    }
}

fn cons_elem_at(cons_expr: &RecExpr<MimSlotted>, index: u64) -> RecExpr<MimSlotted> {
    let mut i = 0;
    let mut curr_cons = cons_expr;
    while let RecExpr {
        node: MimSlotted::Cons(..),
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

fn make_extract_type(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    enode: &MimSlotted,
) -> AnalysisData {
    let (tuple, index) = expect!(enode, MimSlotted::Extract(tuple, index) => (tuple, index));
    let tuple_type = &eg.analysis_data(tuple.id).type_;
    let index_id = eg.find_applied_id(index);
    let index = eg.get_syn_expr(&index_id);

    let mut extract_type = TypeExpr::hole();

    // Extract from pack
    if let TypeExpr {
        node: MimSlotted::Arr(..),
        children: arr_childs,
    } = tuple_type
    {
        let arr_var_scope = arr_childs.first().expect("Expected arr var scope");
        extract_type = arr_var_scope
            .children
            .get(1)
            .expect("Expected array body")
            .clone();
    // Extract from tuple with literal index
    } else if let TypeExpr {
        node: MimSlotted::Sigma(..),
        children: sigma_childs,
    } = tuple_type
        && let RecExpr {
            node: MimSlotted::Lit(..),
            ..
        } = index
    {
        let sigma_var_scope = sigma_childs.first().expect("Expected sigma var scope");
        let sigma_elem_cons = sigma_var_scope
            .children
            .first()
            .expect("Expected sigma elem cons");
        let index_literal = get_literal(&index);
        extract_type = cons_elem_at(sigma_elem_cons, index_literal);
    }

    AnalysisData {
        type_: extract_type,
    }
}

fn cons_insert_at(
    cons_expr: &RecExpr<MimSlotted>,
    value: &RecExpr<MimSlotted>,
    index: u64,
) -> RecExpr<MimSlotted> {
    let mut i = 0;
    let mut curr_cons = cons_expr.clone();
    let mut cursor = &mut curr_cons;

    while let RecExpr {
        node: MimSlotted::Cons(..),
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

fn make_insert_type(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    enode: &MimSlotted,
) -> AnalysisData {
    let (tuple, index, value) =
        expect!(enode, MimSlotted::Insert(tuple, index, value) => (tuple, index, value));

    let tuple_type = &eg.analysis_data(tuple.id).type_;
    let value_type = &eg.analysis_data(value.id).type_;
    let index_id = eg.find_applied_id(index);
    let index = eg.get_syn_expr(&index_id);

    let mut insert_type = TypeExpr::hole();

    // Insert into pack
    if let TypeExpr {
        node: MimSlotted::Arr(..),
        ..
    } = tuple_type
    {
        insert_type = tuple_type.clone();
    // Insert into tuple with literal index
    } else if let TypeExpr {
        node: MimSlotted::Sigma(..),
        children: sigma_childs,
    } = tuple_type
        && let RecExpr {
            node: MimSlotted::Lit(..),
            ..
        } = index
    {
        let sigma_var_scope = sigma_childs.first().expect("Expected sigma var scope");
        let sigma_elem_cons = sigma_var_scope
            .children
            .first()
            .expect("Expected sigma elem cons");
        let index_literal = get_literal(&index);
        let inserted_cons = cons_insert_at(sigma_elem_cons, value_type, index_literal);
        insert_type = TypeExpr::sigma(inserted_cons);
    }

    AnalysisData { type_: insert_type }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extract_type_info() {
        let annotated = "
        (root extern add_lit
            (@ (cn $dummy (scope (cn $dummy (scope I8 nil)) nil))
            (lam
                $return_22296
                (scope
                    (@ Bool
                    (lit ff Bool))
                    (@ (bot (type (lit 0 Univ)))
                    (app
                        (@ (cn $dummy (scope I8 nil))
                        (var $return_22296))
                        (@ I8
                        (lit 6 I8))))))))";

        let annotated: RecExpr<MimSlotted> = RecExpr::parse(annotated).unwrap();
        let typed = extract_type_annotations(&annotated);

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();
        let typed_id = add_expr_typed(&mut eg, typed);

        let enodes = eg.enodes_applied(&typed_id);
        let typed = enodes
            .first()
            .expect("Failed to find typed rec expr in egraph");

        let lam_type = eg
            .analysis_data(typed.applied_id_occurrences()[2].id)
            .type_
            .clone();

        assert_eq!(
            lam_type,
            RecExpr::parse("(cn $dummy (scope (cn $dummy (scope I8 nil)) nil))").unwrap()
        );
    }

    #[test]
    fn make_eta_expansion_hole() {
        let type_of = |eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId| {
            eg.analysis_data(id.id).type_.clone()
        };
        let type_ = |s: &str| RecExpr::<MimSlotted>::parse(s).unwrap();

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let fun_annotated = "(@ (pi $var (scope Nat Bool)) func)";
        let fun_annotated: RecExpr<MimSlotted> = RecExpr::parse(fun_annotated).unwrap();
        let fun_typed = extract_type_annotations(&fun_annotated);
        let fun_typed_id = add_expr_typed(&mut eg, fun_typed);

        let eta_exp = "(lam $x (scope (lit ff Bool) (app func (var $x))))";
        let eta_exp: RecExpr<MimSlotted> = RecExpr::parse(eta_exp).unwrap();
        let eta_exp_id = eg.add_expr(eta_exp);

        assert_eq!(
            type_of(&eg, fun_typed_id),
            type_("(pi $var (scope Nat Bool))")
        );
        assert_eq!(
            type_of(&eg, eta_exp_id),
            type_("(pi $dummy (scope (hole (type (lit 0 Univ))) Bool))")
        );
    }

    #[test]
    fn make_types_var_lit() {
        let type_of = |eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId| {
            eg.analysis_data(id.id).type_.clone()
        };
        let type_ = |s: &str| RecExpr::<MimSlotted>::parse(s).unwrap();

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let lit = "(lit 10 (idx 3))";
        let lit: RecExpr<MimSlotted> = RecExpr::parse(lit).unwrap();
        let lit_id = eg.add_expr(lit);

        assert_eq!(type_of(&eg, lit_id), type_("(idx 3)"));

        let binding = "(let $x (scope (lit tt Bool) (app (lam $y (scope (lit ff Bool) (lit 10 (idx 3)))) (var $x))))";
        let binding: RecExpr<MimSlotted> = RecExpr::parse(binding).unwrap();
        let binding_id = eg.add_expr(binding);

        assert_eq!(type_of(&eg, binding_id), type_("(idx 3)"));

        let var = "(var $foo)";
        let var: RecExpr<MimSlotted> = RecExpr::parse(var).unwrap();
        let var_id = eg.add_expr(var);

        assert_eq!(type_of(&eg, var_id), type_("(hole (type (lit 0 Univ)))"));

        let app = "(app (var $foo) (var $bar))";
        let app: RecExpr<MimSlotted> = RecExpr::parse(app).unwrap();
        let app_id = eg.add_expr(app);

        assert_eq!(type_of(&eg, app_id), type_("(hole (type (lit 0 Univ)))"));
    }

    #[test]
    fn make_types_tuple_pack() {
        let type_of = |eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId| {
            eg.analysis_data(id.id).type_.clone()
        };
        let type_ = |s: &str| RecExpr::<MimSlotted>::parse(s).unwrap();

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let tuple = "(tuple (cons (lit 1 Nat) (cons (lit 2 Nat) (cons (lit 3 Nat) nil))))";
        let tuple: RecExpr<MimSlotted> = RecExpr::parse(tuple).unwrap();
        let tuple_id = eg.add_expr(tuple);

        assert_eq!(
            type_of(&eg, tuple_id),
            type_("(sigma $dummy (scope (cons Nat (cons Nat (cons Nat nil))) nil))")
        );

        let tuple_empty = "(tuple nil)";
        let tuple_empty: RecExpr<MimSlotted> = RecExpr::parse(tuple_empty).unwrap();
        let tuple_empty_id = eg.add_expr(tuple_empty);

        assert_eq!(
            type_of(&eg, tuple_empty_id),
            type_("(sigma $dummy (scope nil nil))")
        );

        let pack = "(pack $dummy (scope (top Nat) (lit 3 Nat)))";
        let pack: RecExpr<MimSlotted> = RecExpr::parse(pack).unwrap();
        let pack_id = eg.add_expr(pack);

        assert_eq!(
            type_of(&eg, pack_id),
            type_("(arr $dummy (scope (top Nat) Nat))")
        );
    }

    #[test]
    fn make_types_extract_insert() {
        let type_of = |eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId| {
            eg.analysis_data(id.id).type_.clone()
        };
        let type_ = |s: &str| RecExpr::<MimSlotted>::parse(s).unwrap();

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let insert_tuple = "(insert (tuple (cons (lit 1 Nat) (cons (lit 2 Nat) nil))) (lit tt Bool) (lit ff Bool))";
        let insert_tuple: RecExpr<MimSlotted> = RecExpr::parse(insert_tuple).unwrap();
        let insert_tuple_id = eg.add_expr(insert_tuple);

        assert_eq!(
            type_of(&eg, insert_tuple_id),
            type_("(sigma $dummy (scope (cons Nat (cons Bool nil)) nil))")
        );

        let insert_pack =
            "(insert (pack $dummy (scope (top Nat) (lit ff Bool))) (lit tt Bool) (lit ff Bool))";
        let insert_pack: RecExpr<MimSlotted> = RecExpr::parse(insert_pack).unwrap();
        let insert_pack_id = eg.add_expr(insert_pack);

        assert_eq!(
            type_of(&eg, insert_pack_id),
            type_("(arr $dummy (scope (top Nat) Bool))")
        );

        let extract_tuple =
            "(extract (tuple (cons (lit 1 Nat) (cons (lit 3 (idx i32)) nil))) (lit tt Bool))";
        let extract_tuple: RecExpr<MimSlotted> = RecExpr::parse(extract_tuple).unwrap();
        let extract_tuple_id = eg.add_expr(extract_tuple);

        assert_eq!(type_of(&eg, extract_tuple_id), type_("(idx i32)"));

        let extract_pack =
            "(extract (pack $dummy (scope (top Nat) (lit ff Bool))) (lit 0 (idx 1)))";
        let extract_pack: RecExpr<MimSlotted> = RecExpr::parse(extract_pack).unwrap();
        let extract_pack_id = eg.add_expr(extract_pack);

        assert_eq!(type_of(&eg, extract_pack_id), type_("Bool"));

        let extract_var = "(extract (var $foo) (lit 0 (idx 1)))";
        let extract_var: RecExpr<MimSlotted> = RecExpr::parse(extract_var).unwrap();
        let extract_var_id = eg.add_expr(extract_var);

        assert_eq!(
            type_of(&eg, extract_var_id),
            type_("(hole (type (lit 0 Univ)))")
        );
    }

    #[test]
    fn make_var_type_hole() {
        let type_of = |eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId| {
            eg.analysis_data(id.id).type_.clone()
        };
        let type_ = |s: &str| RecExpr::<MimSlotted>::parse(s).unwrap();

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let var_annotated = "(@ Bool (var $foo))";
        let var_annotated: RecExpr<MimSlotted> = RecExpr::parse(var_annotated).unwrap();
        let var_typed = extract_type_annotations(&var_annotated);
        let var_typed_id = add_expr_typed(&mut eg, var_typed);

        // The annotated type for var should be overwritten with hole at this point.
        // Since all vars are represented with the same singleton var eclass, we
        // can't maintain the variables' types with an analysis and should hope that
        // the mim compiler can type-infer these var holes.
        assert_eq!(
            type_of(&eg, var_typed_id),
            type_("(hole (type (lit 0 Univ)))")
        );

        let var = "(var $bar)";
        let var: RecExpr<MimSlotted> = RecExpr::parse(var).unwrap();
        let var_id = eg.add_expr(var);

        assert_eq!(type_of(&eg, var_id), type_("(hole (type (lit 0 Univ)))"));
    }

    #[test]
    fn infer_let_type() {
        let type_of = |eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId| {
            eg.analysis_data(id.id).type_.clone()
        };
        let type_ = |s: &str| RecExpr::<MimSlotted>::parse(s).unwrap();

        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let let_annotated = "(let $foo (scope (@ Bool (lit ff Bool)) (@ Nat (lit 1 Nat))))";
        let let_annotated: RecExpr<MimSlotted> = RecExpr::parse(let_annotated).unwrap();
        let let_typed = extract_type_annotations(&let_annotated);
        let let_typed_id = add_expr_typed(&mut eg, let_typed);

        assert_eq!(type_of(&eg, let_typed_id), type_("Nat"));

        let let_var_annotated = "(let $foo (scope (@ Bool (lit ff Bool)) (@ Nat (var $bar))))";
        let let_var_annotated: RecExpr<MimSlotted> = RecExpr::parse(let_var_annotated).unwrap();
        let let_var_typed = extract_type_annotations(&let_var_annotated);
        let let_var_typed_id = add_expr_typed(&mut eg, let_var_typed);

        assert_eq!(
            type_of(&eg, let_var_typed_id),
            type_("(hole (type (lit 0 Univ)))")
        );
    }
}
