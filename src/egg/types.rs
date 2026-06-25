use crate::egg::Mim;
use crate::egg::analysis::{AnalysisData, MimAnalysis};
use crate::egg::util::get_literal;
use egg::*;

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
        let mut sigma = format!("(sigma {}", var.unwrap_or("dummy"));
        for type_ in types {
            sigma.push_str(format!(" {}", type_.pretty(MAX_LINE)).as_str());
        }
        sigma.push(')');
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

        // TODO: Idx, Join, Meet
        (Mim::Symbol(_), Mim::Symbol(_)) => l.clone(),
        (Mim::Arr(_), Mim::Arr(_)) => {
            let l_arity = child!(l, 1);
            let l_body = child!(l, 2);

            let r_arity = child!(r, 1);
            let r_body = child!(r, 2);

            let arity = unify(&l_arity, &r_arity);
            let body = unify(&l_body, &r_body);

            let var = var!(l);

            TypeExpr::arr(arity, body, Some(var))
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

            TypeExpr::arr(l_arity, body, Some(var))
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

            TypeExpr::arr(r_arity, body, Some(var))
        }
        (Mim::Sigma(_), Mim::Sigma(_)) => {
            let l_types = childs!(l, 1);
            let r_types = childs!(r, 1);

            let types: Vec<TypeExpr> = l_types
                .iter()
                .zip(r_types)
                .map(|(l_type, r_type)| unify(l_type, &r_type))
                .collect();

            let var = var!(l);

            TypeExpr::sigma(types, Some(var))
        }
        (Mim::Pi(_), Mim::Pi(_)) | (Mim::ImplicitPi(_), Mim::ImplicitPi(_)) => {
            let l_dom = child!(l, 1);
            let l_codom = child!(l, 2);

            let r_dom = child!(r, 1);
            let r_codom = child!(r, 2);

            let dom = unify(&l_dom, &r_dom);
            let codom = unify(&l_codom, &r_codom);

            let var = var!(l);

            TypeExpr::pi(dom, codom, Some(var))
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
        (Some(_), None) => DidMerge(false, true),
        (None, Some(r_type)) => {
            l.type_ = Some(r_type);
            DidMerge(true, false)
        }
        (Some(l_type), Some(r_type)) => {
            let unified = unify(l_type, &r_type);

            let l_changed = *l_type != unified;
            let r_changed = r_type != unified;

            if l_changed {
                l.type_ = Some(unified);
            }

            DidMerge(l_changed, r_changed)
        }
        _ => DidMerge(false, false),
    }
}

#[macro_export]
macro_rules! expect {
    ($value:expr, $pat:pat => $result:expr) => {
        match $value {
            $pat => $result,
            _ => panic!("Pattern mismatch"),
        }
    };
}

#[allow(unused_macros)]
macro_rules! find {
    ($eg:expr, $id:expr, $pat:pat) => {{
        let enodes = $eg[$id].nodes;
        let node = enodes
            .iter()
            .find(|n| matches!(n, $pat))
            .expect("Expected pattern in find")
            .clone();
        node
    }};
}

fn make_let_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let [_var, _def, expr] = expect!(enode, Mim::Let([var, def, expr]) => [var,def,expr] );
    let expr_type = eg[*expr].data.type_.clone();

    AnalysisData {
        type_: expr_type,
        ..Default::default()
    }
}

