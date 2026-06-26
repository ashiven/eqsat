use slotted_egraphs::*;

pub mod analysis;
pub mod cost;
pub mod equiv;
pub mod print;
pub mod rewrite;
pub mod rules;
pub mod rulesets;
pub mod types;
pub mod util;

#[cfg(test)]
mod test;

// Parsing rec exprs with type annotations can become very stack intensive
// so we preemptively increase the stack size to avoid stack overflows.
const PARSE_STACK_SIZE: usize = 8 * 1024 * 1024;

define_language! {
    pub enum Mim {
        // TERMS

        // (let $name (scope <definition> <expr>))
        Let(Bind<AppliedId>) = "let",
        // (lam $var-name (scope <filter> <body>))
        Lam(Bind<AppliedId>) = "lam",
        // (con $var-name (scope <filter> <body>))
        Con(Bind<AppliedId>) = "con",
        // (fun $var-name (scope <filter> <body>))
        Fun(Bind<AppliedId>) = "fun",
        // (app <callee> <arg>)
        App(AppliedId, AppliedId) = "app",
        // (var $name)
        Var(Slot) = "var",
        // (lit <value> <type>)
        Lit(AppliedId, AppliedId) = "lit",
        // (pack $var (scope <arity> <body>))
        Pack(Bind<AppliedId>) = "pack",
        // (tuple <elem-cons>)
        Tuple(AppliedId) = "tuple",
        // (extract <tuple> <index>)
        Extract(AppliedId, AppliedId) = "extract",
        // (insert <tuple> <index> <value>)
        Insert(AppliedId, AppliedId, AppliedId) = "insert",
        // (rule <name> <meta-var-cons> <lhs> <rhs> <guard>)
        Rule(AppliedId, AppliedId, AppliedId, AppliedId, AppliedId) = "rule",
        // (inj <type> <value>)
        Inj(AppliedId, AppliedId) = "inj",
        // (merge <type> <type-cons>)
        Merge(AppliedId, AppliedId) = "merge",
        // (axm <name>)
        Axm(AppliedId) = "axm",
        // (match <op-cons>)
        Match(AppliedId) = "match",
        // (proxy <type> <pass> <tag> <op-cons>)
        Proxy(AppliedId, AppliedId, AppliedId, AppliedId) = "proxy",


        // TYPES

        // (join <type-cons>)
        Join(AppliedId) = "join",
        // (meet <type-cons>)
        Meet(AppliedId) = "meet",
        // (bot <type>)
        Bot(AppliedId) = "bot",
        // (top <type>)
        Top(AppliedId) = "top",
        // (arr $var (scope <arity> <body>))
        Arr(Bind<AppliedId>) = "arr",
        // (sigma $var (scope <type-cons> nil))
        Sigma(Bind<AppliedId>) = "sigma",
        // (pi* $var (scope <domain> <codomain>))
        ImplicitPi(Bind<AppliedId>) = "pi*",
        // (pi $var (scope <domain> <codomain>))
        Pi(Bind<AppliedId>) = "pi",
        // (cn $var (scope <domain> <codomain>))
        Cn(Bind<AppliedId>) = "cn",
        // (fn $var (scope <domain> <codomain>))
        Fn(Bind<AppliedId>) = "fn",
        // (idx <size>)
        Idx(AppliedId) = "idx",
        // (hole <type>)
        Hole(AppliedId) = "hole",
        // (type <level>)
        Type(AppliedId) = "type",
        // (reform <meta_type>)
        Reform(AppliedId) = "reform",


        // STRUCTURAL

        // We use this to annotate every term in the sexpr with a type as in (@ Bool (lit tt))
        // The sexprs we initially receive from the sexpr backend will be wrapped in types if we
        // provide the compiler flag --sexpr-include-types. However, we will not work with type-wrapped
        // sexprs during equality saturation as types are expected to be invariant per eclass.
        // We instead parse an initial typed RecExpr from which we extract all the type information
        // and then create an untyped RecExpr. The extracted type information will be added to
        // the egraph as analysis data that is merged upon eclass merges.
        // However, we have to reannotate the untyped RecExpr after equality saturation because
        // the rewrite phase requires type information for reconstruction.
        TypeWrap(AppliedId, AppliedId) = "@",

        // This is used to represent the meta variables introduced by rule declarations
        // without clashing with the 'var' nodes using slots.
        // (metavar <name>)
        MetaVar(AppliedId) = "metavar",

        // A root-level sexpr (in most cases this will be a closed/top-level continuation)
        // We introduce a node for this to avoid having to write (con extern main ...) to bind
        // named, top-level constructs and can instead write (root extern main (con ...)).
        // This allows us to omit names from lambda definitions entirely so we can get the full
        // benefits of slotted-egraphs while still having a binder for such constructs.
        // (root <extern> <name> <definition>)
        Root(AppliedId, AppliedId, AppliedId) = "root",

        // This is needed so we can bind a lambda variable to both its filter and body
        // and also bind a let variable to both its definition and its expression:
        // (scope <filter> <body>) or (scope <definition> <expression>)  i.e.: (let $foo (scope <def> <expr>))
        Scope(AppliedId, AppliedId) = "scope",

        // Enables variadic language constructs such as Tuple, Sigma, Match, ...
        // (cons <elem> <next>)
        Cons(AppliedId, AppliedId) = "cons",
        Nil() = "nil",

        // Leaf nodes
        Num(u64),
        Symbol(Symbol),
    }
}
