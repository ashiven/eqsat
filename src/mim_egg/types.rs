use crate::mim_egg::Mim;
use crate::mim_egg::analysis::{AnalysisData, MimAnalysis};
use crate::mim_egg::util::get_literal;
use egg::*;

/***********************************************************/
/* Conversion from type-annotated RecExpr to TypedRecExpr  */
/***********************************************************/

pub type TypeExpr = RecExpr<Mim>;

#[derive(Debug, Clone)]
pub struct TypedRecExpr {
    pub node: Mim,
    pub children: Vec<TypedRecExpr>,
    pub type_: Option<TypeExpr>,
}

pub(crate) fn remove_type_annotations(rec_expr: &RecExpr<Mim>) -> RecExpr<Mim> {
    let mut out = RecExpr::<Mim>::default();
    let mut remap = vec![Id::from(0); rec_expr.len()];

    for (i, node) in rec_expr.iter().enumerate() {
        match node {
            Mim::TypeWrap([_, inner]) => {
                remap[i] = remap[usize::from(*inner)];
            }
            _ => {
                let mut new_node = node.clone();

                new_node.update_children(|id| remap[usize::from(id)]);

                let new_id = out.add(new_node);
                remap[i] = new_id;
            }
        }
    }

    out
}

pub(crate) fn extract_type_annotations(rec_expr: &RecExpr<Mim>) -> TypedRecExpr {
    let root = rec_expr.root();
    extract_types(rec_expr, root)
}

fn extract_types(rec_expr: &RecExpr<Mim>, id: Id) -> TypedRecExpr {
    let node = &rec_expr[id];

    match node {
        Mim::TypeWrap([type_id, expr]) => {
            let mut type_ = RecExpr::<Mim>::default();
            build_type_expr(rec_expr, type_id, &mut type_);

            let mut stripped = extract_types(rec_expr, *expr);
            stripped.type_ = Some(type_);

            stripped
        }

        _ => {
            let children = node
                .children()
                .iter()
                .map(|id| extract_types(rec_expr, *id))
                .collect();

            let mut res = TypedRecExpr {
                node: node.clone(),
                children,
                type_: None,
            };

            if matches!(node, Mim::Let(..)) {
                let expr = &res.children[2];
                res.type_ = expr.type_.clone();
            }

            res
        }
    }
}

fn build_type_expr(rec_expr: &RecExpr<Mim>, id: &Id, type_expr: &mut RecExpr<Mim>) -> Id {
    let mut node = rec_expr[*id].clone();
    node.update_children(|c| build_type_expr(rec_expr, &c, type_expr));
    type_expr.add(node)
}

pub(crate) fn add_expr_typed(eg: &mut EGraph<Mim, MimAnalysis>, rec_expr: TypedRecExpr) -> Id {
    let mut node = rec_expr.node;
    let child_ids = node.children_mut();

    for (i, child) in rec_expr.children.into_iter().enumerate() {
        child_ids[i] = add_expr_typed(eg, child);
    }

    let eclass_id = eg.add(node);

    let analysis_data = &mut eg[eclass_id].data;
    analysis_data.type_ = rec_expr.type_.clone();

    if let Some(type_) = rec_expr.type_ {
        eg.add_expr(&type_);
    }

    eclass_id
}

/***********************************************************/
/*  Analysis maintaining type information on eclasses      */
/***********************************************************/

pub type TypeData = TypeExpr;

pub struct TypeAnalysis;
impl TypeAnalysis {
    pub fn make(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
        make_type(eg, enode)
    }
    pub fn merge(l: &mut AnalysisData, r: AnalysisData) -> DidMerge {
        merge_type(l, r)
    }
}

trait TypeConstructors {
    fn hole() -> Self;
    fn type_(level: u64) -> Self;
    fn bot(type_: TypeExpr) -> Self;
    fn arr(arity: TypeExpr, body: TypeExpr, var: Option<&str>) -> Self;
    fn sigma(types: Vec<TypeExpr>, var: Option<&str>) -> Self;
    fn pi(dom: TypeExpr, codom: TypeExpr, var: Option<&str>) -> Self;
}