#[allow(dead_code)]
fn find_apps(eg: &EGraph<Mim, MimAnalysis>, id: &Id, lam_var: &str, apps: &mut Vec<Mim>) {
    let curr_enodes = &eg[*id].nodes;
    curr_enodes.iter().for_each(|n| {
        n.children()
            .iter()
            .for_each(|id| find_apps(eg, id, lam_var, apps))
    });

    let curr_apps: Vec<Mim> = curr_enodes
        .clone()
        .into_iter()
        .filter(|n| {
            if matches!(n, Mim::App(..)) {
                let app_childs = n.children();
                let arg_id = app_childs.get(1).unwrap();
                let arg_nodes = &eg[*arg_id].nodes;
                arg_nodes.iter().any(|n| {
                    if let Mim::Var(v) = n
                        && let Some(Mim::Symbol(s)) = eg[*v].nodes.first()
                        && s == lam_var
                    {
                        true
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        })
        .collect();

    apps.extend(curr_apps);
}

#[allow(unused_variables)]
#[allow(unused_mut)]
fn infer_dom(eg: &EGraph<Mim, MimAnalysis>, lam_var: &str, body_id: &Id) -> TypeExpr {
    let mut body_apps: Vec<Mim> = vec![];
    #[cfg(test)]
    find_apps(eg, body_id, lam_var, &mut body_apps);

    for app in body_apps.iter() {
        let app_childs = app.children();
        let callee_id = app_childs.first().unwrap();
        let callee_type = &eg[*callee_id].data.type_;
        if let Some(type_expr) = callee_type
            && matches!(type_expr.last(), Some(Mim::Pi(..) | Mim::ImplicitPi(..)))
        {
            let pi_dom = child!(callee_type.clone().unwrap(), 1);
            return pi_dom;
        }
    }

    TypeExpr::hole()
}

fn make_lam_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let childs = expect!(enode, Mim::Lam(childs) => childs );

    if childs.len() <= 1 {
        return AnalysisData {
            type_: Some(TypeExpr::hole()),
            ..Default::default()
        };
    }

    let body_id = childs.get(2).expect("Expected lam body id");
    let body_type = eg[*body_id].data.type_.clone();

    let var_id = childs.first().expect("Expected lam var id");
    let lam_var = eg[*var_id].nodes.first().expect("Expected lam var node");
    let lam_var = {
        if let Mim::Symbol(s) = lam_var {
            s
        } else {
            panic!("Expected lam var to be a symbol")
        }
    };

    let dom = infer_dom(eg, lam_var, body_id);
    let codom = body_type.unwrap_or(TypeExpr::hole());

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
        ..Default::default()
    }
}

fn make_con_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let childs = expect!(enode, Mim::Con(childs) => childs );

    if childs.len() <= 1 {
        return AnalysisData {
            type_: Some(TypeExpr::hole()),
            ..Default::default()
        };
    }

    let body_id = childs.get(2).expect("Expected con body id");

    let var_id = childs.first().expect("Expected con var id");
    let con_var = eg[*var_id].nodes.first().expect("Expected con var node");
    let con_var = {
        if let Mim::Symbol(s) = con_var {
            s
        } else {
            panic!("Expected con var to be a symbol")
        }
    };

    let dom = infer_dom(eg, con_var, body_id);
    let codom = TypeExpr::bot(TypeExpr::type_(0));

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
        ..Default::default()
    }
}

fn make_fun_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let childs = expect!(enode, Mim::Fun(childs) => childs );

    let _body_type = if childs.len() > 1 {
        let body_id = childs.get(2).expect("Expected fun body id");
        eg[*body_id].data.type_.clone()
    } else {
        None
    };

    // TODO: Domain inference is a bit more complicated than for lam - need app to var#0
    // let ret_dom = body_type.unwrap_or(TypeExpr::hole());
    // let ret_codom = TypeExpr::bot(TypeExpr::type_(0));
    // let ret_pi = TypeExpr::pi(ret_dom, ret_codom, None);
    // let dom_inner = TypeExpr::hole();
    // let dom = TypeExpr::sigma(vec![dom_inner, ret_pi], None);
    // let codom = TypeExpr::bot(TypeExpr::type_(0));

    // Furthermore, we can't really use the inferrence above that we used in slotted
    // because here sigmas are variadic and if I infer a sigma with a single hole for
    // the domain, I might end up unifying it with a sigma containing two or more types
    // in place of the domain which leads to messed up unification i.e. [_] + [Nat, Nat, ...] = [Nat]

    let dom = TypeExpr::hole();
    let codom = TypeExpr::bot(TypeExpr::type_(0));

    AnalysisData {
        type_: Some(TypeExpr::pi(dom, codom, None)),
        ..Default::default()
    }
}

