#pragma once

#include <cstdint>

#include <mim/phase.h>

#include "mim/def.h"
#include "mim/rewrite.h"

#include "rust/eqsat_rs.h"

namespace mim::plug::eqsat {

/****************** DEBUG *********************/
inline constexpr bool DEBUG       = false;
inline constexpr bool SCOPES      = false;
inline constexpr bool PERFORMANCE = true;

template<bool DBG_KIND = DEBUG, typename... Args>
void dbg(Args&&... args) {
    if constexpr (DBG_KIND) (std::cout << ... << std::forward<Args>(args)) << "\n";
}

template<bool DBG_KIND = DEBUG, typename... Args>
void dbg_(Args&&... args) {
    if constexpr (DBG_KIND) (std::cout << ... << std::forward<Args>(args));
}

#define START_TIMER(name) auto _start_##name = std::chrono::steady_clock::now();
#define END_TIMER(name)                                                                                             \
    {                                                                                                               \
        auto _end_##name = std::chrono::steady_clock::now();                                                        \
        if constexpr (PERFORMANCE) {                                                                                \
            std::cout << #name << " took: "                                                                         \
                      << std::chrono::duration_cast<std::chrono::milliseconds>(_end_##name - _start_##name).count() \
                      << "ms\n";                                                                                    \
        }                                                                                                           \
    }

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

typedef absl::flat_hash_map<uint32_t, const Def*> Cache;
typedef absl::flat_hash_map<size_t, Cache> CacheMap;
typedef absl::flat_hash_map<size_t, size_t> DepthVisits;
typedef std::unordered_map<Loc, Scope, LocHash> ScopeTree;
typedef absl::flat_hash_map<size_t, ScopeTree> ScopeTreeMap;
typedef std::set<std::pair<std::string, const Def*>> RootScope;
typedef rust::Vec<NodeFFI> Nodes;

typedef struct State {
    Loc loc;
    DepthVisits depth_visits;
    Nodes nodes;
    Cache* cache;
    ScopeTree* scope_tree;
    Scope* scope;
} State;

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
        for (auto [flags, annex] : old_world().flags2annex()) {
            auto new_annex          = new_world().register_annex(flags, rewrite(annex));
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
    const Def* convert_mutables(uint32_t id);
    const Def* convert_immutables(uint32_t id);
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

    // The nodes of the RecExprFFI we are currently processing
    const Nodes& nodes() const { return nodes_; }
    void set_nodes(rust::Vec<NodeFFI> nodes) { nodes_ = nodes; }

    // Stores Defs that were already created for a node via the nodes' id
    Cache* cache() { return cache_; }
    Cache* cache(size_t rec_expr_id) { return &cache_map_[rec_expr_id]; }
    void set_cache(Cache* cache) { cache_ = cache; }
    void set_cache(size_t rec_expr_id) { set_cache(cache(rec_expr_id)); }
    void dump_cache() {
        for (auto [id, def] : cache_map_[0])
            std::cout << id << ": " << def << "\n";
    }

    const Def* cache_get(uint32_t id) {
        auto it = cache()->find(id);
        return it != cache()->end() ? it->second : nullptr;
    }
    const Def* cache_set(uint32_t id, const Def* def) { return (*cache())[id] = def; }
    uint32_t get_id(const Def* def) {
        auto it = std::find_if(cache()->begin(), cache()->end(), [&](const auto& pair) { return pair.second == def; });
        if (it != cache()->end()) return it->first;
        error("Could not find the given Def in the cache.");
        return -1;
    }

    // A node that is associated with a Def can be:
    // 1) A node representing an arbitrary term
    // 2) A symbol node representing an annex
    // 3) A symbol node representing a type or term alias
    // 4) A symbol node representing a variable
    const Def* get_def(uint32_t id) {
        auto def = cache_get(id);
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

    const Def* get_alias(Sym name) { return aliases_.contains(name) ? aliases_[name] : nullptr; }

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
                for (auto [var_name, def] : root_scope())
                    if (var_name == name) return def;
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
    const Def* get_axm(Sym name) { return axms_.contains(name) ? axms_[name] : nullptr; }

    NodeFFI get_node(MimKind expected, uint32_t id) {
        assert(nodes()[id].kind == expected && "get_node: mismatch between expected and actual node kind");
        return nodes()[id];
    }
    NodeFFI get_node_unsafe(uint32_t id) { return nodes()[id]; }

    Sym get_symbol(uint32_t id) {
        auto sv = std::string_view(nodes()[id].symbol);
        return new_world().sym(sv);
    }
    uint64_t get_num(uint32_t id) { return nodes()[id].num; }
    Sym get_slot(uint32_t id) {
        auto sv = std::string_view(nodes()[id].slot);
        return new_world().sym(sv);
    }

    // Returns a flattened vector of node id's for a cons list
    // i.e.: (cons 23 (cons 12 nil)) => [23, 12]
    std::vector<uint32_t> get_cons_flat(uint32_t id) {
        std::vector<uint32_t> flattened;
        auto curr_cons = get_node_unsafe(id);
        while (curr_cons.kind != MimKind::Nil) {
            flattened.push_back(curr_cons.children[0]);
            curr_cons = get_node_unsafe(curr_cons.children[1]);
        }
        return flattened;
    }

    void set_state(size_t rec_expr_id, RecExprFFI rec_expr) {
        reset_loc();
        reset_depth_visits();
        set_nodes(rec_expr.nodes);
        set_cache(rec_expr_id);
        set_scope_tree(rec_expr_id);
        set_scope(loc());
    }

    State save_state() { return State{loc(), depth_visits(), nodes(), cache(), scope_tree(), scope()}; }

    State temp_state(Cache* cache, ScopeTree* scope_tree, Nodes nodes) {
        reset_loc();
        reset_depth_visits();
        set_cache(cache);
        set_scope_tree(scope_tree);
        set_scope(loc());
        set_nodes(nodes);
        return save_state();
    }

    void restore_state(State state, bool keep_cache = false) {
        set_loc(state.loc);
        set_depth_visits(state.depth_visits);
        set_nodes(state.nodes);
        if (!keep_cache) set_cache(state.cache);
        set_scope_tree(state.scope_tree);
        set_scope(state.scope);
    }

    /************ Depth Visits*************/
    const DepthVisits& depth_visits() const { return depth_visits_; }
    void set_depth_visits(absl::flat_hash_map<size_t, size_t> depth_visits) { depth_visits_ = depth_visits; }

    // I doubt that any practical examples would be able to exceed such a scope depth
    // Still, some dynamic resizing needs to be added at some point.
    const size_t MAX_DEPTH = 1024;
    void reset_depth_visits() { set_depth_visits({}); }
    void inc_visit_count(size_t depth) { depth_visits_[depth] += 1; }

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
    //
    // The reason we do this is because we convert the RecExprFFI's
    // into the new_world() in multiple traverses. First, we perform
    // a top-down traverse to create all bindings in their proper scopes
    // and then we perform a bottom-up traverse to create all other Defs.
    //
    // To be able to access the variables we bound in the first top-down
    // traverse, in the second bottom-up traverse, we need only to provide
    // our current location and the name of the variable whose definition
    // we need and we can simply look it up in the scopes_ map.

    Loc loc() const { return curr_loc_; }
    void set_loc(Loc loc) { curr_loc_ = loc; }

    void reset_loc() {
        // We start at Loc {depth: -1, offset: 0} because
        // enter_scope() increments the depth and we want
        // the first scope to begin at (0,0) rather than (1,0)
        set_loc({ROOT_SCOPE_DEPTH, 0});
    }

    /******************* Scope **************/
    Scope* scope() const { return curr_scope_; }
    Scope* scope(Loc loc) { return &(*scope_tree_)[loc]; }
    void set_scope(Scope* scope) { curr_scope_ = scope; }
    void set_scope(Loc loc) { set_scope(scope(loc)); }

    void scope_add(Sym name, const Def* def) {
        scope()->var_name = name;
        scope()->def      = def;
    }

    void update_scope() {
        auto curr_scope = scope(loc());
        curr_scope->loc = loc();
        set_scope(curr_scope);
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

            // We sometimes need to be able to revisit a scope we just exited
            // in the convert() bottom-up traverse. Since the last visit coming
            // from the bottom up was counted, our offset when revisiting needs
            // to be decremented by one in order to account for the fact that
            // we are visiting the same scope again.
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
    ScopeTree* scope_tree() const { return scope_tree_; }
    ScopeTree* scope_tree(size_t rec_expr_id) { return &scope_tree_map_[rec_expr_id]; }
    void set_scope_tree(ScopeTree* scope_tree) { scope_tree_ = scope_tree; }
    void set_scope_tree(size_t rec_expr_id) { set_scope_tree(scope_tree(rec_expr_id)); }

    /************** Root Scope ************/
    const RootScope& root_scope() const { return root_scope_; }

    void root_scope_add(Sym name, const Def* def) { root_scope_.insert({name.str(), def}); }

    /********** SCOPES INTERFACE **********/
    const int32_t ROOT_SCOPE_DEPTH = -1;
    // Keeps track of how often we have visited each scope-depth
    // so we can keep track of the current locations' offset at each depth.
    // maps: Depth -> #Visits
    DepthVisits depth_visits_;
    Loc curr_loc_;
    // The current scope which we mostly use to construct the scope map during init
    Scope* curr_scope_;
    // For every scope-location we store a Scope struct that stores a pointer to its
    // parent scope, the name of the var it introduces, and the Def associated with this var.
    ScopeTree* scope_tree_;
    // For every RecExprFFI keyed by its idx, we store a structure representing its scopetree.
    ScopeTreeMap scope_tree_map_;
    // There is a special root scope which is a registry of all top-level/closed Defs
    // that exist beyond the current RecExprFFI.
    RootScope root_scope_;

    Nodes nodes_;
    Cache* cache_;
    CacheMap cache_map_;
    fe::SymMap<const Def*> axms_;
    fe::SymMap<const Def*> aliases_;
};

}; // namespace mim::plug::eqsat
