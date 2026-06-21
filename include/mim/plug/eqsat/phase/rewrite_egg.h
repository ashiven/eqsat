#pragma once

#include <mim/phase.h>

#include <mim/plug/eqsat/phase/util.h>

#include "mim/def.h"
#include "mim/rewrite.h"

#include "rust/eqsat_rs.h"

namespace mim::plug::eqsat {

/***************** TYPES **********************/
typedef std::vector<std::tuple<std::string, std::string, size_t>> ReachesArgs;
typedef rust::Vec<RuleSet> RuleSets;
typedef std::tuple<RuleSets, CostFn, ReachesArgs, OptionSelected> ConfigValues;

typedef fe::SymMap<const Def*> Sym2Def;
typedef absl::flat_hash_map<uint32_t, const Def*> Cache;
typedef rust::Vec<NodeFFI> Nodes;

typedef struct RecExprState {
    Cache cache;
    Nodes nodes;
} RecExprState;

typedef struct Context {
    size_t id;
    RecExprState* state;
} Context;

typedef absl::flat_hash_map<size_t, RecExprState> RecExprStates;

/***************** REWRITER *********************/
class RewriteEgg : public Phase, public Rewriter {
public:
    RewriteEgg(World& world, std::string name)
        : Phase(world, std::move(name))
        , Rewriter(world.inherit()) {
        register_symbols();
    }
    RewriteEgg(World& world, flags_t annex)
        : Phase(world, annex)
        , Rewriter(world.inherit()) {
        register_symbols();
    }

    void start() override;

    using Phase::world;
    using Rewriter::world;
    World& world() = delete;
    World& old_world() { return Phase::world(); }
    World& new_world() { return Rewriter::world(); }

private:
    void register_symbols() {
        for (auto [flags, e] : old_world().annexes()) {
            auto new_annex          = new_world().annexes().attach(flags, e.sym, rewrite(e.def));
            axms_[new_annex->sym()] = new_annex;
        }

        aliases_[new_world().sym("Univ")] = new_world().univ();
        aliases_[new_world().sym("Bool")] = new_world().type_bool();
        aliases_[new_world().sym("Nat")]  = new_world().type_nat();
        aliases_[new_world().sym("I8")]   = new_world().type_i8();
        aliases_[new_world().sym("I16")]  = new_world().type_i16();
        aliases_[new_world().sym("I32")]  = new_world().type_i32();
        aliases_[new_world().sym("I64")]  = new_world().type_i64();
        aliases_[new_world().sym("ff")]   = new_world().lit_ff();
        aliases_[new_world().sym("tt")]   = new_world().lit_tt();
        aliases_[new_world().sym("i8")]   = new_world().lit_nat(0x100);
        aliases_[new_world().sym("i16")]  = new_world().lit_nat(0x10000);
        aliases_[new_world().sym("i32")]  = new_world().lit_nat(0x100000000);
    }

    bool swap_world_unchanged(OptionSelected selected);
    ConfigValues import_config();

    // Asserts whether a start term can reach an
    // end term in a given number of steps using the
    // provided sets of rules.
    void assert_reaches(std::string sexpr, RuleSets rulesets, ReachesArgs reaches_args);

    // NodeFFI can carry a type that is also in the form
    // of a RecExprFFI. We convert this type with a top-down
    // traversal for creating binders followed by a bottom-up
    // traversal to create the remaining Def's
    const Def* create_type(RecExprFFI type_);

    // Performs a top-down traverse of each RecExprFFI
    // and creates and stores all bindings with their definitions.
    // Lambdas are created without their bodies in this phase.
    void init(rust::Vec<RecExprFFI> rec_exprs);
    const Def* init(uint32_t id);
    const Def* init_lookahead(uint32_t id);
    const Def* init_axm(uint32_t id, NodeFFI node);
    const Def* init_root(uint32_t id, NodeFFI node);
    const Def* init_let(uint32_t id, NodeFFI node);
    const Def* init_lam(uint32_t id, NodeFFI node);
    const Def* init_pi(uint32_t id, NodeFFI node);
    const Def* init_sigma(uint32_t id, NodeFFI node);
    const Def* init_arr(uint32_t id, NodeFFI node);
    const Def* init_pack(uint32_t id, NodeFFI node);

