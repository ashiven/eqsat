use crate::mim_slotted::MimSlotted;
use crate::mim_slotted::analysis::{AnalysisData, MimSlottedAnalysis};
use crate::mim_slotted::util::{cons_elem_at, cons_insert_at, cons_to_vec, get_literal};
use slotted_egraphs::*;

/***********************************************************/
/* Conversion from type-annotated RecExpr to TypedRecExpr  */
/***********************************************************/

pub type TypeExpr = RecExpr<MimSlotted>;

#[derive(Debug, Clone)]
pub struct TypedRecExpr {
    pub node: MimSlotted,
    pub children: Vec<TypedRecExpr>,
    pub type_: Option<TypeExpr>,
}

pub(crate) fn remove_type_annotations(rec_expr: &RecExpr<MimSlotted>) -> RecExpr<MimSlotted> {
    if let MimSlotted::TypeWrap(..) = rec_expr.node {
        let expr = &rec_expr.children[1];
        let stripped = remove_type_annotations(expr);
        return stripped;
    }

    RecExpr::<MimSlotted> {
        node: rec_expr.node.clone(),
        children: rec_expr
            .children
            .iter()
            .map(remove_type_annotations)
            .collect(),
    }
}

pub(crate) fn extract_type_annotations(rec_expr: &RecExpr<MimSlotted>) -> TypedRecExpr {
    if let MimSlotted::TypeWrap(..) = rec_expr.node {
        let type_expr = rec_expr.children[0].clone();
        let expr = &rec_expr.children[1];
        let mut stripped = extract_type_annotations(expr);
        stripped.type_ = Some(type_expr);

        // Instead of the actual type, we give var nodes a hole type
        // to be inferred later on by the mim compiler. This is because
        // all vars are represented with the same singleton var eclass
        // and we can't store different vars' types on this single eclass.
        if let MimSlotted::Var(_slot) = expr.node {
            stripped.type_ = Some(TypeExpr::hole());
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
        type_: None,
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
    analysis_data.type_ = rec_expr.type_.clone();

    if let Some(type_) = rec_expr.type_ {
        eg.add_expr(type_);
    }

    eclass_applied_id
}

/***********************************************************/
/*  Analysis maintaining type information on eclasses      */
/***********************************************************/

pub type TypeData = TypeExpr;

pub struct TypeAnalysis;
impl TypeAnalysis {
    pub fn make(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
        make_type(eg, enode)
    }
    pub fn merge(l: AnalysisData, r: AnalysisData) -> AnalysisData {
        merge_type(l, r)
    }
}

trait TypeConstructors {
    fn hole() -> Self;
    fn nil() -> Self;
    fn type_(level: u64) -> Self;
    fn bot(type_: TypeExpr) -> Self;
    fn cons(elem: TypeExpr, next: TypeExpr) -> Self;
    fn arr(arity: TypeExpr, body: TypeExpr, var: Option<&str>) -> Self;
    fn sigma(types: Vec<TypeExpr>, var: Option<&str>) -> Self;
    fn sigma_cons(type_cons: TypeExpr, var: Option<&str>) -> Self;
    fn pi(dom: TypeExpr, codom: TypeExpr, var: Option<&str>) -> Self;
}

impl TypeConstructors for TypeExpr {
    fn hole() -> Self {
        TypeExpr {
            node: MimSlotted::Hole(AppliedId::null()),
            children: vec![TypeExpr::type_(0)],
        }
    }

    fn nil() -> Self {
        TypeExpr {
            node: MimSlotted::Nil(),
            children: vec![],
        }
    }

    fn type_(level: u64) -> Self {
        TypeExpr {
            node: MimSlotted::Type(AppliedId::null()),
            children: vec![TypeExpr {
                node: MimSlotted::Lit(AppliedId::null(), AppliedId::null()),
                children: vec![
                    TypeExpr {
                        node: MimSlotted::Num(level),
                        children: vec![],
                    },
                    TypeExpr {
                        node: MimSlotted::Symbol("Univ".into()),
                        children: vec![],
                    },
                ],
            }],
        }
    }

    fn bot(type_: TypeExpr) -> Self {
        TypeExpr {
            node: MimSlotted::Bot(AppliedId::null()),
            children: vec![type_],
        }
    }

    fn cons(elem: TypeExpr, next: TypeExpr) -> Self {
        TypeExpr {
            node: MimSlotted::Cons(AppliedId::null(), AppliedId::null()),
            children: vec![elem, next],
        }
    }

    fn arr(arity: TypeExpr, body: TypeExpr, var: Option<&str>) -> Self {
        TypeExpr {
            node: MimSlotted::Arr(Bind {
                slot: Slot::named(var.unwrap_or("dummy")),
                elem: AppliedId::null(),
            }),
            children: vec![TypeExpr {
                node: MimSlotted::Scope(AppliedId::null(), AppliedId::null()),
                children: vec![arity, body],
            }],
        }
    }

    fn sigma(mut types: Vec<TypeExpr>, var: Option<&str>) -> Self {
        let mut type_cons = TypeExpr::nil();
        while let Some(type_) = types.pop() {
            type_cons = TypeExpr::cons(type_, type_cons);
        }
        TypeExpr::sigma_cons(type_cons, var)
    }

    fn sigma_cons(type_cons: TypeExpr, var: Option<&str>) -> Self {
        TypeExpr {
            node: MimSlotted::Sigma(Bind {
                slot: Slot::named(var.unwrap_or("dummy")),
                elem: AppliedId::null(),
            }),
            children: vec![TypeExpr {
                node: MimSlotted::Scope(AppliedId::null(), AppliedId::null()),
                children: vec![type_cons, TypeExpr::nil()],
            }],
        }
    }

    fn pi(dom: TypeExpr, codom: TypeExpr, var: Option<&str>) -> Self {
        TypeExpr {
            node: MimSlotted::Pi(Bind {
                slot: Slot::named(var.unwrap_or("dummy")),
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
        // typeof[(con $x (scope <filter> <body>))]         = Pi(Hole(*), Bot(*))
        MimSlotted::Con(..) => make_con_type(eg, enode),
        // typeof[(fun $x (scope <filter> <body>))]         = Pi(Sigma(Hole(*), Pi(typeof<body>, Bot(*))), Bot(*))
        MimSlotted::Fun(..) => make_fun_type(eg, enode),
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
        // MimSlotted::Inj(..) = make_inj_type(eg, enode),
        // MimSlotted::Merge(..) = make_merge_type(eg, enode),

        // Num terminals and structural nodes should not get a type at all
        MimSlotted::Num(..) => AnalysisData { type_: None },
        MimSlotted::MetaVar(..) => AnalysisData { type_: None },
        MimSlotted::Scope(..) => AnalysisData { type_: None },
        MimSlotted::Root(..) => AnalysisData { type_: None },
        MimSlotted::Cons(..) => AnalysisData { type_: None },
        MimSlotted::Nil(..) => AnalysisData { type_: None },
        MimSlotted::TypeWrap(..) => AnalysisData { type_: None },

        _ => AnalysisData {
            type_: Some(TypeExpr::hole()),
        },
    }
}

macro_rules! child {
    ($value:expr, $idx:expr) => {
        $value.children.get($idx).expect("Failed to get child")
    };
}

macro_rules! var {
    ($value:expr) => {{
        let mut var = $value
            .node
            .all_slot_occurrences()
            .first()
            .expect("Failed to get var")
            .to_string();
        var.remove(0);
        var
    }};
}

fn hole_amount(type_expr: &TypeExpr) -> usize {
    fn holes(type_expr: &TypeExpr) -> usize {
        if let MimSlotted::Hole(..) = type_expr.node {
            1 + type_expr.children.iter().map(holes).sum::<usize>()
        } else {
            type_expr.children.iter().map(holes).sum::<usize>()
        }
    }
    holes(type_expr)
}

fn unify(l: &TypeExpr, r: &TypeExpr) -> TypeExpr {
    match (&l.node, &r.node) {
        (_, MimSlotted::Hole(_)) => l.clone(),
        (MimSlotted::Hole(_), _) => r.clone(),

        (_, MimSlotted::Bot(_)) => l.clone(),
        (MimSlotted::Bot(_), _) => r.clone(),

        (_, MimSlotted::Top(_)) => r.clone(),
        (MimSlotted::Top(_), _) => l.clone(),

        // TODO: Idx, Join, Meet, ImplicitPi
        (MimSlotted::Symbol(_), MimSlotted::Symbol(_)) => l.clone(),
        (MimSlotted::Arr(_), MimSlotted::Arr(_)) => {
            let l_scope = child!(l, 0);
            let l_arity = child!(l_scope, 0);
            let l_body = child!(l_scope, 1);

            let r_scope = child!(r, 0);
            let r_arity = child!(r_scope, 0);
            let r_body = child!(r_scope, 1);

            let arity = unify(l_arity, r_arity);
            let body = unify(l_body, r_body);

            let var = var!(l);

            TypeExpr::arr(arity, body, Some(&var))
        }
        (MimSlotted::Arr(_), MimSlotted::Sigma(_)) => {
            let l_scope = child!(l, 0);
            let l_arity = child!(l_scope, 0);
            let l_body = child!(l_scope, 1);

            let r_scope = child!(r, 0);
            let r_cons = child!(r_scope, 0);
            let r_types = cons_to_vec(r_cons);

            let body = r_types
                .iter()
                .map(|r_type| unify(l_body, r_type))
                .find(|type_| !matches!(type_.node, MimSlotted::Hole(_)))
                .unwrap_or(TypeExpr::hole());

            let var = var!(l);

            TypeExpr::arr(l_arity.clone(), body, Some(&var))
        }
        (MimSlotted::Sigma(_), MimSlotted::Arr(_)) => {
            let l_scope = child!(l, 0);
            let l_cons = child!(l_scope, 0);
            let l_types = cons_to_vec(l_cons);

            let r_scope = child!(r, 0);
            let r_arity = child!(r_scope, 0);
            let r_body = child!(r_scope, 1);

            let body = l_types
                .iter()
                .map(|l_type| unify(r_body, l_type))
                .find(|type_| !matches!(type_.node, MimSlotted::Hole(_)))
                .unwrap_or(TypeExpr::hole());

            let var = var!(l);

            TypeExpr::arr(r_arity.clone(), body, Some(&var))
        }
        (MimSlotted::Sigma(_), MimSlotted::Sigma(_)) => {
            let l_scope = child!(l, 0);
            let l_cons = child!(l_scope, 0);
            let l_types = cons_to_vec(l_cons);

            let r_scope = child!(r, 0);
            let r_cons = child!(r_scope, 0);
            let r_types = cons_to_vec(r_cons);

            let types: Vec<TypeExpr> = l_types
                .into_iter()
                .zip(r_types)
                .map(|(l_type, r_type)| unify(&l_type, &r_type))
                .collect();

            let var = var!(l);

            TypeExpr::sigma(types, Some(&var))
        }
        (MimSlotted::Pi(_), MimSlotted::Pi(_)) => {
            let l_scope = child!(l, 0);
            let l_dom = child!(l_scope, 0);
            let l_codom = child!(l_scope, 1);

            let r_scope = child!(r, 0);
            let r_dom = child!(r_scope, 0);
            let r_codom = child!(r_scope, 1);

            let dom = unify(l_dom, r_dom);
            let codom = unify(l_codom, r_codom);

            let var = var!(l);

            TypeExpr::pi(dom, codom, Some(&var))
        }

        _ => {
            if hole_amount(l) <= hole_amount(r) {
                l.clone()
            } else {
                r.clone()
            }
        }
    }
}

pub(crate) fn merge_type(l: AnalysisData, r: AnalysisData) -> AnalysisData {
    match (l.type_, r.type_) {
        (Some(l_type), None) => AnalysisData {
            type_: Some(l_type),
        },
        (None, Some(r_type)) => AnalysisData {
            type_: Some(r_type),
        },
        (Some(l_type), Some(r_type)) => AnalysisData {
            type_: Some(unify(&l_type, &r_type)),
        },
        _ => AnalysisData { type_: None },
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

#[allow(dead_code)]
fn find_apps(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    id: &AppliedId,
    lam_slot: &Slot,
    apps: &mut Vec<MimSlotted>,
) {
    let curr_enodes = eg.enodes_applied(id);
    curr_enodes.iter().for_each(|n| {
        n.applied_id_occurrences()
            .iter()
            .for_each(|id| find_apps(eg, id, lam_slot, apps))
    });

    let curr_apps: Vec<MimSlotted> = curr_enodes
        .into_iter()
        .filter(|n| {
            if matches!(n, MimSlotted::App(..)) && n.slots().contains(lam_slot) {
                let app_childs = n.applied_id_occurrences();
                let arg_id = app_childs.get(1).unwrap();
                let arg_id = eg.find_applied_id(arg_id);
                let arg_nodes = eg.enodes_applied(&arg_id);
                arg_nodes
                    .iter()
                    .any(|n| matches!(n, MimSlotted::Var(..)) && n.slots().contains(lam_slot))
            } else {
                false
            }
        })
        .collect();

    apps.extend(curr_apps);
}

#[allow(unused_variables)]
#[allow(unused_mut)]
fn infer_dom(
    eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    var_bind: &Bind<AppliedId>,
    body_id: &AppliedId,
) -> TypeExpr {
    let lam_slot = var_bind.slot;

    let mut body_apps: Vec<MimSlotted> = vec![];
    // Finds all applications in the lambda body that fulfill two conditions:
    // 1) The application takes the slot of the lambda as input (applies it either to the callee or arg)
    // 2) The applications' arg eclass contains a variable use (var $lam_slot)
    // Unfortunately, finding these apps performs expensive recursive searches through the e-graph
    // for each invokation of make_lam_type which leads to stack overflows on any more complex examples.
    // We therefore only use it during testing for now.
    #[cfg(test)]
    find_apps(eg, body_id, &lam_slot, &mut body_apps);

    // We go through all of the found applications and look for such applications
    // where the left hand side is a proper pi type from which we can infer the dom of the lambda
    for app in body_apps.iter() {
        let app_childs = app.applied_id_occurrences();
        let callee_id = app_childs.first().unwrap();
        let callee_type = &eg.analysis_data(callee_id.id).type_;
        if let Some(TypeExpr {
            node: MimSlotted::Pi(..) | MimSlotted::ImplicitPi(..),
            children,
        }) = callee_type
        {
            let pi_scope = children.first().unwrap();
            let pi_dom = pi_scope.children.first().unwrap();
            return pi_dom.clone();
        }
    }

    TypeExpr::hole()
}

fn make_lam_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let var_bind = expect!(enode, MimSlotted::Lam(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, MimSlotted::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let body_id = var_scope_childs.get(1).expect("Expected lam body id");
    let body_type = eg.analysis_data(body_id.id).type_.clone();

    let dom = infer_dom(eg, var_bind, body_id);
    let codom = body_type.unwrap_or(TypeExpr::hole());

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
    }
}

fn make_con_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let var_bind = expect!(enode, MimSlotted::Con(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, MimSlotted::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let body_id = var_scope_childs.get(1).expect("Expected lam body id");

    let dom = infer_dom(eg, var_bind, body_id);
    let codom = TypeExpr::bot(TypeExpr::type_(0));

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
    }
}

fn make_fun_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let var_bind = expect!(enode, MimSlotted::Fun(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, MimSlotted::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let body_id = var_scope_childs.get(1).expect("Expected lam body id");
    let body_type = eg.analysis_data(body_id.id).type_.clone();

    let ret_dom = body_type.unwrap_or(TypeExpr::hole());
    let ret_codom = TypeExpr::bot(TypeExpr::type_(0));
    let ret_pi = TypeExpr::pi(ret_dom, ret_codom, None);

    // TODO: Domain inference is a bit more complicated than for lam - need app to var#0
    let dom_inner = TypeExpr::hole();
    let dom = TypeExpr::sigma(vec![dom_inner, ret_pi], None);
    let codom = TypeExpr::bot(TypeExpr::type_(0));

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
    }
}

fn make_app_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let (callee, _arg) = expect!(enode, MimSlotted::App(callee, arg) => (callee, arg));
    let callee_type = &eg.analysis_data(callee.id).type_;

    match callee_type {
        Some(TypeExpr {
            node: MimSlotted::Pi(..) | MimSlotted::ImplicitPi(..),
            children,
        }) => {
            let scope = children.first().expect("Expected pi var scope");
            let codomain = scope.children.get(1).expect("Expected pi codom");
            AnalysisData {
                type_: Some(codomain.clone()),
            }
        }
        _ => AnalysisData {
            type_: Some(TypeExpr::hole()),
        },
    }
}

// We should always give var a hole type because all vars
// are represented in the same eclass and therefore we can't
// associate the different variables' types with this eclass.
fn make_var_type(
    _eg: &EGraph<MimSlotted, MimSlottedAnalysis>,
    _enode: &MimSlotted,
) -> AnalysisData {
    AnalysisData {
        type_: Some(TypeExpr::hole()),
    }
}

fn make_lit_type(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, enode: &MimSlotted) -> AnalysisData {
    let (_val, type_) = expect!(enode, MimSlotted::Lit(val, type_) => (val, type_));

    let type_id = eg.find_applied_id(type_);
    let type_ = eg.get_syn_expr(&type_id);
    AnalysisData { type_: Some(type_) }
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
        type_: Some(TypeExpr::arr(
            arity,
            body_type.unwrap_or(TypeExpr::hole()),
            None,
        )),
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
        elem_types.push(curr_elem_type.unwrap_or(TypeExpr::hole()));
        curr_cons = find!(eg, next, MimSlotted::Cons(..) | MimSlotted::Nil());
    }

    AnalysisData {
        type_: Some(TypeExpr::sigma(elem_types, None)),
    }
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

    if let Some(TypeExpr {
        node: MimSlotted::Arr(..),
        children,
    }) = tuple_type
    {
        let arr_var_scope = children.first().expect("Expected arr var scope");
        extract_type = child!(arr_var_scope, 1).clone();
    } else if let Some(TypeExpr {
        node: MimSlotted::Sigma(..),
        children,
    }) = tuple_type
        && let RecExpr {
            node: MimSlotted::Lit(..),
            ..
        } = index
    {
        let sigma_var_scope = children.first().expect("Expected sigma var scope");
        let sigma_elem_cons = child!(sigma_var_scope, 0);
        let index_literal = get_literal(&index);
        extract_type = cons_elem_at(sigma_elem_cons, index_literal);
    }

    AnalysisData {
        type_: Some(extract_type),
    }
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

    if let Some(TypeExpr {
        node: MimSlotted::Arr(..),
        ..
    }) = tuple_type
    {
        insert_type = tuple_type.clone().unwrap_or(TypeExpr::hole());
    } else if let Some(TypeExpr {
        node: MimSlotted::Sigma(..),
        children,
    }) = tuple_type
        && let RecExpr {
            node: MimSlotted::Lit(..),
            ..
        } = index
    {
        let sigma_var_scope = children.first().expect("Expected sigma var scope");
        let sigma_elem_cons = child!(sigma_var_scope, 0);
        let index_literal = get_literal(&index);
        let value_type = value_type.clone().unwrap_or(TypeExpr::hole());
        let inserted_cons = cons_insert_at(sigma_elem_cons, &value_type, index_literal);
        insert_type = TypeExpr::sigma_cons(inserted_cons, None);
    }

    AnalysisData {
        type_: Some(insert_type),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ffi::FFI;

    fn type_of(eg: &EGraph<MimSlotted, MimSlottedAnalysis>, id: AppliedId) -> Option<TypeData> {
        eg.analysis_data(id.id).type_.clone()
    }

    fn type_(s: &str) -> Option<TypeData> {
        Some(RecExpr::<MimSlotted>::parse(s).unwrap())
    }

    #[test]
    fn extract_type_info() {
        let annotated = "
        (root extern add_lit
            (@ (cn $dummy (scope (cn $dummy (scope I8 nil)) nil))
            (fun
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
        let lam_id = typed.applied_id_occurrences()[2].clone();

        assert_eq!(
            type_of(&eg, lam_id),
            type_("(cn $dummy (scope (cn $dummy (scope I8 nil)) nil))")
        );
    }

    #[test]
    fn make_eta_expansion_hole() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let fun_annotated = "(@ (pi $var (scope Nat Bool)) func)";
        let fun_annotated: RecExpr<MimSlotted> = RecExpr::parse(fun_annotated).unwrap();
        let fun_typed = extract_type_annotations(&fun_annotated);
        let fun_typed_id = add_expr_typed(&mut eg, fun_typed);

        assert_eq!(
            type_of(&eg, fun_typed_id),
            type_("(pi $var (scope Nat Bool))")
        );

        let eta_exp_lam = "(lam $x (scope (lit ff Bool) (app func (var $x))))";
        let eta_exp_lam: RecExpr<MimSlotted> = RecExpr::parse(eta_exp_lam).unwrap();
        let eta_exp_lam_id = eg.add_expr(eta_exp_lam);

        assert_eq!(
            type_of(&eg, eta_exp_lam_id),
            type_("(pi $dummy (scope Nat Bool))")
        );

        let eta_exp_con = "(con $x (scope (lit ff Bool) (app func (var $x))))";
        let eta_exp_con: RecExpr<MimSlotted> = RecExpr::parse(eta_exp_con).unwrap();
        let eta_exp_con_id = eg.add_expr(eta_exp_con);

        assert_eq!(
            type_of(&eg, eta_exp_con_id),
            type_("(pi $dummy (scope Nat (bot (type (lit 0 Univ)))))")
        );

        let eta_exp_fun = "(fun $x (scope (lit ff Bool) (app func (var $x))))";
        let eta_exp_fun: RecExpr<MimSlotted> = RecExpr::parse(eta_exp_fun).unwrap();
        let eta_exp_fun_id = eg.add_expr(eta_exp_fun);

        assert_eq!(
            type_of(&eg, eta_exp_fun_id),
            type_(
                "(pi $dummy (scope (sigma $dummy (scope (cons (hole (type (lit 0 Univ))) (cons (pi $dummy (scope Bool (bot (type (lit 0 Univ))))) nil)) nil)) (bot (type (lit 0 Univ)))))"
            )
        );
    }

    #[test]
    fn make_types_var_lit() {
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

    #[test]
    fn implicit_pi_callee() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let f_annotated = "(@ (pi* $dummy (scope Nat (pi $dummy (scope Nat Nat)))) f)";
        let f_annotated: RecExpr<MimSlotted> = RecExpr::parse(f_annotated).unwrap();
        let f_typed = extract_type_annotations(&f_annotated);
        let f_typed_id = add_expr_typed(&mut eg, f_typed);

        assert_eq!(
            type_of(&eg, f_typed_id),
            type_("(pi* $dummy (scope Nat (pi $dummy (scope Nat Nat))))")
        );

        let implicit_app = "(app f (lit 1 Nat))";
        let implicit_app: RecExpr<MimSlotted> = RecExpr::parse(implicit_app).unwrap();
        let implicit_app_id = eg.add_expr(implicit_app);
        assert_eq!(
            type_of(&eg, implicit_app_id),
            type_("(pi $dummy (scope Nat Nat))")
        );
    }

    #[test]
    fn union_hole_pis() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let f_annotated = "(@ (pi $dummy (scope (hole (type (lit 0 Univ))) Nat)) f)";
        let f_annotated: RecExpr<MimSlotted> = RecExpr::parse(f_annotated).unwrap();
        let f_typed = extract_type_annotations(&f_annotated);
        let f_typed_id = add_expr_typed(&mut eg, f_typed);

        assert_eq!(
            type_of(&eg, f_typed_id.clone()),
            type_("(pi $dummy (scope (hole (type (lit 0 Univ))) Nat))")
        );

        let g_annotated = "(@ (pi $dummy (scope Nat (hole (type (lit 0 Univ))))) g)";
        let g_annotated: RecExpr<MimSlotted> = RecExpr::parse(g_annotated).unwrap();
        let g_typed = extract_type_annotations(&g_annotated);
        let g_typed_id = add_expr_typed(&mut eg, g_typed);

        eg.union(&f_typed_id, &g_typed_id);

        let f_typed_id = eg.find_applied_id(&f_typed_id);
        let g_typed_id = eg.find_applied_id(&g_typed_id);

        assert_eq!(
            type_of(&eg, f_typed_id),
            type_("(pi $dummy (scope Nat Nat))")
        );
        assert_eq!(
            type_of(&eg, g_typed_id),
            type_("(pi $dummy (scope Nat Nat))")
        );
    }

    #[test]
    fn union_sigma_with_vars() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let a = "(@ (sigma $foo (scope (cons (hole (type (lit 0 Univ))) (cons (extract (var $foo) (lit ff Bool)) nil)) nil)) a)";
        let a: RecExpr<MimSlotted> = RecExpr::parse(a).unwrap();
        let a = extract_type_annotations(&a);
        let a_id = add_expr_typed(&mut eg, a);

        // TODO: Lets assume that the let node from which sigma derives its reference to $bar, gets
        // rewritten in the e-graph and gets a new slot $f1, we then need to have this change be reflected
        // in this type, living on eclasses as analysis data, as well. The way to do this would be
        // to add all types to the egraph (maybe as part of extract_type_annotations), each one with
        // a new root. (This way we ensure that the slots in types - in this case, $bar remain updated).
        // Then when it comes to converting the final rec exprs to ffi, we not only look up the node to get
        // the type from the analysis data, we also then look up the type that we got by converting it
        // to a pattern via re_to_pattern and then e-matching it in the e-graph to get the
        // corresponding type with the correct updated slots.
        //
        // - Remember that $dummy can create problems when e-matching so keep that in mind
        // - In unify we need to add new type expressions to the egraph if we actually end up
        //   creating a new type (for this we need some global egraph var similar to rulesets)
        // - Before creating a new expr, we need to actually e-match the terms l and r containing
        //   $bar to get the most up to date version of $bar and only then add the unification of
        //   that to the egraph
        // - Also, we might not need a global egraph and can instead use Analysis::modify
        // - On another note, it might be easier to just store the initial applied ids of types
        //   as analysis data instead of their entire syntactic representations (Would that even work?)

        let b = "(let $bar (scope (tuple nil) (@ (sigma $foo (scope (cons (extract (var $bar) (lit ff Bool)) (cons (hole (type (lit 0 Univ))) nil)) nil)) b)))";
        let b: RecExpr<MimSlotted> = RecExpr::parse(b).unwrap();
        let b = extract_type_annotations(&b);
        let b_id = add_expr_typed(&mut eg, b);

        eg.union(&a_id, &b_id);

        let a_id = eg.find_applied_id(&a_id);
        let b_id = eg.find_applied_id(&b_id);

        assert_eq!(
            type_of(&eg, a_id),
            type_(
                "(sigma $foo (scope (cons (extract (var $bar) (lit ff Bool)) (cons (extract (var $foo) (lit ff Bool)) nil)) nil))"
            )
        );
        assert_eq!(
            type_of(&eg, b_id),
            type_(
                "(sigma $foo (scope (cons (extract (var $bar) (lit ff Bool)) (cons (extract (var $foo) (lit ff Bool)) nil)) nil))"
            )
        );
    }

    #[test]
    fn union_arr_with_vars() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let a = "(@ (arr $foo (scope (hole (type (lit 0 Univ))) (extract (var $foo) (lit ff Bool)))) a)";
        let a: RecExpr<MimSlotted> = RecExpr::parse(a).unwrap();
        let a = extract_type_annotations(&a);
        let a_id = add_expr_typed(&mut eg, a);

        let b = "(let $bar (scope (tuple nil) (@ (arr $foo (scope (extract (var $bar) (lit ff Bool)) (hole (type (lit 0 Univ))))) b)))";
        let b: RecExpr<MimSlotted> = RecExpr::parse(b).unwrap();
        let b = extract_type_annotations(&b);
        let b_id = add_expr_typed(&mut eg, b);

        eg.union(&a_id, &b_id);

        let a_id = eg.find_applied_id(&a_id);
        let b_id = eg.find_applied_id(&b_id);

        assert_eq!(
            type_of(&eg, a_id),
            type_(
                "(arr $foo (scope (extract (var $bar) (lit ff Bool)) (extract (var $foo) (lit ff Bool))))"
            )
        );
        assert_eq!(
            type_of(&eg, b_id),
            type_(
                "(arr $foo (scope (extract (var $bar) (lit ff Bool)) (extract (var $foo) (lit ff Bool))))"
            )
        );
    }

    #[test]
    fn type_depending_on_outer_slot() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let b = "(let $bar (scope
                            (lit ff Bool)
                            (@ (arr $foo (scope (extract (var $bar) (lit ff Bool)) (extract (var $foo) (lit ff Bool))))
                            b)))";
        let b: RecExpr<MimSlotted> = RecExpr::parse(b).unwrap();
        let b = extract_type_annotations(&b);
        let b_id = add_expr_typed(&mut eg, b);

        assert_eq!(
            type_of(&eg, b_id.clone()),
            type_(
                "(arr $foo (scope (extract (var $bar) (lit ff Bool)) (extract (var $foo) (lit ff Bool))))"
            )
        );

        let lam_rewrite: Rewrite<MimSlotted, MimSlottedAnalysis> = rw!("lam-rewrite";
            "(let $bar (scope (lit ff Bool) ?e))"
            => "(let $baz (scope Nat ?e))" );

        let mut runner = Runner::<MimSlotted, MimSlottedAnalysis>::default().with_egraph(eg);
        runner.run(&[lam_rewrite]);

        let extractor = Extractor::new(&runner.egraph, AstSize);
        let b = extractor.extract(&b_id, &runner.egraph);

        assert_eq!(format!("{}", b), "(let $f13 (scope Nat b))",);

        let b_ffi = b.to_ffi(Some(&runner.egraph));
        let _b_ffi_type = &b_ffi.nodes.last().unwrap().type_;

        // TODO: This should work but doesn't - see TODO in ffi.rs line 353
        // assert_eq!(
        //     format!("{}", b_ffi_type.pretty(80)),
        //     "(arr\n  $foo\n  (scope (extract (var $f13) (lit ff Bool)) (extract (var $foo) (lit ff Bool))))"
        // );

        let b_id = runner.egraph.find_applied_id(&b_id);
        assert_eq!(
            type_of(&runner.egraph, b_id),
            type_(
                "(arr $foo (scope (extract (var $bar) (lit ff Bool)) (extract (var $foo) (lit ff Bool))))"
            )
        );
    }

    #[test]
    fn infer_lam_domain() {
        let mut eg = EGraph::<MimSlotted, MimSlottedAnalysis>::default();

        let f = "(@ (pi $dummy (scope Nat Bool)) f)";
        let f = RecExpr::<MimSlotted>::parse(f).unwrap();
        let f = extract_type_annotations(&f);
        let f_id = add_expr_typed(&mut eg, f);

        assert_eq!(type_of(&eg, f_id), type_("(pi $dummy (scope Nat Bool))"));

        let g = "(@ (pi $dummy (scope Bool Nat)) g)";
        let g = RecExpr::<MimSlotted>::parse(g).unwrap();
        let g = extract_type_annotations(&g);
        let g_id = add_expr_typed(&mut eg, g);

        assert_eq!(type_of(&eg, g_id), type_("(pi $dummy (scope Bool Nat))"));

        let lam = "(lam $x (scope
                            (lit ff Bool)
                            (lam $y (scope
                                (lit ff Bool)
                                (tuple
                                    (cons
                                        (app g (var $y))
                                    (cons
                                        (app f (var $x))
                                    nil))))))) ";

        let lam = RecExpr::<MimSlotted>::parse(lam).unwrap();
        let lam_id = eg.add_expr(lam);

        assert_eq!(
            type_of(&eg, lam_id),
            type_(
                "(pi $dummy (scope
                    Nat
                    (pi $dummy (scope
                        Bool
                        (sigma $dummy (scope (cons Nat (cons Bool nil)) nil))))))"
            )
        );
    }
}
