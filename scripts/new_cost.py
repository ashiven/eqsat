from pathlib import Path
from argparse import ArgumentParser
from re import compile, DOTALL, MULTILINE


def replace_cost_mim(implementation: str, cost_name: str):
    pattern = compile(
        rf"(^\s*/// AUTOGEN START:\s*{implementation}-cost-mim\s*$)"
        rf"(.*?)"
        rf"(^\s*/// AUTOGEN END:\s*{implementation}-cost-mim\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
axm %eqsat.{cost_name}: %eqsat.Config;
"""

    file_path = Path(__file__).parent.parent / "eqsat.mim"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_cost_cpp(implementation: str, cost_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-cost-cpp\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-cost-cpp\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
            }} else if (Axm::isa<eqsat::{cost_name}>(config_val)) {{
                cost_fn = CostFn::{cost_name};
"""

    file_path = (
        Path(__file__).parent.parent / f"plug/phase/rewrite_{implementation}.cpp"
    )

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_cost_rust_import(implementation: str, cost_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-cost-rust-import\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-cost-rust-import\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
use crate::{implementation}::cost::{cost_name};
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/mod.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_cost_rust_match(implementation: str, cost_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-cost-rust-match\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-cost-rust-match\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
        CostFn::{cost_name} => rewrite_sexprs(&sexprs, &selected, rules, || {cost_name}),
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/mod.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_cost_rust_ffi(implementation: str, cost_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-cost-rust-ffi\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-cost-rust-ffi\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
        {cost_name},
"""

    file_path = Path(__file__).parent.parent / "src/ffi.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_cost_rust_impl(implementation: str, cost_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-cost-rust-impl\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-cost-rust-impl\s*$)",
        DOTALL | MULTILINE,
    )

    generated_slotted = f"""
pub struct {cost_name};
impl CostFunction<Mim> for {cost_name} {{
    type Cost = u64;
    fn cost<C>(&self, enode: &Mim, costs: C) -> u64
    where
        C: Fn(Id) -> u64,
    {{
        let mut s: u64 = 1;
        for x in enode.applied_id_occurrences() {{
            s = s.saturating_add(costs(x.id));
        }}
        s
    }}
}}
"""

    generated_egg = f"""
#[derive(Debug)]
pub struct {cost_name};
impl CostFunction<Mim> for {cost_name} {{
    type Cost = usize;
    fn cost<C>(&mut self, enode: &Mim, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {{
        enode.fold(1, |sum, id| sum.saturating_add(costs(id)))
    }}
}}
"""

    generated = generated_egg if implementation == "egg" else generated_slotted

    file_path = Path(__file__).parent.parent / f"src/{implementation}/cost.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def main():
    parser = ArgumentParser()
    parser.add_argument("implementation", choices=["egg", "slotted"])
    parser.add_argument("ruleset_name")

    args = parser.parse_args()

    replace_cost_mim(args.implementation, args.ruleset_name)
    replace_cost_cpp(args.implementation, args.ruleset_name)
    replace_cost_rust_import(args.implementation, args.ruleset_name)
    replace_cost_rust_match(args.implementation, args.ruleset_name)
    replace_cost_rust_ffi(args.implementation, args.ruleset_name)
    replace_cost_rust_impl(args.implementation, args.ruleset_name)


if __name__ == "__main__":
    main()
