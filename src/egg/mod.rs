// AUTOGEN START: egg-cost-rust-import
// AUTOGEN END: egg-cost-rust-import
use egg::*;

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

define_language! {
    pub enum Mim {
        // TERMS

        // (let <var> <definition> <expression>)
        "let" = Let([Id; 3]),
        // (lam <var> [<filter> <body>])
        "lam" = Lam(Box<[Id]>),
        // (con <var> [<filter> <body>])
        "con" = Con(Box<[Id]>),
        // (fun <var> [<filter> <body>])
        "fun" = Fun(Box<[Id]>),
        // (app <callee> <arg>)
        "app" = App([Id; 2]),
        // (var <name>)
        "var" = Var(Id),
        // (lit <value> <type>)
        "lit" = Lit([Id; 2]),
        // (pack <var> <arity> <body>)
        "pack" = Pack([Id; 3]),
        // (tuple <elems>...)
        "tuple" = Tuple(Box<[Id]>),
        // (extract <tuple> <index>)
        "extract" = Extract([Id; 2]),
        // (ins <tuple> <index> <value>)
        "insert" = Insert([Id; 3]),
        // (rule <name> <meta-var> <lhs> <rhs> <guard>)
        "rule" = Rule([Id; 5]),
        // (inj <type> <value>)
        "inj" = Inj([Id; 2]),
        // (merge <type> <values>...)
        "merge" = Merge(Box<[Id]>),
        // (axm <name>)
        "axm" = Axm(Id),
        // (match <scrutinee> <arms>...)
        "match" = Match(Box<[Id]>),
        // (proxy <type> <pass> <tag> <ops>...)
        "proxy" = Proxy(Box<[Id]>),


        // TYPES

        // (join <types>...)
        "join" = Join(Box<[Id]>),
        // (meet <types>...)
        "meet" = Meet(Box<[Id]>),
        // (bot <type>)
        "bot" = Bot(Id),
        // (top <type>)
        "top" = Top(Id),
        // (arr <var> <arity> <body>)
        "arr" = Arr([Id; 3]),
        // (sigma <var> <types>...)
        "sigma" = Sigma(Box<[Id]>),
        // (fn <var> <domain> <codomain>)
        "fn" = Fn_([Id; 3]),
        // (cn <var> <domain> <codomain>)
        "cn" = Cn([Id; 3]),
        // (pi <var> <domain> <codomain>)
        "pi" = Pi([Id; 3]),
        // (pi* <var> <domain> <codomain>)
        "pi*" = ImplicitPi([Id; 3]),
        // (idx <size>)
        "idx" = Idx(Id),
        // (hole <type>)
        "hole" = Hole(Id),
        // (type <level>)
        "type" = Type(Id),
        // (reform <meta_type>)
        "reform" = Reform(Id),

        // (@ <type> <value>)
        "@" = TypeWrap([Id; 2]),
        // (root <extern> <name> <definition>)
        "root" = Root([Id; 3]),
        // (metavar <name> [<projs>...])
        "metavar" = MetaVar(Box<[Id]>),

        Num(u64), Symbol(String),
    }
}