const MAX_LINE: usize = 80;

impl TypeConstructors for TypeExpr {
    fn hole() -> Self {
        "(hole (type (lit 0 Univ)))".parse().unwrap()
    }

    fn type_(level: u64) -> Self {
        format!("(type (lit {} Univ))", level).parse().unwrap()
    }

    fn bot(type_: TypeExpr) -> Self {
        format!("(bot {})", type_.pretty(MAX_LINE)).parse().unwrap()
    }

    fn arr(arity: TypeExpr, body: TypeExpr, var: Option<&str>) -> Self {
        format!(
            "(arr {} {} {})",
            var.unwrap_or("dummy"),
            arity.pretty(MAX_LINE),
            body.pretty(MAX_LINE)
        )
        .parse()
        .unwrap()
    }

    fn sigma(types: Vec<TypeExpr>, var: Option<&str>) -> Self {
        let mut sigma = String::from(format!("(sigma {}", var.unwrap_or("dummy")));
        for type_ in types {
            sigma.push_str(format!(" {}", type_.pretty(MAX_LINE)).as_str());
        }
        sigma.push_str(")");
        sigma.parse().unwrap()
    }

    fn pi(dom: TypeExpr, codom: TypeExpr, var: Option<&str>) -> Self {
        format!(
            "(pi {} {} {})",
            var.unwrap_or("dummy"),
            dom.pretty(MAX_LINE),
            codom.pretty(MAX_LINE),
        )
        .parse()
        .unwrap()
    }
}

pub(crate) fn make_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    match enode {
        // typeof[(let $name (scope <definition> <expr>))]  = typeof(<expr>)
        Mim::Let(..) => make_let_type(eg, enode),
        // typeof[(lam $x (scope <filter> <body>))]         = Pi(Hole(*), typeof(<body>))
        Mim::Lam(..) => make_lam_type(eg, enode),
        // typeof[(con $x (scope <filter> <body>))]         = Pi(Hole(*), Bot(*))
        Mim::Con(..) => make_con_type(eg, enode),
        // typeof[(fun $x (scope <filter> <body>))]         = Pi(Sigma(Hole(*), Pi(typeof<body>, Bot(*))), Bot(*))
        Mim::Fun(..) => make_fun_type(eg, enode),
        // typeof[(app <callee> <arg>)]                     = typeof(<callee-codomain>)
        Mim::App(..) => make_app_type(eg, enode),
        // typeof[(var $x)]                                 = Hole(*)
        Mim::Var(..) => make_var_type(eg, enode),
        // typeof[(lit <val> <type>)]                       = <type>
        Mim::Lit(..) => make_lit_type(eg, enode),
        // typeof[(pack <arity> <body>)]                    = Arr(<arity>, typeof(<body>))
        Mim::Pack(..) => make_pack_type(eg, enode),
        // typeof[(tuple <elem-cons>)]                      = Sigma(<elem-type-cons>)
        Mim::Tuple(..) => make_tuple_type(eg, enode),
        // typeof[(extract <tuple> <index>)]                = typeof(<extracted-elem>)
        Mim::Extract(..) => make_extract_type(eg, enode),
        // typeof[(insert <tuple> <index> <value>)]         = typeof(<inserted-tuple>)
        Mim::Insert(..) => make_insert_type(eg, enode),

        // TODO:
        // Mim::Inj(..) = make_inj_type(eg, enode),
        // Mim::Merge(..) = make_merge_type(eg, enode),

        // Num terminals and structural nodes should not get a type at all
        Mim::Num(..) => AnalysisData {
            type_: None,
            ..Default::default()
        },
        Mim::Root(..) => AnalysisData {
            type_: None,
            ..Default::default()
        },
        Mim::TypeWrap(..) => AnalysisData {
            type_: None,
            ..Default::default()
        },

        _ => AnalysisData {
            type_: Some(TypeExpr::hole()),
            ..Default::default()
        },
    }
}

