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

    file_path = Path(__file__).parent.parent / f"src/{implementation}/rulesets/mod.rs"

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

    file_path = Path(__file__).parent.parent / f"src/{implementation}/rulesets/mod.rs"

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

    file_path = Path(__file__).parent.parent / "src/ffi/bridge.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_analysis_rust_import(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-analysis-rust-import\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-analysis-rust-import\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
use crate::{implementation}::rulesets::{ruleset_name}::{{{ruleset_name.capitalize()}Analysis, {ruleset_name.capitalize()}Data}};
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/analysis.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_analysis_rust_make(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-analysis-rust-make\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-analysis-rust-make\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
                RuleSet::{ruleset_name.capitalize()} => {{
                    let data = {ruleset_name.capitalize()}Analysis::make(eg, enode);
                    combined_data.combine(data);
                }}
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/analysis.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_analysis_rust_merge(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-analysis-rust-merge\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-analysis-rust-merge\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
                RuleSet::{ruleset_name.capitalize()} => {{
                    let {"data" if implementation == "slotted" else "merge"} = {ruleset_name.capitalize()}Analysis::merge(l{".clone()" if implementation == "slotted" else ""}, r.clone());
                    {"combined_data.combine(data);" if implementation == "slotted" else "combined_merge = combined_merge | merge;"}
                }}
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/analysis.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_analysis_rust_data(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-analysis-rust-data\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-analysis-rust-data\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
    pub {ruleset_name}: Option<{ruleset_name.capitalize()}Data>,
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/analysis.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def replace_analysis_rust_combine(implementation: str, ruleset_name: str):
    pattern = compile(
        rf"(^\s*// AUTOGEN START:\s*{implementation}-analysis-rust-combine\s*$)"
        rf"(.*?)"
        rf"(^\s*// AUTOGEN END:\s*{implementation}-analysis-rust-combine\s*$)",
        DOTALL | MULTILINE,
    )

    generated = f"""
        self.{ruleset_name} = self.{ruleset_name}.take().or(other.{ruleset_name});
"""

    file_path = Path(__file__).parent.parent / f"src/{implementation}/analysis.rs"

    content = file_path.read_text()
    content = pattern.sub(
        lambda m: m.group(1) + m.group(2).rstrip() + generated + m.group(3),
        content,
    )

    file_path.write_text(content)


def create_new_ruleset_file(implementation: str, ruleset_name: str):
    file_path = (
        Path(__file__).parent.parent
        / f"src/{implementation}/rulesets/{ruleset_name}.rs"
    )

    generated_slotted = f"""
use crate::slotted::{{Mim, analysis::AnalysisData, analysis::MimAnalysis}};
use slotted_egraphs::{{EGraph, Rewrite}};

pub fn rules() -> Vec<Rewrite<Mim, MimAnalysis>> {{
    let rules = vec![
        my_rule(),
    ];

    rules
}}

fn my_rule() -> Rewrite<Mim, MimAnalysis> {{
    let pat = "(tuple (cons ?a (cons ?a (cons ?a nil))))";
    let outpat = "(pack $dummy (scope (lit 3 Nat) ?a))";
    Rewrite::new("my-rule", pat, outpat)
}}

pub type {ruleset_name.capitalize()}Data = ();
pub struct {ruleset_name.capitalize()}Analysis;

impl {ruleset_name.capitalize()}Analysis {{
    pub fn make(_eg: &EGraph<Mim, MimAnalysis>, _enode: &Mim) -> AnalysisData {{
        AnalysisData::default()
    }}
    pub fn merge(_l: AnalysisData, _r: AnalysisData) -> AnalysisData {{
        AnalysisData::default()
    }}
}}
""".lstrip()

    generated_egg = f"""
use crate::egg::{{Mim, analysis::AnalysisData, analysis::MimAnalysis}};
use egg::{{EGraph, Rewrite, Pattern}};

pub fn rules() -> Vec<Rewrite<Mim, MimAnalysis>> {{
    let rules = vec![
        my_rule(),
    ];

    rules
}}

fn my_rule() -> Rewrite<Mim, MimAnalysis> {{
    let pat: Pattern<Mim> = "(app %core.nat.add (tuple (lit 0 Nat) ?e))".parse().unwrap();
    let outpat: Pattern<Mim> = "?e".parse().unwrap();
    Rewrite::new("my-rule", pat, outpat).unwrap()
}}

pub type {ruleset_name.capitalize()}Data = ();
pub struct {ruleset_name.capitalize()}Analysis;

impl {ruleset_name.capitalize()}Analysis {{
    pub fn make(_eg: &EGraph<Mim, MimAnalysis>, _enode: &Mim) -> AnalysisData {{
        AnalysisData::default()
    }}
    pub fn merge(_l: &mut AnalysisData, _r: AnalysisData) -> DidMerge {{
        DidMerge(false, false)
    }}
}}
""".lstrip()

    file_path.write_text(
        generated_egg if implementation == "egg" else generated_slotted
    )


def main():
    parser = ArgumentParser()
    parser.add_argument("implementation", choices=["egg", "slotted"])
    parser.add_argument("ruleset_name", type=str.lower)

    args = parser.parse_args()

    replace_ruleset_mim(args.implementation, args.ruleset_name)
    replace_ruleset_cpp(args.implementation, args.ruleset_name)
    replace_ruleset_rust_mod(args.implementation, args.ruleset_name)
    replace_ruleset_rust_match(args.implementation, args.ruleset_name)
    replace_ruleset_rust_ffi(args.implementation, args.ruleset_name)

    replace_analysis_rust_import(args.implementation, args.ruleset_name)
    replace_analysis_rust_make(args.implementation, args.ruleset_name)
    replace_analysis_rust_merge(args.implementation, args.ruleset_name)
    replace_analysis_rust_data(args.implementation, args.ruleset_name)
    replace_analysis_rust_combine(args.implementation, args.ruleset_name)

    create_new_ruleset_file(args.implementation, args.ruleset_name)


if __name__ == "__main__":
    main()
