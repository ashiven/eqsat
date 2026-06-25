use crate::OptionSelected;
use crate::{
    eqsat_egg, eqsat_slotted, node_ffi_str, pretty_egg, pretty_ffi, pretty_slotted, reaches_egg,
    reaches_slotted, type_str,
};

#[allow(clippy::module_inception)]
#[cxx::bridge]
pub mod bridge {
    #[derive(Debug)]
    enum RuleSet {
        // Egg
        Core,
        Math,
        // AUTOGEN START: egg-ruleset-rust-ffi
        // AUTOGEN END: egg-ruleset-rust-ffi

        // Slotted
        Standard,
        Rise,
        Normalize,
        // AUTOGEN START: slotted-ruleset-rust-ffi
        // AUTOGEN END: slotted-ruleset-rust-ffi
    }

    #[derive(Debug)]
    enum CostFn {
        // Egg/Slotted
        AstSize,

        // Egg
        AstDepth,
        // AUTOGEN START: egg-cost-rust-ffi
        // AUTOGEN END: egg-cost-rust-ffi

        // Slotted
        MaxAstSize,
        // AUTOGEN START: slotted-cost-rust-ffi
        // AUTOGEN END: slotted-cost-rust-ffi
    }

    #[derive(Debug, Hash, Default)]
    enum MimKind {
        Let,
        Lam,
        Con,
        Fun,
        App,
        Var,
        Lit,
        Pack,
        Tuple,
        Extract,
        Insert,
        Rule,
        Inj,
        Merge,
        Axm,
        Match,
        Proxy,
        Join,
        Meet,
        Bot,
        Top,
        Arr,
        Sigma,
        ImplicitPi,
        Pi,
        Cn,
        Fn,
        Idx,
        Hole,
        Type,
        Reform,
        TypeWrap,
        MetaVar,
        Root,
        Scope,
        Cons,
        #[default]
        Nil,
        Num,
        Symbol,
    }

    #[derive(Debug, Hash, Default, Eq, PartialEq)]
    struct NodeFFI {
        kind: MimKind,
        children: Vec<u32>,
        num: u64,
        symbol: String,
        slot: String,
        type_: RecExprFFI,
    }

    #[derive(Debug, Hash, Default, Eq, PartialEq)]
    struct RecExprFFI {
        nodes: Vec<NodeFFI>,
    }

    struct OptionSelected {
        option: *mut Vec<String>,
    }

    extern "Rust" {
        fn eqsat_egg(
            sexpr: &str,
            selected: OptionSelected,
            rulesets: Vec<RuleSet>,
            cost_fn: CostFn,
        ) -> Vec<RecExprFFI>;
        fn reaches_egg(
            sexpr: &str,
            rulesets: Vec<RuleSet>,
            start_name: &str,
            end_name: &str,
            max_steps: usize,
        ) -> bool;
        fn pretty_egg(sexpr: &str, line_len: usize) -> String;

        fn eqsat_slotted(
            sexpr: &str,
            selected: OptionSelected,
            rulesets: Vec<RuleSet>,
            cost_fn: CostFn,
        ) -> Vec<RecExprFFI>;
        fn reaches_slotted(
            sexpr: &str,
            rulesets: Vec<RuleSet>,
            start_name: &str,
            end_name: &str,
            max_steps: usize,
        ) -> bool;
        fn pretty_slotted(sexpr: &str, line_len: usize) -> String;

        fn pretty_ffi(sexpr: Vec<RecExprFFI>, line_len: usize) -> String;
        fn node_ffi_str(node: NodeFFI) -> String;
        fn type_str(type_: RecExprFFI, line_len: usize) -> String;
    }
}

impl OptionSelected {
    pub fn none() -> Self {
        OptionSelected {
            option: std::ptr::null_mut(),
        }
    }
}
