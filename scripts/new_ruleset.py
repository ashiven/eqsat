from pathlib import Path
from argparse import ArgumentParser
from re import compile, DOTALL, MULTILINE


def replace_ruleset_mim(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*/// AUTOGEN START:\s*{implementation}-ruleset-mim\s*$)"
        rf"(.*?)"
        rf"(^\s*/// AUTOGEN END:\s*{implementation}-ruleset-mim\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
axm %eqsat.{ruleset_name}: %eqsat.Ruleset;
"""

    file_path = Path(__file__).parent.parent / "eqsat.mim"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_ruleset_cpp(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-ruleset-cpp\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-ruleset-cpp\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
                    else if (Axm::isa<eqsat::{ruleset_name}>(ruleset))
                        rulesets.push_back(RuleSet::{ruleset_name.capitalize()});
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


def replace_ruleset_rust_mod(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-ruleset-rust-mod\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-ruleset-rust-mod\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
pub mod {ruleset_name};
"""

    file_path = (
        Path(__file__).parent.parent / f"src/mim_{implementation}/rulesets/mod.rs"
    )

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_ruleset_rust_match(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-ruleset-rust-match\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-ruleset-rust-match\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
                RuleSet::{ruleset_name.capitalize()} => rules.extend({ruleset_name}::rules()),
"""

    file_path = (
        Path(__file__).parent.parent / f"src/mim_{implementation}/rulesets/mod.rs"
    )

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_ruleset_rust_ffi(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-ruleset-rust-ffi\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-ruleset-rust-ffi\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
        {ruleset_name.capitalize()},
"""

    file_path = Path(__file__).parent.parent / "src/ffi.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def create_new_ruleset_file(implementation: str, ruleset_name: str):
    file_path = (
        Path(__file__).parent.parent
        / f"src/mim_{implementation}/rulesets/{ruleset_name}.rs"
    )

    generated_slotted = """
use crate::mim_slotted::{MimSlotted, analysis::MimSlottedAnalysis};
use slotted_egraphs::Rewrite;

pub fn rules() -> Vec<Rewrite<MimSlotted, MimSlottedAnalysis>> {
    let rules = vec![
        my_rule(),
    ];

    rules
}

fn my_rule() -> Rewrite<MimSlotted, MimSlottedAnalysis> {
    let pat = "(tuple (cons ?a (cons ?a (cons ?a nil))))";
    let outpat = "(pack $dummy (scope (lit 3 Nat) ?a))";
    Rewrite::new("my-rule", pat, outpat)
}
""".lstrip()

    generated_egg = """
use crate::mim_egg::{Mim, analysis::MimAnalysis};
use egg::Rewrite;

pub fn rules() -> Vec<Rewrite<Mim, MimAnalysis>> {
    let rules = vec![
        my_rule(),
    ];

    rules
}

fn my_rule() -> Rewrite<Mim, MimAnalysis> {
    let pat: Pattern<Mim> = "(app %core.nat.add (tuple (lit 0 Nat) ?e))".parse().unwrap();
    let outpat: Pattern<Mim> = "?e".parse().unwrap();
    Rewrite::new("my-rule", pat, outpat).unwrap()
}
""".lstrip()

    file_path.write_text(
        generated_egg if implementation == "egg" else generated_slotted
    )


def main():
    parser = ArgumentParser()
    parser.add_argument("implementation", choices=["egg", "slotted"])
    parser.add_argument("ruleset_name")

    args = parser.parse_args()

    replace_ruleset_mim(args.implementation, args.ruleset_name)
    replace_ruleset_cpp(args.implementation, args.ruleset_name)
    replace_ruleset_rust_mod(args.implementation, args.ruleset_name)
    replace_ruleset_rust_match(args.implementation, args.ruleset_name)
    replace_ruleset_rust_ffi(args.implementation, args.ruleset_name)

    create_new_ruleset_file(args.implementation, args.ruleset_name)


if __name__ == "__main__":
    main()
