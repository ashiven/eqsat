#include <cstdint>

#include <mim/plug/eqsat/eqsat.h>
#include <mim/plug/eqsat/phase/rewrite_egg.h>

#include "mim/def.h"
#include "mim/driver.h"

const bool DEBUG = false;

namespace mim::plug::eqsat {

void RewriteEgg::start() {
    auto [rulesets, cost_fn] = import_config();

    // We are assuming that the core plugin and its backends have been loaded at this point
    // because the 'eqsat' plugin declared it as a dependency via 'plugin core;'
    std::ostringstream sexpr;
    driver().backend("sexpr")(old_world(), sexpr);

    if (DEBUG) std::cout << sexpr.str() << "\n";

    auto rewrites = eqsat_egg(sexpr.str(), rulesets, cost_fn);

    if (DEBUG) std::cout << pretty_ffi(rewrites, 80).c_str() << "\n";

    init(rewrites, InitStage::Declarations);
    init(rewrites, InitStage::Lambdas);
    init(rewrites, InitStage::Bindings);
    convert(rewrites);

    swap(old_world(), new_world());
}

std::pair<rust::Vec<RuleSet>, CostFn> RewriteEgg::import_config() {
    // Internalize eqsat config lambdas (lam with signature [] -> %eqsat.Config | <<n; %eqsat.Config>>)
    DefVec lams;
    for (auto def : old_world().externals().mutate()) {
        if (auto lam = def->isa<Lam>()) {
            if (auto arr = lam->codom()->isa<Arr>();
                (arr && Axm::isa<eqsat::Config>(arr->body())) || Axm::isa<eqsat::Config>(lam->codom())) {
                lams.push_back(lam);
                def->internalize();
            }
        }
    }

    // Import predefined rulesets and cost function from config lambdas
    rust::Vec<RuleSet> rulesets;
    CostFn cost_fn = CostFn::AstSize;
    for (auto lam : lams) {
        auto body               = lam->as<Lam>()->body();
        DefVec singleton_config = {body};
        auto config_vals        = body->isa<Tuple>() ? body->as<Tuple>()->ops() : Defs(singleton_config);
        for (auto config_val : config_vals) {
            if (auto ruleset_config = Axm::isa<eqsat::rulesets>(config_val)) {
                for (auto ruleset : ruleset_config->args())
                    if (Axm::isa<eqsat::core>(ruleset))
                        rulesets.push_back(RuleSet::Core);
                    else if (Axm::isa<eqsat::math>(ruleset))
                        rulesets.push_back(RuleSet::Math);
                    // AUTOGEN START: egg-ruleset-cpp
                    // AUTOGEN END: egg-ruleset-cpp
                    else
                        assert(false && "Provided ruleset does not exist for egg");

            } else if (auto reaches = Axm::isa<eqsat::reaches>(config_val)) {
                continue;

            } else if (Axm::isa<eqsat::AstSize>(config_val)) {
                cost_fn = CostFn::AstSize;
            } else if (Axm::isa<eqsat::AstDepth>(config_val)) {
                cost_fn = CostFn::AstDepth;

            } else if (auto select = Axm::isa<eqsat::select>(config_val)) {
                continue;

            } else if (Axm::isa<eqsat::rules>(config_val) || Axm::isa<eqsat::rules_kind>(config_val)) {
                auto dom       = old_world().sigma();
                auto codom     = old_world().annex<eqsat::Config>();
                auto rules_lam = old_world().mut_lam(dom, codom)->set("_rules");
                rules_lam->set_filter(false);
                rules_lam->set_body(config_val);
                rules_lam->externalize();

            } else if (Axm::isa<eqsat::slotted>(config_val) || Axm::isa<eqsat::egg>(config_val)) {
                continue;

            } else {
                assert(false && "Invalid config value provided for egg");
            }
        }
    }

    return {rulesets, cost_fn};
}

// Initially creates Defs in the new world according to the specfied 'InitStage'
// This is done in a particular order to ensure that dependencies are upheld.
// Defs are initialized in this order: (Declarations->Lambdas->Let-bindings)
// The bodies of lambdas can only be created and then set inside of convert(...)
// after every Def from the init stage has been created, because they
// can depend on any declaration, lambda, or let-binding.
void RewriteEgg::init(rust::Vec<RecExprFFI> rewrites, InitStage stage) {
    for (auto rewrite : rewrites) {
        added_ = {};
        res_   = rewrite.nodes;
        for (uint32_t id = res_.size() - 1; id > 0; id--) {
            auto node      = get_node_unsafe(id);
            const Def* res = nullptr;
            switch (node.kind) {
                case MimKind::Axm: res = stage == InitStage::Declarations ? init_axm(id, node) : nullptr; break;
                case MimKind::Fun:
                case MimKind::Con:
                case MimKind::Lam: res = stage == InitStage::Lambdas ? init_lam(id, node) : nullptr; break;
                case MimKind::Let: res = stage == InitStage::Bindings ? init_let(id, node) : nullptr; break;
                default: break;
            }
            added_[id] = res;
        }
    }
}

// (lam <extern> <name> <var> <domain> <codomain> [<filter> <body>])
const Def* RewriteEgg::init_lam(uint32_t id, NodeFFI node) {
    if (DEBUG) std::cout << "init - current node(" << id << "): " << node_ffi_str(node).c_str() << " - ";

    auto domain         = convert(node.children[3], true);
    auto codomain       = convert(node.children[4], true);
    auto new_lam        = new_world().mut_lam(domain, codomain);
    auto lam_name       = get_symbol(node.children[1]);
    auto lam_name_nouid = remove_uid(lam_name);
    new_lam->set(lam_name_nouid);
    register_lam(lam_name, new_lam);

    // 'var_node' is a var node so index 0 contains its name and back() its type.
    // For reference: (var <name> [<proj1> <proj2>...] <type>)
    auto var_node       = get_node(MimKind::Var, node.children[2]);
    auto var_name       = get_symbol(var_node.children[0]);
    auto var_name_nouid = remove_uid(var_name);
    auto var            = new_lam->var();
    var->set(var_name_nouid);
    register_var(var_name, var);
    register_projs(node.children[2]);

    if (DEBUG) std::cout << new_lam << "\n";
    return new_lam;
}

// (let <name> <definition> <expression>)
const Def* RewriteEgg::init_let(uint32_t id, NodeFFI node) {
    if (DEBUG) std::cout << "init - current node(" << id << "): " << node_ffi_str(node).c_str() << " - ";
    // If the let-binding is for a lambda, this lambda will already have been
    // created, set and registered via init_lam/con and thus we can skip it.
    auto let_def = get_def(node.children[0]);
    if (let_def) return nullptr;

    auto name       = get_symbol(node.children[0]);
    auto name_nouid = remove_uid(name);
    auto def        = convert(node.children[1], true);
    def->set(name_nouid);
    register_var(name, def);

    if (DEBUG) std::cout << def << "\n";
    return nullptr;
}

// (axm <name> <type>)
const Def* RewriteEgg::init_axm(uint32_t id, NodeFFI node) {
    if (DEBUG) std::cout << "init - current node(" << id << "): " << node_ffi_str(node).c_str() << " - ";
    auto name = get_symbol(node.children[0]);
    auto type = convert(node.children[1], true);

    auto new_axm = new_world().axm(type);
    new_axm->set(name);
    register_axm(name, new_axm);

    if (DEBUG) std::cout << new_axm << "\n";
    return new_axm;
}

// Converts remaining nodes to Def's in the new world and sets the bodies of the previously created lambdas.
void RewriteEgg::convert(rust::Vec<RecExprFFI> rewrites) {
    for (auto rewrite : rewrites) {
        added_ = {};
        res_   = rewrite.nodes;
        for (uint32_t id = 0; id < res_.size(); id++)
            convert(id);
    }
}

const Def* RewriteEgg::convert(uint32_t id, bool recurse) {
    auto node = get_node_unsafe(id);

    if (recurse)
        for (uint32_t child : node.children)
            convert(child, recurse);

    const Def* res = added_[id];
    if (res && node.kind != MimKind::Con && node.kind != MimKind::Lam) return res;

    if (DEBUG) std::cout << "convert - current node(" << id << "): " << node_ffi_str(node).c_str() << " - ";
    switch (node.kind) {
        case MimKind::Let: res = convert_let(id, node); break;
        case MimKind::Fun:
        case MimKind::Con:
        case MimKind::Lam: res = convert_lam(id, node); break;
        case MimKind::App: res = convert_app(id, node); break;
        case MimKind::Var: res = convert_var(id, node); break;
        case MimKind::Lit: res = convert_lit(id, node); break;
        case MimKind::Pack: res = convert_pack(id, node); break;
        case MimKind::Tuple: res = convert_tuple(id, node); break;
        case MimKind::Extract: res = convert_extract(id, node); break;
        case MimKind::Insert: res = convert_insert(id, node); break;
        case MimKind::Inj: res = convert_inj(id, node); break;
        case MimKind::Merge: res = convert_merge(id, node); break;
        case MimKind::Match: res = convert_match(id, node); break;
        case MimKind::Proxy: res = convert_proxy(id, node); break;
        case MimKind::Join: res = convert_join(id, node); break;
        case MimKind::Meet: res = convert_meet(id, node); break;
        case MimKind::Bot: res = convert_bot(id, node); break;
        case MimKind::Top: res = convert_top(id, node); break;
        case MimKind::Arr: res = convert_arr(id, node); break;
        case MimKind::Sigma: res = convert_sigma(id, node); break;
        case MimKind::Fn:
        case MimKind::Cn:
        case MimKind::ImplicitPi:
        case MimKind::Pi: res = convert_pi(id, node); break;
        case MimKind::Idx: res = convert_idx(id, node); break;
        case MimKind::Hole: res = convert_hole(id, node); break;
        case MimKind::Type: res = convert_type(id, node); break;
        case MimKind::Num: res = convert_num(id, node); break;
        case MimKind::Symbol: res = convert_symbol(id, node); break;
        default: break;
    }

    if (DEBUG) std::cout << res << "\n";
    return added_[id] = res;
}

// (let <name> <definition> <expression>)
const Def* RewriteEgg::convert_let(uint32_t id, NodeFFI node) {
    auto expr = get_def(node.children[2]);
    return expr;
}

// (lam <extern> <name> <var> <domain> <codomain> [<filter> <body>])
const Def* RewriteEgg::convert_lam(uint32_t id, NodeFFI node) {
    auto lam = get_def(node.children[1])->as_mut<Lam>();

    if (node.children.size() == 7) {
        auto filter = get_def(node.children[5]);
        auto body   = get_def(node.children[6]);
        lam->set_filter(filter);
        lam->set_body(body);
    } else
        lam->set_filter(false);

    auto is_extern = get_symbol(node.children[0]);
    if (is_extern == "extern") lam->externalize();

    return lam;
}

// (app <callee> <arg>)
const Def* RewriteEgg::convert_app(uint32_t id, NodeFFI node) {
    auto callee  = get_def(node.children[0]);
    auto arg     = get_def(node.children[1]);
    auto new_app = new_world().app(callee, arg);
    return new_app;
}

// (var <name> [<proj1> <proj2> ...] <type>)
const Def* RewriteEgg::convert_var(uint32_t id, NodeFFI node) {
    auto var = get_def(node.children[0]);
    return var;
}

// (lit <val> <type>)
const Def* RewriteEgg::convert_lit(uint32_t id, NodeFFI node) {
    auto lit_def = get_def(node.children[0]);
    if (lit_def) return lit_def;

    auto lit_val  = get_num(node.children[0]);
    auto lit_type = get_def(node.children[1]);
    auto new_lit  = new_world().lit(lit_type, lit_val);
    return new_lit;
}

// (pack <var> <arity> <body>)
const Def* RewriteEgg::convert_pack(uint32_t id, NodeFFI node) {
    auto arity    = get_def(node.children[1]);
    auto body     = get_def(node.children[2]);
    auto new_pack = new_world().pack(arity, body);
    return new_pack;
}

// (tuple <node1> <node2> ...)
const Def* RewriteEgg::convert_tuple(uint32_t id, NodeFFI node) {
    DefVec ops;
    for (auto child : node.children) {
        auto op = get_def(child);
        ops.push_back(op);
    }
    auto new_tuple = new_world().tuple(ops);
    return new_tuple;
}

// (extract <tuple> <index>)
const Def* RewriteEgg::convert_extract(uint32_t id, NodeFFI node) {
    auto tuple       = get_def(node.children[0]);
    auto index       = get_def(node.children[1]);
    auto new_extract = new_world().extract(tuple, index);
    return new_extract;
}

// (ins <tuple> <index> <value>)
const Def* RewriteEgg::convert_insert(uint32_t id, NodeFFI node) {
    auto tuple      = get_def(node.children[0]);
    auto index      = get_def(node.children[1]);
    auto value      = get_def(node.children[2]);
    auto new_insert = new_world().insert(tuple, index, value);
    return new_insert;
}

// (inj <type> <value>)
const Def* RewriteEgg::convert_inj(uint32_t id, NodeFFI node) {
    auto type    = get_def(node.children[0]);
    auto value   = get_def(node.children[1]);
    auto new_inj = new_world().inj(type, value);
    return new_inj;
}

// (merge <type> <value1> <value2> ...)
const Def* RewriteEgg::convert_merge(uint32_t id, NodeFFI node) {
    auto type = get_def(node.children[0]);
    DefVec values;
    for (auto child : node.children | std::views::drop(1)) {
        auto value = get_def(child);
        values.push_back(value);
    }
    auto new_merge = new_world().merge(type, values);
    return new_merge;
}

// (match <scrutinee> <arm1> <arm2> ...)
const Def* RewriteEgg::convert_match(uint32_t id, NodeFFI node) {
    auto scrutinee = get_def(node.children[0]);
    DefVec ops     = {scrutinee};
    for (auto child : node.children | std::views::drop(1)) {
        auto arm = get_def(child);
        ops.push_back(arm);
    }
    auto new_match = new_world().match(ops);
    return new_match;
}

// (proxy <type> <pass> <tag> <op1> <op2> ...)
const Def* RewriteEgg::convert_proxy(uint32_t id, NodeFFI node) {
    auto type = get_def(node.children[0]);
    auto pass = get_num(node.children[1]);
    auto tag  = get_num(node.children[2]);
    DefVec ops;
    for (auto child : node.children | std::views::drop(3)) {
        auto op = get_def(child);
        ops.push_back(op);
    }
    auto new_proxy = new_world().proxy(type, ops, pass, tag);
    return new_proxy;
}

// (join <type1> <type2> ...)
const Def* RewriteEgg::convert_join(uint32_t id, NodeFFI node) {
    DefVec types;
    for (auto child : node.children) {
        auto type = get_def(child);
        if (type) types.push_back(type);
    }
    if (types.size() > 0) {
        auto new_join = new_world().join(types);
        return new_join;
    }
    return nullptr;
}

// (meet <type1> <type2> ...)
const Def* RewriteEgg::convert_meet(uint32_t id, NodeFFI node) {
    DefVec types;
    for (auto child : node.children) {
        auto type = get_def(child);
        if (type) types.push_back(type);
    }
    if (types.size() > 0) {
        auto new_meet = new_world().meet(types);
        return new_meet;
    }
    return nullptr;
}

// (bot <type>)
const Def* RewriteEgg::convert_bot(uint32_t id, NodeFFI node) {
    auto type    = get_def(node.children[0]);
    auto new_bot = new_world().bot(type);
    return new_bot;
}

// (top <type>)
const Def* RewriteEgg::convert_top(uint32_t id, NodeFFI node) {
    auto type    = get_def(node.children[0]);
    auto new_top = new_world().top(type);
    return new_top;
}

// (arr <var> <arity> <body>)
const Def* RewriteEgg::convert_arr(uint32_t id, NodeFFI node) {
    auto arity   = get_def(node.children[1]);
    auto body    = get_def(node.children[2]);
    auto new_arr = new_world().arr(arity, body);
    return new_arr;
}

// (sigma <var> <type1> <type2> ...)
const Def* RewriteEgg::convert_sigma(uint32_t id, NodeFFI node) {
    DefVec types;
    for (auto child : node.children | std::views::drop(1)) {
        auto type = get_def(child);
        types.push_back(type);
    }

    auto new_sigma = new_world().sigma(types);
    return new_sigma;
}

// (cn <domain>)
const Def* RewriteEgg::convert_cn(uint32_t id, NodeFFI node) {
    auto domain = get_def(node.children[0]);
    auto new_cn = new_world().cn(domain);
    return new_cn;
}

// (pi <var> <domain> <codomain>)
const Def* RewriteEgg::convert_pi(uint32_t id, NodeFFI node) {
    auto domain   = get_def(node.children[1]);
    auto codomain = get_def(node.children[2]);
    auto implicit = node.kind == MimKind::ImplicitPi;
    auto new_pi   = new_world().pi(domain, codomain, implicit);
    return new_pi;
}

// (idx <size>)
const Def* RewriteEgg::convert_idx(uint32_t id, NodeFFI node) {
    auto size    = get_def(node.children[0]);
    auto new_idx = new_world().type_idx(size);
    return new_idx;
}

// (hole <type>)
const Def* RewriteEgg::convert_hole(uint32_t id, NodeFFI node) {
    auto type_    = get_def(node.children[0]);
    auto new_hole = new_world().mut_hole(type_);
    return new_hole;
}

// (type <level>)
const Def* RewriteEgg::convert_type(uint32_t id, NodeFFI node) {
    auto level    = get_def(node.children[0]);
    auto new_type = new_world().type(level);
    return new_type;
}

// <i64>
const Def* RewriteEgg::convert_num(uint32_t id, NodeFFI node) { return nullptr; }

// <string>
const Def* RewriteEgg::convert_symbol(uint32_t id, NodeFFI node) {
    auto def = get_def(id);
    return def;
}

} // namespace mim::plug::eqsat
