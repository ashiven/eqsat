#pragma once

#include <cstdint>

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

typedef struct Loc {
    int32_t depth;
    size_t offset;

    bool operator==(const Loc& other) const noexcept { return depth == other.depth && offset == other.offset; }

    std::string to_str() const {
        std::ostringstream os;
        os << "Loc{ depth=" << depth << ", offset=" << offset << " }";
        return os.str();
    }
} Loc;

typedef struct LocHash {
    std::size_t operator()(const Loc& loc) const noexcept {
        return std::hash<size_t>()(loc.depth) ^ (std::hash<size_t>()(loc.offset) << 1);
    }
} LocHash;

typedef struct Scope {
    Loc loc;
    Loc parent_loc;
    Sym var_name;
    const Def* def;

    std::string to_str() const {
        std::ostringstream os;
        os << "Scope{ loc=" << loc.to_str() << ", parent_loc=" << parent_loc.to_str() << ", var=\"" << var_name
           << "\", def=";
        if (def)
            os << def;
        else
            os << "null";
        os << " }";
        return os.str();
    }
} Scope;

typedef fe::SymMap<const Def*> Sym2Def;
typedef absl::flat_hash_map<uint32_t, const Def*> Cache;
typedef absl::flat_hash_map<size_t, size_t> DepthVisits;
typedef std::unordered_map<Loc, Scope, LocHash> ScopeTree;
typedef Sym2Def RootScope;
typedef rust::Vec<NodeFFI> Nodes;

namespace scoped {
typedef struct RecExprState {
    DepthVisits depth_visits;
    Cache cache;
    ScopeTree scope_tree;
    Nodes nodes;
} RecExprState;

typedef struct Context {
    size_t id;
    Loc loc;
    RecExprState* state;
} Context;

typedef absl::flat_hash_map<size_t, RecExprState> RecExprStates;

} // namespace scoped

using ScopedState   = scoped::RecExprState;
using ScopedStates  = scoped::RecExprStates;
using ScopedContext = scoped::Context;

/***************** REWRITER *********************/
class RewriteSlotted : public Phase, public Rewriter {
public:
    RewriteSlotted(World& world, std::string name)
        : Phase(world, std::move(name))
        , Rewriter(world.inherit()) {
        register_symbols();
    }
    RewriteSlotted(World& world, flags_t annex)
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

    size_t id() { return ctx().id; }
    void set_id(size_t id) { ctx().id = id; }

    Nodes& nodes() { return state()->nodes; }
    void set_nodes(size_t id, Nodes nodes) { states_[id].nodes = nodes; }
    size_t root() { return nodes().size() - 1; }

    Cache& cache() { return state()->cache; }
    void set_cache(size_t id, Cache cache) { states_[id].cache = cache; }

