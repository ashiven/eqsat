<p align="center">
  <h2 align="center">eqsat</h2>
</p>

<p align="center">
  <b>Equality Saturation</b> in <b>MimIR</b>
</p>

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub Release](https://img.shields.io/github/v/release/ashiven/eqsat)](https://github.com/ashiven/eqsat/releases)
[![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/ashiven/eqsat)](https://github.com/ashiven/eqsat/issues)
[![GitHub Issues or Pull Requests](https://img.shields.io/github/issues-pr/ashiven/eqsat)](https://github.com/ashiven/eqsat/pulls)

</div>

**Equality Saturation** is a compiler optimization technique that is primarily used to solve the [Phase-Ordering Problem](https://www2.imm.dtu.dk/pubdb/edoc/imm5406.pdf) for compiler optimization passes. It utilizes [E-Graphs](https://en.wikipedia.org/wiki/E-graph#Equality_saturation) to simultaneously represent a set of equivalent program terms according to a set of rewrite-rules and find the most optimal one according to a cost heuristic. This repository contains **Equality Saturation** implementations in [egg](https://github.com/egraphs-good/egg) and [slotted-egraphs](https://github.com/memoryleak47/slotted-egraphs) as a plugin for the functional higher-order intermediate representation [MimIR](https://github.com/mimir/mimir).

## Table of Contents

- [Usage](#usage)
  - [C++ API](#c-api)
  - [Mim](#mim)
- [Installation](#installation)
- [Rulesets](#rulesets)
- [Provided Methods](#provided-methods)
- [Contributing](#contributing)
- [License](#license)

## Usage

You may use this plugin through the **MimIR** C++ API or its textual representation **Mim**.
Consider the following lightweight examples to get started. The examples both perform the same
optimization:

- Define a rewrite-rule `?n + 0 => ?n`
- Define a term `fun(x: Nat): Nat = return (x + 0);`
- Perform equality saturation in `slotted-egraphs`
- Extract an optimal term by smallest `AstSize`

### C++ API

```cpp
#include <fstream>
#include <mim/driver.h>
#include <mim/ast/parser.h>
#include <mim/pass/optimize.h>
#include <mim/util/sys.h>
#include <mim/plug/eqsat/eqsat.h>

using namespace mim;
using namespace mim::plug;

int main(int, char**) {
    try {
        auto driver = Driver("eqsat");
        auto& w     = driver.world();
        driver.log().set(&std::cerr).set(Log::Level::Debug);
        ast::load_plugins(w, View<std::string>{"core", "ll", "eqsat"});

        // rule foo (x: Nat): %core.nat.add (x, 0) => x;
        auto foo = w.mut_rule(w.type_nat())->set("foo");
        auto x = foo->var()->set("x");
        auto lhs = w.call(core::nat::add, w.tuple(x, lit_nat(0)))
        auto rhs = x;
        foo->set_lhs(lhs);
        foo->set_rhs(rhs);
        foo->set_guard(w.lit_tt());

        // Quickly define config values
        eqsat_config(
            w,
            eqsat::slotted,
            eqsat::AstSize,
            eqsat_rulesets(eqsat::standard),
            eqsat_rules(foo),
        );   

        // fun extern main(x: Nat): Nat = return %core.nat.add (x, 0);
        auto main   = w.mut_fun({w.type_nat()}, {w.type_nat()})->set("main");
        auto x = main->var(2, 0)->set("x");
        auto ret               = main->var(2, 1);
        main->app(false, ret, x);
        main->externalize();

        // Equality saturation and code gen are performed here
        optimize(w);

        sys::system("clang eqsat.ll -o eqsat -Wno-override-module");
        std::println("exit code: {}", sys::system("./eqsat"));
    } catch (const std::exception& e) {
        std::println(std::cerr, "{}", e.what());
        return EXIT_FAILURE;
    } catch (...) {
        std::println(std::cerr, "error: unknown exception");
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
```

### Mim

```
plugin core;
plugin eqsat;

// You can define your own syntactic rewrite-rules here
rule foo (x: Nat): %core.nat.add (x, 0) => x;

lam extern _config() =
    %eqsat.config (
        // Specifies whether the plugin should use its egg or slotted-egraphs backend
        %eqsat.slotted,

        // Defines the cost function that should be used for term extraction
        %eqsat.AstSize,

        // Specifies a set of rules directly implemented in egg or slotted-egraphs
        // To implement and use your own ruleset, follow the instructions under **Rulesets**.
        %eqsat.rulesets (%eqsat.normalize),

        // To use the rule 'foo' that we defined above for equality saturation
        %eqsat.rules (foo),
        
        // Here you may provide two terms to assert whether term A can reach term B in a number of steps
        %eqsat.reaches (term_A, term_B, 10),

        // Here you may select specific terms that should be rewritten
        // When providing an empty tuple, no terms will be rewritten
        %eqsat.select (),
    );

fun extern main(x: Nat): Nat =
    return %core.nat.add (x, 0);
```

## Installation

To install this plugin simply follow the instructions below:

1. Clone the `mimir` repository

```bash
git clone --recursive https://github.com/mimir/mimir.git
```

2. Clone the `eqsat` repository

```bash
cd mimir/extra
git clone https://github.com/ashiven/eqsat.git
cd ..
```

3. Ensure that Rust and Cargo are installed

```bash
curl https://sh.rustup.rs -sSf | sh
```

4. Build the project

```bash
cmake -S . -B build -DBUILD_TESTING=ON -DMIM_BUILD_EXAMPLES=ON
cmake --build build -j$(nproc)
```

## Rulesets

You may want to define a set of rewrite-rules that are more complex than the syntactic rewrite-rules
that can be defined in **MimIR**. In this case, you should follow the implementation guide below on adding
a set of rules directly in **egg** or **slotted-egraphs**.

To automatically generate all of the boilerplate code shown below, use the following script:

```bash
python ./scripts/new_ruleset.py egg MyRules
```

1. Define a set of rules in `src/egg/rulesets/myrules.rs`

```rust
use crate::egg::{Mim, analysis::MimAnalysis};
use egg::{Rewrite, Pattern};

pub fn rules() -> Vec<Rewrite<Mim, MimAnalysis>> {
    let rules = vec![
        my_rule(),
    ];
    rules
}

fn my_rule() -> Rewrite<Mim, MimAnalysis> {
    let pat: Pattern<Mim> = "(app %foo.bar ?baz)".parse().unwrap();
    let outpat: Pattern<Mim> = "?baz".parse().unwrap();
    Rewrite::new("my-rule", pat, outpat).unwrap()
}
```

2. Add your ruleset to the `RuleSet` enum in `src/ffi/bridge.rs`

```rust
// ...
#[cxx::bridge]
pub mod bridge {
    #[derive(Debug)]
    enum RuleSet {
        // Egg
        Core,
        MyRules,
        // Slotted
        Standard,
    }
// ...
```

3. Ensure that your ruleset is registered in `src/egg/rulesets/mod.rs`

```rust
use crate::RuleSet;
use crate::egg::{Mim, analysis:MimAnalysis};
use egg::Rewrite;

pub mod core;
pub mod myrules;

pub fn get_rules(rulesets: Vec<RuleSet>) -> Vec<Rewrite<Mim, MimAnalysis>> {
    let mut rules = Vec::new();
    for ruleset in rulesets {
        match ruleset {
            RuleSet::Core => rules.extend(core::rules()),
            RuleSet::MyRules => rules.extend(myrules::rules()),
            _ => (),
        }
    }
    rules
}
```

4. Add your ruleset as a new axiom to `eqsat.mim`

```
/// ...
/// ## Rulesets
///
/// ### Egg
///
axm %eqsat.core: %eqsat.Ruleset;
axm %eqsat.myrules: %eqsat.Ruleset;
///
/// ### Slotted
///
axm %eqsat.standard: %eqsat.Ruleset;
/// ...
```

5. Patch the rewrite phase in `plug/phase/rewrite_egg.cpp`

```cpp
// ...
for (auto ruleset : ruleset_config->args())
    if (Axm::isa<eqsat::core>(ruleset))
        rulesets.push_back(RuleSet::Core);
    else if (Axm::isa<eqsat::myrules>(ruleset))
        rulesets.push_back(RuleSet::MyRules);
// ...
```

## Provided Methods

This library also exposes its methods in a C++ FFI, which 
was required to integrate it into the **MimIR** plugin system. 
The following documents the signatures generated for these methods via [CXX](https://cxx.rs)
along with a short description of what they do.

### Rewriting

```cpp
/**
 *  Rewrites an sexpr in `egg` format
 *
 *  sexpr:     a symbolic expr in `egg` format (emitted by the `mim` compiler via `--output-sexpr`)
 *  selected:  optionally, a list of identifiers for terms that should be rewritten
 *  rulesets:  provides a list of identifiers to rulesets that should be used for rewriting (see src/egg/rulesets)
 *  cost_fn:   provides a cost function that should be used for extraction (currently only AstSize and AstDepth)
 */
rust::Vec<RecExprFFI> eqsat_egg(rust::Str sexpr, OptionSelected selected, rust::Vec<RuleSet> rulesets, CostFn cost_fn);
```

```cpp
/**
 *  Rewrites an sexpr in `slotted-egraphs` format
 *
 *  sexpr:     a symbolic expr in `slotted-egraphs` format (emitted by the `mim` compiler via `--slotted --output-sexpr`)
 *  selected:  optionally, a list of identifiers for terms that should be rewritten
 *  rulesets:  provides a list of identifiers to rulesets that should be used for rewriting (see src/mim_slotted/rulesets)
 *  cost_fn:   provides a cost function that should be used for extraction (currently only AstSize)
 */
rust::Vec<RecExprFFI> eqsat_slotted(rust::Str sexpr, OptionSelected selected, rust::Vec<RuleSet> rulesets, CostFn cost_fn);
```

### Proving equivalence

```cpp
/**
 *  Uses `slotted-egraphs` to prove whether two terms are equivalent
 *
 *  sexpr:      a symbolic expr in `slotted-egraphs` format (emitted by the `mim` compiler via `--slotted --output-sexpr`)
 *  rulesets:   provides a list of identifiers to rulesets that should be used for rewriting (see src/mim_slotted/rulesets)
 *  start_name: an identifier for the starting term
 *  end_name:   an identifier for the end term that the start term should reach via rewriting
 *  max_steps:  the maximum number of iterations in which the start term should reach the end term
 */
bool reaches_egg(rust::Str sexpr, rust::Vec<RuleSet> rulesets, rust::Str start_name, rust::Str end_name, std::size_t max_steps);
```

```cpp
/**
 *  Uses `egg` to prove whether two terms are equivalent
 *
 *  sexpr:      a symbolic expr in `slotted-egraphs` format (emitted by the `mim` compiler via `--slotted --output-sexpr`)
 *  rulesets:   provides a list of identifiers to rulesets that should be used for rewriting (see src/mim_slotted/rulesets)
 *  start_name: an identifier for the starting term
 *  end_name:   an identifier for the end term that the start term should reach via rewriting
 *  max_steps:  the maximum number of iterations in which the start term should reach the end term
 */
bool reaches_slotted(rust::Str sexpr, rust::Vec<::RuleSet> rulesets, rust::Str start_name, rust::Str end_name, std::size_t max_steps);
```

### Pretty-printing

```cpp
/**
 *  Pretty-prints an sexpr in `egg` format
 *
 *  sexpr:     a symbolic expr in `egg` format (emitted by the `mim` compiler via `--output-sexpr`)
 *  line_len:  the maximal line length after which the sexpr continues on a new line
 */
rust::String pretty_egg(rust::Str sexpr, std::size_t line_len);
```

```cpp
/**
 *  Pretty-prints an sexpr in `slotted-egraphs` format
 *
 *  sexpr:     a symbolic expr in `slotted-egraphs` format (emitted by the `mim` compiler via `--slotted --output-sexpr`)
 *  line_len:  the maximal line length after which the sexpr continues on a new line
 */
rust::String pretty_slotted(rust::Str sexpr, std::size_t line_len);
```

```cpp
/**
 *  Pretty-prints an sexpr represented by a Vec<RecExprFFI>
 *
 *  sexprs:    a vector of symbolic expressions in RecExprFFI format (the result of equality saturation)
 *  line_len:  the maximal line length after which the sexpr continues on a new line
 */
rust::String pretty_ffi(rust::Vec<RecExprFFI> sexprs, std::size_t line_len);
```

## Contributing

Please feel free to submit a [pull request](https://github.com/ashiven/cs2tracker/pulls) or open an [issue](https://github.com/ashiven/cs2tracker/issues).

1. Fork the repository
2. Create a new branch: `git checkout -b feature-name`.
3. Make your changes
4. Push your branch: `git push origin feature-name`.
5. Submit a PR

## License

This project is licensed under the [MIT License](./LICENSE).

---

> GitHub [@ashiven](https://github.com/Ashiven) &nbsp;&middot;&nbsp;
> Twitter [ashiven\_](https://twitter.com/ashiven_)