fn make_app_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let [callee, _arg] = expect!(enode, Mim::App([callee, arg]) => [callee, arg]);
    let callee_type = &eg[*callee].data.type_;

    match callee_type {
        Some(type_expr) => {
            if matches!(type_expr.last(), Some(Mim::Pi(..) | Mim::ImplicitPi(..))) {
                let codom = child!(type_expr, 2);
                AnalysisData {
                    type_: Some(codom),
                    ..Default::default()
                }
            } else {
                AnalysisData {
                    type_: Some(TypeExpr::hole()),
                    ..Default::default()
                }
            }
        }
        _ => AnalysisData {
            type_: Some(TypeExpr::hole()),
            ..Default::default()
        },
    }
}

fn make_var_type(_eg: &EGraph<Mim, MimAnalysis>, _enode: &Mim) -> AnalysisData {
    AnalysisData {
        type_: Some(TypeExpr::hole()),
        ..Default::default()
    }
}

fn make_lit_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let [_val, type_] = expect!(enode, Mim::Lit([val, type_]) => [val, type_]);

    let type_ = eg.id_to_expr(*type_);
    AnalysisData {
        type_: Some(type_),
        ..Default::default()
    }
}

fn make_pack_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let [_var, arity, body] = expect!(enode, Mim::Pack([var, arity, body]) => [var, arity, body]);

    let arity = eg.id_to_expr(*arity);
    let body_type = eg[*body].data.type_.clone();

    AnalysisData {
        type_: Some(TypeExpr::arr(
            arity,
            body_type.unwrap_or(TypeExpr::hole()),
            None,
        )),
        ..Default::default()
    }
}

fn make_tuple_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let childs = expect!(enode, Mim::Tuple(childs) => childs);

    let mut types: Vec<TypeExpr> = Vec::new();

    for child in childs.iter() {
        let type_ = eg[*child].data.type_.clone();
        types.push(type_.unwrap_or(TypeExpr::hole()));
    }

    AnalysisData {
        type_: Some(TypeExpr::sigma(types, None)),
        ..Default::default()
    }
}

fn make_extract_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let [tuple, index] = expect!(enode, Mim::Extract([tuple, index]) => [tuple, index]);
    let tuple_type = &eg[*tuple].data.type_;
    let index = eg.id_to_expr(*index);

    let mut extract_type = TypeExpr::hole();

    if let Some(type_expr) = tuple_type
        && matches!(type_expr.last(), Some(Mim::Arr(..)))
    {
        extract_type = child!(type_expr, 2).clone();
    } else if let Some(type_expr) = tuple_type
        && matches!(type_expr.last(), Some(Mim::Sigma(..)))
        && matches!(index.last(), Some(Mim::Lit(..)))
    {
        let sigma_types = childs!(type_expr, 1);
        let index_literal = get_literal(&index);
        extract_type = sigma_types[index_literal as usize].clone();
    }

    AnalysisData {
        type_: Some(extract_type),
        ..Default::default()
    }
}