    const Def* get_def(uint32_t id) {
        auto def = cache()[id];
        if (!def) {
            auto sym = get_symbol(id);
            sym.empty() ? sym = get_slot(id) : sym;
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

    void register_var(Sym name, const Def* def) {
        if (loc().depth == ROOT_SCOPE_DEPTH) {
            root_scope_add(name, def);
            dbg<SCOPES>("Registering: ", name, "-", def, " in root scope");
        } else {
            scope_add(name, def);
            dbg<SCOPES>("Registering: ", scope()->to_str());
        }
    }

    const Def* get_var(Sym name) {
        auto curr_scope = scope();

        while (name != curr_scope->var_name) {
            if (curr_scope->parent_loc.depth == ROOT_SCOPE_DEPTH) {
                auto it = root_scope().find(name);
                if (it != root_scope().end()) return it->second;
                break;
            }
            curr_scope = scope(curr_scope->parent_loc);
        }

        if (name == curr_scope->var_name) return curr_scope->def;

        return nullptr;
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
    Sym get_slot(uint32_t id) {
        auto node = nodes()[id];
        auto slot = node.slot.c_str();
        return new_world().sym(slot);
    }

    std::vector<uint32_t> get_cons_flat(uint32_t id) {
        std::vector<uint32_t> flattened;
        flattened.reserve(16);
        auto curr_cons = get_node_unsafe(id);
        while (curr_cons.kind != MimKind::Nil) {
            flattened.push_back(curr_cons.children[0]);
            curr_cons = get_node_unsafe(curr_cons.children[1]);
        }
        return flattened;
    }

    /************ State *************/
    ScopedContext& ctx() { return ctx_; }

    ScopedState* state() { return ctx().state; }
    void set_state(size_t id) { ctx().state = &states_[id]; }

    void init_state(size_t id, RecExprFFI& rec_expr) {
        set_depth_visits(id, {});
        set_cache(id, {});
        set_scope_tree(id, {});
        set_nodes(id, rec_expr.nodes);
    }

    void switch_context(size_t id) {
        set_id(id);
        set_state(id);
        reset_loc();
        reset_depth_visits();
    }

    void switch_context(ScopedContext& other) {
        set_id(other.id);
        set_state(other.id);
        set_loc(other.loc);
    }

    void dump_cache() {
        for (auto [id, def] : cache())
            std::cout << id << ": " << def << "\n";
    }
    void dump_scope_tree() {
        for (auto [l, s] : scope_tree())
            std::cout << l.to_str() << ": " << s.to_str() << "\n";
    }
    void dump_depth_visits() {
        for (auto [d, v] : depth_visits())
            std::cout << d << ": " << v << "\n";
    }
    void dump_nodes() {
        for (auto n : nodes())
            std::cout << node_ffi_str(n).c_str() << "\n";
    }
    void dump_state() {
        dbg("----------STATE-----------");
        dbg("Curr ID: ", id());
        dbg("Curr Cache: ");
        dump_cache();
        dbg("Curr Scope Tree: ");
        dump_scope_tree();
        dbg("Curr Loc: ", loc().to_str());
        dbg("Curr Depth Visits: ");
        dump_depth_visits();
        dbg("Curr Scope: ", scope()->to_str());
        dbg("Curr Nodes: ");
        dump_nodes();
        dbg("---------------------------");
    }

    /************ Depth Visits*************/
    DepthVisits& depth_visits() { return state()->depth_visits; }
    void set_depth_visits(size_t id, DepthVisits depth_visits) { states_[id].depth_visits = depth_visits; }
    void set_depth_visits(DepthVisits depth_visits) { state()->depth_visits = depth_visits; }

    void reset_depth_visits() { set_depth_visits({}); }
    void inc_visit_count(size_t depth) { state()->depth_visits[depth] += 1; }

    /******************* Loc **************/
    // Loc tracks the current location in the scope tree.
    // This is done by maintaining a depth and an offset while
    // traversing the RecExprFFI that indicate the exact scope we
    // are currently in. To visualize this:
    //
    //                s1
    //              /    \
    //             s2    s3
    //            /     /  \
    //           s4    s5  s6
    //
    // The location of scope s5 would be at (2, 1) because it is at
    // at a tree-depth of 2 and at an offset of 1 at that depth.
    Loc loc() { return ctx().loc; }
    void set_loc(Loc loc) { ctx().loc = loc; }

    static constexpr int32_t ROOT_SCOPE_DEPTH = -1;
    static constexpr Loc ROOT_LOC             = Loc{ROOT_SCOPE_DEPTH, 0};

    void reset_loc() {
        // We start at Loc {depth: -1, offset: 0} because
        // enter_scope() increments the depth and we want
        // the first scope to begin at (0,0) rather than (1,0)
        set_loc(ROOT_LOC);
    }

    /******************* Scope **************/
    Scope* scope() { return &state()->scope_tree[loc()]; }
    Scope* scope(Loc loc) { return &state()->scope_tree[loc]; }

    void scope_add(Sym name, const Def* def) {
        scope()->var_name = name;
        scope()->def      = def;
    }

    void update_scope() {
        auto curr_scope = scope(loc());
        curr_scope->loc = loc();
    }

    size_t next_offset(size_t next_depth) {
        auto it          = depth_visits().find(next_depth);
        auto next_offset = it == depth_visits().end() ? 0 : it->second;
        return next_offset;
    }

    void enter_scope(NodeFFI node, bool revisit = false) {
        if (node.kind == MimKind::Scope) {
            auto parent_loc = loc();

            auto next_depth = loc().depth + 1;
            auto nxt_offset = next_offset(next_depth);
            auto next_loc   = Loc{next_depth, nxt_offset};
            set_loc(next_loc);

            if (revisit) {
                auto curr_depth   = loc().depth;
                auto prev_offset  = loc().offset - 1;
                auto adjusted_loc = Loc{curr_depth, prev_offset};
                set_loc(adjusted_loc);
            }

            update_scope();
            scope()->parent_loc = parent_loc;
            dbg<SCOPES>("Entering: ", scope()->to_str());
        }
    }

    void exit_scope(NodeFFI node, bool count_visit = false) {
        if (node.kind == MimKind::Scope) {
            dbg<SCOPES>("Exiting: ", scope()->to_str());

            if (count_visit) inc_visit_count(loc().depth);

            auto next_depth = loc().depth - 1;
            auto nxt_offset = next_offset(next_depth);
            auto next_loc   = Loc{next_depth, nxt_offset};
            set_loc(next_loc);

            update_scope();
        }
    }

    /************** Scope Tree ************/
    ScopeTree& scope_tree() { return state()->scope_tree; }
    void set_scope_tree(size_t id, ScopeTree scope_tree) { states_[id].scope_tree = scope_tree; }
    void set_scope_tree(ScopeTree scope_tree) { state()->scope_tree = scope_tree; }

    /************** Root Scope ************/
    const RootScope& root_scope() const { return root_scope_; }
    void root_scope_add(Sym name, const Def* def) { root_scope_[name] = def; }

    Sym2Def axms_;
    Sym2Def aliases_;
    ScopedContext ctx_;
    ScopedStates states_;
    RootScope root_scope_;
};

}; // namespace mim::plug::eqsat