macro_rules! child {
    ($type_expr:expr, $idx:expr) => {{
        let child_id = $type_expr[$type_expr.root()].children()[$idx];
        let mut child_type_expr = TypeExpr::default();
        build_type_expr(&$type_expr, &child_id, &mut child_type_expr);
        child_type_expr
    }};
}

macro_rules! childs {
    ($type_expr:expr, $idx:expr) => {{
        let mut res: Vec<TypeExpr> = vec![];
        for (i, child_id) in $type_expr[$type_expr.root()].children().iter().enumerate() {
            if i >= $idx {
                let mut child_type_expr = TypeExpr::default();
                build_type_expr(&$type_expr, &child_id, &mut child_type_expr);
                res.push(child_type_expr);
            }
        }
        res
    }};
}

macro_rules! var {
    ($type_expr:expr) => {{
        let var_id = $type_expr[$type_expr.root()].children()[0];
        let var = &$type_expr[var_id];
        if let Mim::Symbol(s) = var {
            s
        } else {
            panic!("Expected var symbol");
        }
    }};
}

fn hole_amount(type_expr: &TypeExpr) -> usize {
    let mut holes = 0;
    for node in type_expr {
        if let Mim::Hole(_) = node {
            holes += 1;
        }
    }
    holes
}

fn unify(l: &TypeExpr, r: &TypeExpr) -> TypeExpr {
    match (&l[l.root()], &r[r.root()]) {
        (_, Mim::Hole(_)) => l.clone(),
        (Mim::Hole(_), _) => r.clone(),

        (_, Mim::Bot(_)) => l.clone(),
        (Mim::Bot(_), _) => r.clone(),

        (_, Mim::Top(_)) => r.clone(),
        (Mim::Top(_), _) => l.clone(),

        // TODO: Idx, Join, Meet, ImplicitPi
        (Mim::Symbol(_), Mim::Symbol(_)) => l.clone(),
        (Mim::Arr(_), Mim::Arr(_)) => {
            let l_arity = child!(l, 1);
            let l_body = child!(l, 2);

            let r_arity = child!(r, 1);
            let r_body = child!(r, 2);

            let arity = unify(&l_arity, &r_arity);
            let body = unify(&l_body, &r_body);

            let var = var!(l);

            TypeExpr::arr(arity, body, Some(&var))
        }
        (Mim::Arr(_), Mim::Sigma(_)) => {
            let l_arity = child!(l, 1);
            let l_body = child!(l, 2);

            let r_types = childs!(r, 1);

            let body = r_types
                .iter()
                .map(|r_type| unify(&l_body, r_type))
                .find(|type_| !matches!(type_[type_.root()], Mim::Hole(_)))
                .unwrap_or(TypeExpr::hole());

            let var = var!(l);

            TypeExpr::arr(l_arity.clone(), body, Some(&var))
        }
        (Mim::Sigma(_), Mim::Arr(_)) => {
            let l_types = childs!(l, 1);

            let r_arity = child!(r, 1);
            let r_body = child!(r, 2);

            let body = l_types
                .iter()
                .map(|l_type| unify(&r_body, l_type))
                .find(|type_| !matches!(type_[type_.root()], Mim::Hole(_)))
                .unwrap_or(TypeExpr::hole());

            let var = var!(l);

            TypeExpr::arr(r_arity.clone(), body, Some(&var))
        }
        (Mim::Sigma(_), Mim::Sigma(_)) => {
            let l_types = childs!(l, 1);
            let r_types = childs!(r, 1);

            let types: Vec<TypeExpr> = l_types
                .iter()
                .zip(r_types)
                .map(|(l_type, r_type)| unify(&l_type, &r_type))
                .collect();

            let var = var!(l);

            TypeExpr::sigma(types, Some(&var))
        }
        (Mim::Pi(_), Mim::Pi(_)) | (Mim::ImplicitPi(_), Mim::ImplicitPi(_)) => {
            let l_dom = child!(l, 1);
            let l_codom = child!(l, 2);

            let r_dom = child!(r, 1);
            let r_codom = child!(r, 2);

            let dom = unify(&l_dom, &r_dom);
            let codom = unify(&l_codom, &r_codom);

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

pub(crate) fn merge_type(l: &mut AnalysisData, r: AnalysisData) -> DidMerge {
    match (&l.type_, r.type_) {
        (Some(_), None) => DidMerge(false, false),
        (None, Some(r_type)) => {
            l.type_ = Some(r_type);
            DidMerge(true, true)
        }
        (Some(l_type), Some(r_type)) => {
            l.type_ = Some(unify(&l_type, &r_type));
            DidMerge(true, true)
        }
        _ => DidMerge(false, false),
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
/*

fn make_let_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let var_bind = expect!(enode, Mim::Let(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, Mim::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let expr_id = var_scope_childs.get(1).expect("Expected let expr id");
    let expr_type = eg.analysis_data(expr_id.id).type_.clone();

    AnalysisData { type_: expr_type }
}

#[allow(dead_code)]
fn find_apps(eg: &EGraph<Mim, MimAnalysis>, id: &AppliedId, lam_slot: &Slot, apps: &mut Vec<Mim>) {
    let curr_enodes = eg.enodes_applied(id);
    curr_enodes.iter().for_each(|n| {
        n.applied_id_occurrences()
            .iter()
            .for_each(|id| find_apps(eg, id, lam_slot, apps))
    });

    let curr_apps: Vec<Mim> = curr_enodes
        .into_iter()
        .filter(|n| {
            if matches!(n, Mim::App(..)) && n.slots().contains(lam_slot) {
                let app_childs = n.applied_id_occurrences();
                let arg_id = app_childs.get(1).unwrap();
                let arg_id = eg.find_applied_id(arg_id);
                let arg_nodes = eg.enodes_applied(&arg_id);
                arg_nodes
                    .iter()
                    .any(|n| matches!(n, Mim::Var(..)) && n.slots().contains(lam_slot))
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
    eg: &EGraph<Mim, MimAnalysis>,
    var_bind: &Bind<AppliedId>,
    body_id: &AppliedId,
) -> TypeExpr {
    let lam_slot = var_bind.slot;

    let mut body_apps: Vec<Mim> = vec![];
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
            node: Mim::Pi(..) | Mim::ImplicitPi(..),
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

fn make_lam_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let var_bind = expect!(enode, Mim::Lam(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, Mim::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let body_id = var_scope_childs.get(1).expect("Expected lam body id");
    let body_type = eg.analysis_data(body_id.id).type_.clone();

    let dom = infer_dom(eg, var_bind, body_id);
    let codom = body_type.unwrap_or(TypeExpr::hole());

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
    }
}

fn make_con_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let var_bind = expect!(enode, Mim::Con(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, Mim::Scope(..));
    let var_scope_childs = var_scope.applied_id_occurrences();

    let body_id = var_scope_childs.get(1).expect("Expected lam body id");

    let dom = infer_dom(eg, var_bind, body_id);
    let codom = TypeExpr::bot(TypeExpr::type_(0));

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
    }
}

fn make_fun_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let var_bind = expect!(enode, Mim::Fun(var_bind) => var_bind );
    let var_scope = find!(eg, var_bind.elem, Mim::Scope(..));
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

fn make_app_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let (callee, _arg) = expect!(enode, Mim::App(callee, arg) => (callee, arg));
    let callee_type = &eg.analysis_data(callee.id).type_;

    match callee_type {
        Some(TypeExpr {
            node: Mim::Pi(..) | Mim::ImplicitPi(..),
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
fn make_var_type(_eg: &EGraph<Mim, MimAnalysis>, _enode: &Mim) -> AnalysisData {
    AnalysisData {
        type_: Some(TypeExpr::hole()),
    }
}

fn make_lit_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let (_val, type_) = expect!(enode, Mim::Lit(val, type_) => (val, type_));

    let type_id = eg.find_applied_id(type_);
    let type_ = eg.get_syn_expr(&type_id);
    AnalysisData { type_: Some(type_) }
}

fn make_pack_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let var_bind = expect!(enode, Mim::Pack(var_bind) => var_bind);
    let var_scope = find!(eg, var_bind.elem, Mim::Scope(..));
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

fn make_tuple_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let elem_cons = expect!(enode, Mim::Tuple(elem_cons)=> elem_cons);
    let elem_cons = find!(eg, elem_cons, Mim::Cons(..) | Mim::Nil());

    let mut elem_types: Vec<TypeExpr> = Vec::new();
    let mut curr_cons = elem_cons;
    while let Mim::Cons(elem, next) = curr_cons {
        let curr_elem_id = eg.find_applied_id(&elem);
        let curr_elem_type = eg.analysis_data(curr_elem_id.id).type_.clone();
        elem_types.push(curr_elem_type.unwrap_or(TypeExpr::hole()));
        curr_cons = find!(eg, next, Mim::Cons(..) | Mim::Nil());
    }

    AnalysisData {
        type_: Some(TypeExpr::sigma(elem_types, None)),
    }
}

fn make_extract_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let (tuple, index) = expect!(enode, Mim::Extract(tuple, index) => (tuple, index));
    let tuple_type = &eg.analysis_data(tuple.id).type_;
    let index_id = eg.find_applied_id(index);
    let index = eg.get_syn_expr(&index_id);

    let mut extract_type = TypeExpr::hole();

    if let Some(TypeExpr {
        node: Mim::Arr(..),
        children,
    }) = tuple_type
    {
        let arr_var_scope = children.first().expect("Expected arr var scope");
        extract_type = child!(arr_var_scope, 1).clone();
    } else if let Some(TypeExpr {
        node: Mim::Sigma(..),
        children,
    }) = tuple_type
        && let RecExpr {
            node: Mim::Lit(..), ..
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

fn make_insert_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let (tuple, index, value) =
        expect!(enode, Mim::Insert(tuple, index, value) => (tuple, index, value));
    let tuple_type = &eg.analysis_data(tuple.id).type_;
    let value_type = &eg.analysis_data(value.id).type_;
    let index_id = eg.find_applied_id(index);
    let index = eg.get_syn_expr(&index_id);

    let mut insert_type = TypeExpr::hole();

    if let Some(TypeExpr {
        node: Mim::Arr(..), ..
    }) = tuple_type
    {
        insert_type = tuple_type.clone().unwrap_or(TypeExpr::hole());
    } else if let Some(TypeExpr {
        node: Mim::Sigma(..),
        children,
    }) = tuple_type
        && let RecExpr {
            node: Mim::Lit(..), ..
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
*/

#[cfg(test)]
mod test {
    use super::*;

    fn type_of(eg: &EGraph<Mim, MimAnalysis>, id: &Id) -> Option<TypeData> {
        eg[*id].data.type_.clone()
    }

    fn type_(s: &str) -> Option<TypeData> {
        Some(s.parse().unwrap())
    }

    #[test]
    fn extract_type_info() {
        let annotated = "
        (root extern add_lit
            (@ (cn dummy (cn dummy I8 nil) nil)
            (fun
                return_22296
                    (@ Bool
                    (lit ff Bool))
                    (@ (bot (type (lit 0 Univ)))
                    (app
                        (@ (cn dummy I8 nil)
                        return_22296)
                        (@ I8
                        (lit 6 I8)))))))";

        let annotated: RecExpr<Mim> = annotated.parse().unwrap();
        let untyped = remove_type_annotations(&annotated);

        assert_eq!(
            untyped.pretty(80),
            "(root\n  extern\n  add_lit\n  (fun return_22296 (lit ff Bool) (app return_22296 (lit 6 I8))))"
        );

        let typed = extract_type_annotations(&annotated);

        let mut eg = EGraph::<Mim, MimAnalysis>::default();
        let typed_id = add_expr_typed(&mut eg, typed);

        let enodes = &eg[typed_id].nodes;
        let typed = enodes
            .first()
            .expect("Failed to find typed rec expr in egraph");
        let lam_id = typed.children()[2];

        assert_eq!(
            type_of(&eg, &lam_id),
            type_("(cn dummy (cn dummy I8 nil) nil)")
        );
    }
}
