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

**Equality Saturation** is a compiler optimization technique that utilizes [E-Graphs](https://en.wikipedia.org/wiki/E-graph#Equality_saturation) to simultaneously represent a set of equivalent program terms according to a set of rewrite-rules and find their most optimal versions, thereby attempting to solve the [Phase-Ordering Problem](https://www2.imm.dtu.dk/pubdb/edoc/imm5406.pdf).
This repository contains **Equality Saturation** implementations in [egg](https://github.com/egraphs-good/egg) and [slotted-egraphs](https://github.com/memoryleak47/slotted-egraphs) as a plugin for the higher-order intermediate representation [MimIR](https://github.com/mimir/mimir).

## Table of Contents

- [Usage](#usage)
  - [C++ API](#option-1-c-api)
  - [Mim](#option-2-mim)
- [Installation](#installation)
- [Rulesets](#rulesets)
- [Provided Methods](#provided-methods)
- [Contributing](#contributing)
- [License](#license)

## Usage

You may use this plugin through the **MimIR** C++ API or its textual representation **Mim**.
Consider the following lightweight examples to get you started.

### Option 1: C++ API

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
        Driver driver;
        auto& w = driver.world();
        driver.log().set(&std::cerr).set(Log::Level::Debug);
        ast::load_plugins(w, View<std::string>{"compile", "core", "opt", "eqsat"});

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

        // Equality saturation is performed as part of optimization
        optimize(w);
        std::ofstream ofs("eqsat.ll");
        driver.backend("ll")(w, ofs);
        ofs.close();

        sys::system("clang eqsat.ll -o eqsat -Wno-override-module");
        outln("exit code: {}", sys::system("./eqsat"));
    } catch (const std::exception& e) {
        errln("{}", e.what());
        return EXIT_FAILURE;
    } catch (...) {
        errln("error: unknown exception");
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
```

### Option 2: Mim

```
plugin core;
plugin eqsat;

// You can define your own syntactic rewrite-rules in `MimIR`.
rule foo (x: Nat): %core.nat.add (x, 0) => x;

lam extern _config() =
    %eqsat.config (
        // Here you can specify whether the plugin should use its `egg` or `slotted-egraphs` backend.
        // The default implementation when nothing gets specified is `slotted`.
        // Note that the `egg` implementation is still incomplete and experimental.
        %eqsat.slotted,

        // To define the cost function that should be used for term extraction
        %eqsat.AstSize,

        // To use a set of rules directly implemented in `egg` or `slotted-egraphs`.
        // To see the existing rulesets, have a look at `src\mim_[egg|slotted]\rulesets`.
        // To implement and use your own ruleset, follow the instructions under **Rulesets**.
        %eqsat.rulesets (%eqsat.normalize),

        // To use the rule `foo` that we defined above
        %eqsat.rules (foo),
        
        // Here you may provide two terms to assert whether term A can reach term B in a number of steps.
        %eqsat.reaches (term_A, term_B, 10),

        // Here you may select specific terms that should be saturated.
        // When providing an empty tuple, no terms will be saturated.
        %eqsat.select (),
    );

fun extern main(x: Nat): Nat =
    return %core.nat.add (x, 0);
```

## Installation

To install this plugin simply follow the instructions below:

**1. Clone the `mimir` repository if you haven't already**

```bash
git clone --recursive https://github.com/mimir/mimir.git
```

**2. Clone the `eqsat` repository into `mimir/extra`**

```bash
cd mimir/extra
git clone https://github.com/ashiven/eqsat.git
cd ..
```

**3. Ensure that Rust and Cargo are installed**

```bash
curl https://sh.rustup.rs -sSf | sh
```

**4. Build the project according to the [instructions](https://mimir.github.io/index.html#autotoc_md92)**

```bash
cmake -S . -B build -DBUILD_TESTING=ON -DMIM_BUILD_EXAMPLES=ON
cmake --build build -j$(nproc)
```

## Rulesets

You may want to define a set of rewrite-rules that are more complex than the syntactic rewrite-rules
that can be defined in **MimIR**. In this case, you should follow the implementation guide below on adding
a set of rules directly in **egg** or **slotted-egraphs**. (The example defines a ruleset for **egg**)

To automatically generate all of the boilerplate code shown below, use the following script:

```bash
python ./scripts/new_ruleset.py egg myrules
```

**1. Define a set of rules in `src/mim_egg/rulesets/myrules.rs`**

```rust
use crate::mim_egg::{Mim, analysis::MimAnalysis};
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

**2. Add your ruleset to the RuleSet enum in `src/ffi.rs`**

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

**3. Ensure that your ruleset is registered in `src/mim_egg/rulesets/mod.rs`**

```rust
use crate::RuleSet;
use crate::mim_egg::Mim;
use crate::mim_egg::analysis::MimAnalysis;
use egg::Rewrite;

pub mod core;
// Add the module:
pub mod myrules;

pub fn get_rules(rulesets: Vec<RuleSet>) -> Vec<Rewrite<Mim, MimAnalysis>> {
    let mut rules = Vec::new();
    for ruleset in rulesets {
        match ruleset {
            RuleSet::Core => rules.extend(core::rules()),
            // Add the ruleset:
            RuleSet::MyRules => rules.extend(myrules::rules()),
            _ => (),
        }
    }
    rules
}
```

**4. Add your ruleset as a new axiom to `eqsat.mim`**

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

**5. Patch the rewrite phase in `plug/phase/rewrite_egg.cpp`**

```cpp
    // ...
    for (auto ruleset : ruleset_config->args())
        if (Axm::isa<eqsat::core>(ruleset))
            rulesets.push_back(RuleSet::Core);
        else if (Axm::isa<eqsat::math>(ruleset))
            rulesets.push_back(RuleSet::Math);
        // Add the ruleset:
        else if (Axm::isa<eqsat::myrules>(ruleset))
            rulesets.push_back(RuleSet::MyRules);
    // ...
}
```

## Provided Methods

There are two separate implementations in [egg](https://github.com/egraphs-good/egg) and [slotted-egraphs](https://github.com/memoryleak47/slotted-egraphs) that expose the following methods:

### Rewriting

```cpp
/**
 *  Rewrites an sexpr in `egg` format
 *
 *  sexpr:     a symbolic expr in `egg` format (emitted by the `mim` compiler via `--output-sexpr`)
 *  rulesets:  provides a list of identifiers to rulesets that should be used for rewriting (see src/mim_egg/rulesets)
 *  cost_fn:   provides a cost function that should be used for extraction (currently only AstSize and AstDepth)
 */
rust::Vec<RecExprFFI> eqsat_egg(std::string sexpr, rust::Vec<RuleSet> rulesets, CostFn cost_fn);
```

```cpp
/**
 *  Rewrites an sexpr in `slotted-egraphs` format
 *
 *  sexpr:     a symbolic expr in `slotted-egraphs` format (emitted by the `mim` compiler via `--slotted --output-sexpr`)
 *  rulesets:  provides a list of identifiers to rulesets that should be used for rewriting (see src/mim_slotted/rulesets)
 *  cost_fn:   provides a cost function that should be used for extraction (currently only AstSize)
 */
rust::Vec<RecExprFFI> eqsat_slotted(std::string sexpr, rust::Vec<RuleSet> rulesets, CostFn cost_fn);
```

### Pretty-printing

```cpp
/**
 *  Pretty-prints an sexpr in `egg` format
 *
 *  sexpr:     a symbolic expr in `egg` format (emitted by the `mim` compiler via `--output-sexpr`)
 *  line_len:  the maximal line length after which the sexpr continues on a new line
 */
rust::String pretty_egg(std::string sexpr, size_t line_len);
```

```cpp
/**
 *  Pretty-prints an sexpr in `slotted-egraphs` format
 *
 *  sexpr:     a symbolic expr in `slotted-egraphs` format (emitted by the `mim` compiler via `--slotted --output-sexpr`)
 *  line_len:  the maximal line length after which the sexpr continues on a new line
 */
rust::String pretty_slotted(std::string sexpr, size_t line_len);
```

```cpp
/**
 *  Pretty-prints an sexpr represented by a Vec<RecExprFFI>
 *
 *  sexprs:    a vector of symbolic expressions in RecExprFFI format (the result of equality saturation)
 *  line_len:  the maximal line length after which the sexpr continues on a new line
 */
rust::String pretty_ffi(rust::Vec<RecExprFFI> sexprs, size_t line_len);
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