fn make_insert_type(eg: &EGraph<Mim, MimAnalysis>, enode: &Mim) -> AnalysisData {
    let [tuple, index, value] =
        expect!(enode, Mim::Insert([tuple, index, value]) => [tuple, index, value]);
    let tuple_type = &eg[*tuple].data.type_;
    let value_type = &eg[*value].data.type_;
    let index = eg.id_to_expr(*index);

    let mut insert_type = TypeExpr::hole();

    if let Some(type_expr) = tuple_type
        && matches!(type_expr.last(), Some(Mim::Arr(..)))
    {
        insert_type = type_expr.clone();
    } else if let Some(type_expr) = tuple_type
        && matches!(type_expr.last(), Some(Mim::Sigma(..)))
        && matches!(index.last(), Some(Mim::Lit(..)))
    {
        let mut sigma_types = childs!(type_expr, 1);
        let index_literal = get_literal(&index);
        let value_type = value_type.clone().unwrap_or(TypeExpr::hole());
        sigma_types[index_literal as usize] = value_type;
        insert_type = TypeExpr::sigma(sigma_types, None);
    }

    AnalysisData {
        type_: Some(insert_type),
        ..Default::default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ffi::FFI;
    use egg::rewrite;
    const MAX_LINE: usize = 80;

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
            untyped.pretty(MAX_LINE),
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

    #[test]
    fn make_eta_expansion_hole() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let fun_annotated = "(@ (pi x Nat Bool) func)".parse().unwrap();
        let fun_typed = extract_type_annotations(&fun_annotated);
        let fun_typed_id = add_expr_typed(&mut eg, fun_typed);

        assert_eq!(type_of(&eg, &fun_typed_id), type_("(pi x Nat Bool)"));

        let eta_exp_lam = "(lam x (lit ff Bool) (app func (var x)))".parse().unwrap();
        let eta_exp_lam_id = eg.add_expr(&eta_exp_lam);

        assert_eq!(type_of(&eg, &eta_exp_lam_id), type_("(pi dummy Nat Bool)"));

        let eta_exp_con = "(con x (lit ff Bool) (app func (var x)))".parse().unwrap();
        let eta_exp_con_id = eg.add_expr(&eta_exp_con);

        assert_eq!(
            type_of(&eg, &eta_exp_con_id),
            type_("(pi dummy Nat (bot (type (lit 0 Univ))))")
        );

        let eta_exp_fun = "(fun x (lit ff Bool) (app func (var x)))".parse().unwrap();
        let eta_exp_fun_id = eg.add_expr(&eta_exp_fun);

        assert_eq!(
            type_of(&eg, &eta_exp_fun_id),
            type_("(pi dummy (hole (type (lit 0 Univ))) (bot (type (lit 0 Univ))))")
        );
    }

    #[test]
    fn make_types_var_lit() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let lit = "(lit 10 (idx 3))".parse().unwrap();
        let lit_id = eg.add_expr(&lit);

        assert_eq!(type_of(&eg, &lit_id), type_("(idx 3)"));

        let binding = "(let x (lit tt Bool) (app (lam y (lit ff Bool) (lit 10 (idx 3))) (var x)))"
            .parse()
            .unwrap();
        let binding_id = eg.add_expr(&binding);

        assert_eq!(type_of(&eg, &binding_id), type_("(idx 3)"));

        let var = "foo".parse().unwrap();
        let var_id = eg.add_expr(&var);

        assert_eq!(type_of(&eg, &var_id), type_("(hole (type (lit 0 Univ)))"));

        let app = "(app foo bar)".parse().unwrap();
        let app_id = eg.add_expr(&app);

        assert_eq!(type_of(&eg, &app_id), type_("(hole (type (lit 0 Univ)))"));
    }

    #[test]
    fn make_types_tuple_pack() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let tuple = "(tuple (lit 1 Nat) (lit 2 Nat) (lit 3 Nat))"
            .parse()
            .unwrap();
        let tuple_id = eg.add_expr(&tuple);

        assert_eq!(type_of(&eg, &tuple_id), type_("(sigma dummy Nat Nat Nat)"));

        let tuple_empty = "(tuple)".parse().unwrap();
        let tuple_empty_id = eg.add_expr(&tuple_empty);

        assert_eq!(type_of(&eg, &tuple_empty_id), type_("(sigma dummy)"));

        let pack = "(pack dummy (top Nat) (lit 3 Nat))".parse().unwrap();
        let pack_id = eg.add_expr(&pack);

        assert_eq!(type_of(&eg, &pack_id), type_("(arr dummy (top Nat) Nat)"));
    }

    #[test]
    fn make_types_extract_insert() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let insert_tuple = "(insert (tuple (lit 1 Nat) (lit 2 Nat)) (lit tt Bool) (lit ff Bool))"
            .parse()
            .unwrap();
        let insert_tuple_id = eg.add_expr(&insert_tuple);

        assert_eq!(
            type_of(&eg, &insert_tuple_id),
            type_("(sigma dummy Nat Bool)")
        );

        let insert_pack =
            "(insert (pack dummy (top Nat) (lit ff Bool)) (lit tt Bool) (lit ff Bool))"
                .parse()
                .unwrap();
        let insert_pack_id = eg.add_expr(&insert_pack);

        assert_eq!(
            type_of(&eg, &insert_pack_id),
            type_("(arr dummy (top Nat) Bool)")
        );

        let extract_tuple = "(extract (tuple (lit 1 Nat) (lit 3 (idx i32))) (lit tt Bool))"
            .parse()
            .unwrap();
        let extract_tuple_id = eg.add_expr(&extract_tuple);

        assert_eq!(type_of(&eg, &extract_tuple_id), type_("(idx i32)"));

        let extract_pack = "(extract (pack dummy (top Nat) (lit ff Bool)) (lit 0 (idx 1)))"
            .parse()
            .unwrap();
        let extract_pack_id = eg.add_expr(&extract_pack);

        assert_eq!(type_of(&eg, &extract_pack_id), type_("Bool"));

        let extract_var = "(extract foo (lit 0 (idx 1)))".parse().unwrap();
        let extract_var_id = eg.add_expr(&extract_var);

        assert_eq!(
            type_of(&eg, &extract_var_id),
            type_("(hole (type (lit 0 Univ)))")
        );
    }

    #[test]
    fn make_var_type() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let var_annotated = "(@ Bool (var foo))".parse().unwrap();
        let var_typed = extract_type_annotations(&var_annotated);
        let var_typed_id = add_expr_typed(&mut eg, var_typed);

        assert_eq!(type_of(&eg, &var_typed_id), type_("Bool"));

        let var = "(var bar)".parse().unwrap();
        let var_id = eg.add_expr(&var);

        assert_eq!(type_of(&eg, &var_id), type_("(hole (type (lit 0 Univ)))"));
    }

    #[test]
    fn infer_let_type() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let let_annotated = "(let foo (@ Bool (lit ff Bool)) (@ Nat (lit 1 Nat)))"
            .parse()
            .unwrap();
        let let_typed = extract_type_annotations(&let_annotated);
        let let_typed_id = add_expr_typed(&mut eg, let_typed);

        assert_eq!(type_of(&eg, &let_typed_id), type_("Nat"));

        let let_var_annotated = "(let foo (@ Bool (lit ff Bool)) (@ Nat (var bar)))"
            .parse()
            .unwrap();
        let let_var_typed = extract_type_annotations(&let_var_annotated);
        let let_var_typed_id = add_expr_typed(&mut eg, let_var_typed);

        assert_eq!(type_of(&eg, &let_var_typed_id), type_("Nat"));
    }

    #[test]
    fn implicit_pi_callee() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let f_annotated = "(@ (pi* dummy Nat (pi dummy Nat Nat)) (var f))"
            .parse()
            .unwrap();
        let f_typed = extract_type_annotations(&f_annotated);
        let f_typed_id = add_expr_typed(&mut eg, f_typed);

        assert_eq!(
            type_of(&eg, &f_typed_id),
            type_("(pi* dummy Nat (pi dummy Nat Nat))")
        );

        let implicit_app = "(app (var f) (lit 1 Nat))".parse().unwrap();
        let implicit_app_id = eg.add_expr(&implicit_app);
        assert_eq!(type_of(&eg, &implicit_app_id), type_("(pi dummy Nat Nat)"));
    }

    #[test]
    fn union_hole_pis() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let f_annotated = "(@ (pi dummy (hole (type (lit 0 Univ))) Nat) (var f))"
            .parse()
            .unwrap();
        let f_typed = extract_type_annotations(&f_annotated);
        let f_typed_id = add_expr_typed(&mut eg, f_typed);

        assert_eq!(
            type_of(&eg, &f_typed_id),
            type_("(pi dummy (hole (type (lit 0 Univ))) Nat)")
        );

        let g_annotated = "(@ (pi dummy Nat (hole (type (lit 0 Univ)))) (var g))"
            .parse()
            .unwrap();
        let g_typed = extract_type_annotations(&g_annotated);
        let g_typed_id = add_expr_typed(&mut eg, g_typed);

        eg.union(f_typed_id, g_typed_id);

        assert_eq!(type_of(&eg, &f_typed_id), type_("(pi dummy Nat Nat)"));
        assert_eq!(type_of(&eg, &g_typed_id), type_("(pi dummy Nat Nat)"));
    }

    #[test]
    fn union_sigma_with_vars() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let a =
            "(@ (sigma foo (hole (type (lit 0 Univ))) (extract (var foo) (lit ff Bool))) (var a))"
                .parse()
                .unwrap();
        let a = extract_type_annotations(&a);
        let a_id = add_expr_typed(&mut eg, a);

        let b = "(let bar (tuple) (@ (sigma foo (extract (var bar) (lit ff Bool)) (hole (type (lit 0 Univ)))) (var b)))".parse().unwrap();
        let b = extract_type_annotations(&b);
        let b_id = add_expr_typed(&mut eg, b);

        eg.union(a_id, b_id);

        assert_eq!(
            type_of(&eg, &a_id),
            type_(
                "(sigma foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))"
            )
        );
        assert_eq!(
            type_of(&eg, &b_id),
            type_(
                "(sigma foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))"
            )
        );
    }

    #[test]
    fn union_arr_with_vars() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let a =
            "(@ (arr foo (hole (type (lit 0 Univ))) (extract (var foo) (lit ff Bool))) (var a))"
                .parse()
                .unwrap();
        let a = extract_type_annotations(&a);
        let a_id = add_expr_typed(&mut eg, a);

        let b = "(let bar (tuple) (@ (arr foo (extract (var bar) (lit ff Bool)) (hole (type (lit 0 Univ)))) (var b)))".parse().unwrap();
        let b = extract_type_annotations(&b);
        let b_id = add_expr_typed(&mut eg, b);

        eg.union(a_id, b_id);

        assert_eq!(
            type_of(&eg, &a_id),
            type_("(arr foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))")
        );
        assert_eq!(
            type_of(&eg, &b_id),
            type_("(arr foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))")
        );
    }

    #[test]
    fn type_depending_on_outer_var() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let b = "(let bar
                            (lit ff Bool)
                            (@ (arr foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))
                            (var b)))"
        .parse()
        .unwrap();
        let b = extract_type_annotations(&b);
        let b_id = add_expr_typed(&mut eg, b);

        assert_eq!(
            type_of(&eg, &b_id),
            type_("(arr foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))")
        );

        let lam_rewrite: Rewrite<Mim, MimAnalysis> = rewrite!("lam-rewrite";
            "(let bar (lit ff Bool) ?e)"
            => "(let baz Nat ?e)" );

        let runner = Runner::<Mim, MimAnalysis>::default().with_egraph(eg);
        let runner = runner.run(&[lam_rewrite]);

        let extractor = Extractor::new(&runner.egraph, AstSize);
        let (_c, b) = extractor.find_best(b_id);

        assert_eq!(format!("{}", b), "(let baz Nat (var b))",);

        let b_ffi = b.to_ffi(Some(&runner.egraph));
        let _b_ffi_type = &b_ffi.nodes.last().unwrap().type_;

        // TODO: This should work but doesn't yet
        // assert_eq!(
        //     format!("{}", b_ffi_type.pretty(80)),
        //     "(arr foo (extract (var baz) (lit ff Bool)) (extract (var foo) (lit ff Bool)))"
        // );

        assert_eq!(
            type_of(&runner.egraph, &b_id),
            type_("(arr foo (extract (var bar) (lit ff Bool)) (extract (var foo) (lit ff Bool)))")
        );
    }

    #[test]
    fn infer_lam_domain() {
        let mut eg = EGraph::<Mim, MimAnalysis>::default();

        let f = "(@ (pi dummy Nat Bool) (var f))".parse().unwrap();
        let f = extract_type_annotations(&f);
        let f_id = add_expr_typed(&mut eg, f);

        assert_eq!(type_of(&eg, &f_id), type_("(pi dummy Nat Bool)"));

        let g = "(@ (pi dummy Bool Nat) (var g))".parse().unwrap();
        let g = extract_type_annotations(&g);
        let g_id = add_expr_typed(&mut eg, g);

        assert_eq!(type_of(&eg, &g_id), type_("(pi dummy Bool Nat)"));

        let lam = "(lam x
                            (lit ff Bool)
                            (lam y
                                (lit ff Bool)
                                (tuple
                                    (app (var g) (var y))
                                    (app (var f) (var x)))))"
            .parse()
            .unwrap();
        let lam_id = eg.add_expr(&lam);

        assert_eq!(
            type_of(&eg, &lam_id),
            type_(
                "(pi dummy
                    Nat
                    (pi dummy
                        Bool
                        (sigma dummy Nat Bool)))"
            )
        );
    }
}