    // Performs a bottom-up traverse of each RecExprFFI and
    // creates a Def in the new_world() for every node.
    // At this point, the bodies of the lambdas created
    // in the init phase will be set.
    void convert(rust::Vec<RecExprFFI> rec_exprs);
    const Def* convert(uint32_t id);
    const Def* convert_root(uint32_t id, NodeFFI node);
    const Def* convert_let(uint32_t id, NodeFFI node);
    const Def* convert_lam(uint32_t id, NodeFFI node);
    const Def* convert_app(uint32_t id, NodeFFI node);
    const Def* convert_var(uint32_t id, NodeFFI node);
    const Def* convert_lit(uint32_t id, NodeFFI node);
    const Def* convert_pack(uint32_t id, NodeFFI node);
    const Def* convert_tuple(uint32_t id, NodeFFI node);
    const Def* convert_extract(uint32_t id, NodeFFI node);
    const Def* convert_insert(uint32_t id, NodeFFI node);
    const Def* convert_inj(uint32_t id, NodeFFI node);
    const Def* convert_merge(uint32_t id, NodeFFI node);
    const Def* convert_match(uint32_t id, NodeFFI node);
    const Def* convert_proxy(uint32_t id, NodeFFI node);
    const Def* convert_join(uint32_t id, NodeFFI node);
    const Def* convert_meet(uint32_t id, NodeFFI node);
    const Def* convert_bot(uint32_t id, NodeFFI node);
    const Def* convert_top(uint32_t id, NodeFFI node);
    const Def* convert_arr(uint32_t id, NodeFFI node);
    const Def* convert_sigma(uint32_t id, NodeFFI node);
    const Def* convert_cn(uint32_t id, NodeFFI node);
    const Def* convert_pi(uint32_t id, NodeFFI node);
    const Def* convert_idx(uint32_t id, NodeFFI node);
    const Def* convert_hole(uint32_t id, NodeFFI node);
    const Def* convert_type(uint32_t id, NodeFFI node);
    const Def* convert_num(uint32_t id, NodeFFI node);
    const Def* convert_symbol(uint32_t id, NodeFFI node);

    Context& ctx() { return ctx_; }

    size_t id() { return ctx().id; }
    void set_id(size_t id) { ctx().id = id; }

    void switch_context(size_t id) {
        set_id(id);
        set_state(id);
    }

    void switch_context(Context& other) {
        set_id(other.id);
        set_state(other.id);
    }

    RecExprState* state() { return ctx().state; }
    void set_state(size_t id) { ctx().state = &states_[id]; }

    Nodes& nodes() { return state()->nodes; }
    void set_nodes(size_t id, Nodes nodes) { states_[id].nodes = nodes; }
    size_t root() { return nodes().size() - 1; }

    Cache& cache() { return state()->cache; }
    void set_cache(size_t id, Cache cache) { states_[id].cache = cache; }

    void init_state(size_t id, RecExprFFI& rec_expr) {
        set_cache(id, {});
        set_nodes(id, rec_expr.nodes);
    }

    const Def* get_def(uint32_t id) {
        auto def = cache()[id];
        if (!def) {
            auto sym = get_symbol(id);
            if (auto alias = get_alias(sym))
                def = alias;
            else if (auto axm = get_axm(sym))
                def = axm;
            else if (auto var = get_var(sym))
                def = var;
        }
        return def;
    }

    const Def* get_alias(Sym name) {
        auto it = aliases_.find(name);
        return it == aliases_.end() ? nullptr : it->second;
    }

    void register_var(Sym name, const Def* def) { vars_[name] = def; }
    const Def* get_var(Sym name) {
        auto it = vars_.find(name);
        return it == vars_.end() ? nullptr : it->second;
    }

    void register_axm(Sym name, const Axm* converted) {
        if (!axms_.contains(name)) axms_[name] = converted;
    }
    const Def* get_axm(Sym name) {
        auto it = axms_.find(name);
        return it == axms_.end() ? nullptr : it->second;
    }

    NodeFFI& get_node(MimKind expected, uint32_t id) {
        NodeFFI& node = nodes()[id];
        assert(node.kind == expected && "get_node: mismatch between expected and actual node kind");
        return node;
    }
    NodeFFI& get_node_unsafe(uint32_t id) { return nodes()[id]; }

    Sym get_symbol(uint32_t id) {
        auto node = nodes()[id];
        auto sym  = node.symbol.c_str();
        return new_world().sym(sym);
    }
    uint64_t get_num(uint32_t id) { return nodes()[id].num; }

    std::string remove_uid(std::string name) {
        if (auto pos = name.rfind("_"); pos != std::string::npos) {
            auto maybe_uid = name.substr(pos + 1);
            if (!maybe_uid.empty() && std::all_of(maybe_uid.begin(), maybe_uid.end(), ::isdigit))
                return name.substr(0, pos);
        }
        return name;
    }

    Sym2Def vars_;
    Sym2Def axms_;
    Sym2Def aliases_;
    Context ctx_;
    RecExprStates states_;
};

}; // namespace mim::plug::eqsat
