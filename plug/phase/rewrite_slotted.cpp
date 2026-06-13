#include <cstdint>

#include <mim/plug/eqsat/eqsat.h>
#include <mim/plug/eqsat/phase/rewrite_slotted.h>

#include "mim/def.h"
#include "mim/sexpr.h"

#include "mim/plug/eqsat/autogen.h"

namespace mim::plug::eqsat {

const std::unordered_set MUTABLES   = {MimKind::Lam, MimKind::Con, MimKind::Fun,   MimKind::ImplicitPi, MimKind::Pi,
                                       MimKind::Cn,  MimKind::Fn,  MimKind::Sigma, MimKind::Arr,        MimKind::Pack};
const std::unordered_set NO_CONVERT = {MimKind::Axm};

void RewriteSlotted::start() {
    auto [rulesets, cost_fn, reaches_args, selected] = import_config();

    START_TIMER(sexpr)
    std::ostringstream sexpr;
    sexpr::emit_slotted_typed(old_world(), sexpr);
    END_TIMER(sexpr)

    dbg(sexpr.str());

    START_TIMER(reaches)
    assert_reaches(sexpr.str(), rulesets, reaches_args);
    END_TIMER(reaches)

    // If no terms are selected for saturation, we simply use the Rewriter to transfer the old world to
    // the new world unchanged, which is faster and less involved than init + convert.
    if (swap_world_unchanged(selected)) return;

    START_TIMER(eqsat)
    auto rec_exprs = eqsat_slotted(sexpr.str(), selected, rulesets, cost_fn);
    END_TIMER(eqsat)

    // Heap-allocated pointer needs manual dealloc and the reason we even use pointers
    // here is that Cxx doesn't yet have an Option type implemented for its FFI, so the
    // workaround to that is to use a raw pointer where nullptr represents the None variant.
    if (selected.option) delete selected.option;

    dbg(pretty_ffi(rec_exprs, 80).c_str());

    START_TIMER(rewrite)
    init(rec_exprs);
    convert(rec_exprs);
    END_TIMER(rewrite)

    swap(old_world(), new_world());
}

bool RewriteSlotted::swap_world_unchanged(OptionSelected selected) {
    if (selected.option && selected.option->empty()) {
        delete selected.option;
        for (auto mut : old_world().externals().muts()) {
            auto new_mut = rewrite(mut)->as_mut();
            if (mut->is_external()) new_mut->externalize();
        }
        swap(old_world(), new_world());
        return true;
    }
    return false;
}

ConfigValues RewriteSlotted::import_config() {
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

    // Import config values from the internalized config lambdas
    RuleSets rulesets;
    CostFn cost_fn = CostFn::AstSize;
    ReachesArgs reaches_args;
    OptionSelected selected = {nullptr};

    for (auto lam : lams) {
        auto body               = lam->as<Lam>()->body();
        DefVec singleton_config = {body};
        auto config_vals        = body->isa<Tuple>() ? body->as<Tuple>()->ops() : Defs(singleton_config);
        for (auto config_val : config_vals) {
            if (auto ruleset_config = Axm::isa<eqsat::rulesets>(config_val)) {
                // Rulesets
                for (auto ruleset : ruleset_config->args())
                    if (Axm::isa<eqsat::standard>(ruleset))
                        rulesets.push_back(RuleSet::Standard);
                    else if (Axm::isa<eqsat::rise>(ruleset))
                        rulesets.push_back(RuleSet::Rise);
                    else if (Axm::isa<eqsat::normalize>(ruleset))
                        rulesets.push_back(RuleSet::Normalize);
                    // AUTOGEN START: slotted-ruleset-cpp
                    // AUTOGEN END: slotted-ruleset-cpp
                    else
                        error("%eqsat.rulesets: Ruleset {} not found for %eqsat.slotted", ruleset);

            } else if (auto reaches = Axm::isa<eqsat::reaches>(config_val)) {
                // Reaches assertions
                auto [start_term, end_term, max_steps] = reaches->args<3>();
                if (auto start_lam = start_term->isa<Lam>(); !(start_lam && start_lam->is_closed()))
                    error("%eqsat.reaches currently only supports variables to root-level lambdas");
                if (auto end_lam = end_term->isa<Lam>(); !(end_lam && end_lam->is_closed()))
                    error("%eqsat.reaches currently only supports variables to root-level lambdas");
                reaches_args.push_back({start_term->sym().str(), end_term->sym().str(), max_steps->as<Lit>()->get()});

            } else if (Axm::isa<eqsat::AstSize>(config_val)) {
                // Cost functions
                cost_fn = CostFn::AstSize;
            } else if (Axm::isa<eqsat::MaxAstSize>(config_val)) {
                cost_fn = CostFn::MaxAstSize;

            } else if (auto select = Axm::isa<eqsat::select>(config_val)) {
                // Selections
                auto option = new rust::Vec<rust::String>();
                for (auto term : select->args()) {
                    if (auto lam = term->isa<Lam>(); !(lam && lam->is_closed()))
                        error("%eqsat.select currently only supports variables to root-level lambdas");
                    option->push_back(term->sym().str());
                }
                selected.option = option;

            } else if (Axm::isa<eqsat::rules>(config_val) || Axm::isa<eqsat::rules_kind>(config_val)) {
                // Rules
                auto dom       = old_world().sigma();
                auto codom     = old_world().annex<eqsat::Config>();
                auto rules_lam = old_world().mut_lam(dom, codom)->set("_rules");
                rules_lam->set_filter(false);
                rules_lam->set_body(config_val);
                rules_lam->externalize();

            } else if (Axm::isa<eqsat::slotted>(config_val) || Axm::isa<eqsat::egg>(config_val)) {
                // Implementations
                continue;

            } else {
                error("Slotted: Invalid config value: {}", config_val);
            }
        }
    }

    return {rulesets, cost_fn, reaches_args, selected};
}

void RewriteSlotted::assert_reaches(std::string sexpr, RuleSets rulesets, ReachesArgs reaches_args) {
    for (auto [start_term, end_term, max_steps] : reaches_args)
        if (!reaches_slotted(sexpr, rulesets, start_term, end_term, max_steps))
            error("%eqsat.reaches: {} could not reach {} in under {} steps.", start_term, end_term, max_steps);
}

const Def* RewriteSlotted::create_type(RecExprFFI type_) {
    if (type_.nodes.empty()) error("Tried to create an empty type.");
    auto outer_state = save_state();

    auto type_state   = temp_state(type_.nodes);
    auto type_root_id = type_.nodes.size() - 1;
    init(type_root_id);

    dbg("Type init stage complete!");

    restore_state(type_state, true);
    auto res = convert(type_root_id);

    dbg("Type convert stage complete!");

    restore_state(outer_state);
    return res;
}

void RewriteSlotted::init(rust::Vec<RecExprFFI> rec_exprs) {
    for (size_t rec_expr_id = 0; rec_expr_id < rec_exprs.size(); rec_expr_id++) {
        dbg("\nInitializing RecExpr: ", rec_expr_id);

        auto rec_expr = rec_exprs[rec_expr_id];
        set_state(rec_expr_id, rec_expr);

        auto root_id = nodes()->size() - 1;
        init(root_id);
    }
}

const Def* RewriteSlotted::init(uint32_t id) {
    auto node = get_node_unsafe(id);
    enter_scope(node);

    const Def* res = cache_get(id);
    if (!res) {
        switch (node.kind) {
            case MimKind::Axm: res = init_axm(id, node); break;
            case MimKind::Root: res = init_root(id, node); break;
            case MimKind::Fun:
            case MimKind::Con:
            case MimKind::Lam: res = init_lam(id, node); break;
            case MimKind::Let: res = init_let(id, node); break;
            case MimKind::Fn:
            case MimKind::Cn:
            case MimKind::ImplicitPi:
            case MimKind::Pi: res = init_pi(id, node); break;
            case MimKind::Sigma: res = init_sigma(id, node); break;
            case MimKind::Arr: res = init_arr(id, node); break;
            case MimKind::Pack: res = init_pack(id, node); break;
            default: break;
        }
    }

    for (uint32_t child : node.children)
        init(child);

    exit_scope(node, true);
    return cache_set(id, res);
}

const Def* RewriteSlotted::init_lookahead(uint32_t id) {
    auto node = get_node_unsafe(id);

    const Def* res = cache_get(id);
    if (!res) {
        switch (node.kind) {
            case MimKind::Fun:
            case MimKind::Con:
            case MimKind::Lam: res = init_lam(id, node); break;
            case MimKind::Fn:
            case MimKind::Cn:
            case MimKind::ImplicitPi:
            case MimKind::Pi: res = init_pi(id, node); break;
            case MimKind::Sigma: res = init_sigma(id, node); break;
            case MimKind::Arr: res = init_arr(id, node); break;
            case MimKind::Pack: res = init_pack(id, node); break;
            default:
                auto saved_state = save_state();

                init(id);
                restore_state(saved_state, true);

                res = convert(id);
                restore_state(saved_state, true);
                break;
        }
    }
    return cache_set(id, res);
}

// (axm <name>)
const Def* RewriteSlotted::init_axm(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto name = get_symbol(node.children[0]);

    auto type = create_type(node.type_);

    auto new_axm = new_world().axm(type);
    new_axm->set(name);
    register_axm(name, new_axm);

    dbg(new_axm);
    return new_axm;
}

// (root <extern> <name> <definition>)
const Def* RewriteSlotted::init_root(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");

    auto name = get_symbol(node.children[1]);

    auto def = init_lookahead(node.children[2]);
    def->set(name);
    register_var(name, def);

    dbg(def);
    return nullptr;
}

// (let $var (scope <definition> <expression>))
const Def* RewriteSlotted::init_let(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope);

    auto var_name = get_slot(id);

    auto def = init_lookahead(var_scope.children[0]);
    def->set(var_name);
    register_var(var_name, def);

    dbg(def);
    exit_scope(var_scope);
    return nullptr;
}

// (lam $var (scope <filter> <body>))
const Def* RewriteSlotted::init_lam(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope);

    auto pi_type = create_type(node.type_)->as<Pi>();
    auto mut_lam = new_world().mut_lam(pi_type);

    auto var_name = get_slot(id);
    auto var      = mut_lam->var();
    var->set(var_name);
    register_var(var_name, var);

    dbg(mut_lam);
    exit_scope(var_scope);
    return mut_lam;
}

// (pi $var (scope <dom> <codom>))
const Def* RewriteSlotted::init_pi(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope);

    auto implicit = node.kind == MimKind::ImplicitPi;
    auto mut_pi   = new_world().mut_pi(new_world().type_infer_univ(), implicit);

    auto var_name = get_slot(id);
    auto var      = mut_pi->var();
    var->set(var_name);
    register_var(var_name, var);

    auto dom = init_lookahead(var_scope.children[0]);
    mut_pi->set_dom(dom);
    auto codom = init_lookahead(var_scope.children[1]);
    mut_pi->set_codom(codom);

    dbg(mut_pi);
    exit_scope(var_scope);

    if (auto imm_pi = mut_pi->immutabilize()) return imm_pi;
    return mut_pi;
}

// (sigma $var (scope <type-cons> nil))
const Def* RewriteSlotted::init_sigma(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope);

    auto type_ids = get_cons_flat(var_scope.children[0]);
    auto size     = type_ids.size();

    auto mut_sigma = new_world().mut_sigma(new_world().type_infer_univ(), size);

    auto var_name = get_slot(id);
    auto var      = mut_sigma->var();
    var->set(var_name);
    register_var(var_name, var);

    auto saved_state = save_state();
    for (size_t i = 0; i < size; i++) {
        auto type = init_lookahead(type_ids[i]);
        mut_sigma->set(i, type);
        inc_visit_count(loc().depth + 1);
    }
    restore_state(saved_state);

    dbg(mut_sigma);
    exit_scope(var_scope);

    if (auto imm_sigma = mut_sigma->immutabilize()) return imm_sigma;
    return mut_sigma;
}

// (arr $var (scope <arity> <body>))
const Def* RewriteSlotted::init_arr(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope);

    auto mut_arr = new_world().mut_arr(new_world().type_infer_univ());
    auto arity   = init_lookahead(var_scope.children[0]);
    mut_arr->set_arity(arity);

    auto var_name = get_slot(id);
    auto var      = mut_arr->var();
    var->set(var_name);
    register_var(var_name, var);

    auto body = init_lookahead(var_scope.children[1]);
    mut_arr->set_body(body);

    dbg(mut_arr);
    exit_scope(var_scope);

    if (auto imm_arr = mut_arr->immutabilize()) return imm_arr;
    return mut_arr;
}

// (pack $var (scope <arity> <body>))
const Def* RewriteSlotted::init_pack(uint32_t id, NodeFFI node) {
    dbg("init - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope);

    auto mut_arr  = new_world().mut_arr(new_world().type_infer_univ());
    auto mut_pack = new_world().mut_pack(mut_arr);

    auto arity = init_lookahead(var_scope.children[0]);
    mut_arr->set_arity(arity);

    auto var_name = get_slot(id);
    auto var      = mut_pack->var();
    var->set(var_name);
    register_var(var_name, var);

    auto body = init_lookahead(var_scope.children[1]);
    mut_arr->set_body(body->type());
    mut_pack->set(body);

    dbg(mut_pack);
    exit_scope(var_scope);

    if (auto imm_pack = mut_pack->immutabilize()) return imm_pack;
    return mut_pack;
}

void RewriteSlotted::convert(rust::Vec<RecExprFFI> rec_exprs) {
    for (size_t rec_expr_id = 0; rec_expr_id < rec_exprs.size(); rec_expr_id++) {
        dbg("\nConverting RecExpr: ", rec_expr_id);
        auto rec_expr = rec_exprs[rec_expr_id];
        set_state(rec_expr_id, rec_expr);

        auto root_id = nodes()->size() - 1;
        convert(root_id);
    }
}

const Def* RewriteSlotted::convert(uint32_t id) {
    auto node = get_node_unsafe(id);
    enter_scope(node);

    if (NO_CONVERT.contains(node.kind)) return nullptr;

    for (uint32_t child : node.children)
        convert(child);

    const Def* res = cache_get(id);
    if (res && !MUTABLES.contains(node.kind)) return res;

    dbg_("convert - current node(", id, "): ", node_ffi_str(node).c_str(), " - ");
    switch (node.kind) {
        case MimKind::Root: res = convert_root(id, node); break;
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

    if (res)
        if (auto mut = res->isa_mut()) mut->immutabilize();

    if (node.kind == MimKind::Scope) dbg_<SCOPES>("\n");
    exit_scope(node, true);

    dbg(res);
    return cache_set(id, res);
}

// (root <extern> <name> <definition>)
const Def* RewriteSlotted::convert_root(uint32_t id, NodeFFI node) {
    auto is_extern = get_symbol(node.children[0]);
    auto def       = get_def(node.children[1]);

    if (auto lam = def->isa_mut<Lam>()) {
        if (is_extern == "extern") lam->externalize();
    }

    return def;
}

// (let $var (scope <definition> <expression>))
const Def* RewriteSlotted::convert_let(uint32_t id, NodeFFI node) {
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope, true);

    auto expr = get_def(var_scope.children[1]);

    exit_scope(var_scope);
    return expr;
}

// (lam $var (scope <filter> <body>))
const Def* RewriteSlotted::convert_lam(uint32_t id, NodeFFI node) {
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope, true);

    auto lam = get_def(id)->as<Lam>();

    if (auto mut_lam = lam->isa_mut<Lam>()) {
        auto filter = get_def(var_scope.children[0]);
        auto body   = get_def(var_scope.children[1]);

        mut_lam->unset();

        if (filter && body)
            mut_lam->set(filter, body);
        else
            mut_lam->set_filter(false);
    }

    exit_scope(var_scope);
    return lam;
}

// (app <callee> <arg>)
const Def* RewriteSlotted::convert_app(uint32_t id, NodeFFI node) {
    auto callee  = get_def(node.children[0]);
    auto arg     = get_def(node.children[1]);
    auto new_app = new_world().app(callee, arg);
    return new_app;
}

// (var $name)
const Def* RewriteSlotted::convert_var(uint32_t id, NodeFFI node) {
    auto var = get_def(id);
    return var;
}

// (lit <val> <type>)
const Def* RewriteSlotted::convert_lit(uint32_t id, NodeFFI node) {
    auto lit_def = get_def(node.children[0]);
    if (lit_def) return lit_def;

    auto lit_val  = get_num(node.children[0]);
    auto lit_type = get_def(node.children[1]);
    auto new_lit  = new_world().lit(lit_type, lit_val);
    return new_lit;
}

// (pack $var (scope <arity> <body>))
const Def* RewriteSlotted::convert_pack(uint32_t id, NodeFFI node) {
    dbg_<SCOPES>("\n");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope, true);

    auto pack = get_def(id)->as<Pack>();

    if (auto mut_pack = pack->isa_mut<Pack>()) {
        auto body = get_def(var_scope.children[1]);

        mut_pack->unset();
        mut_pack->set(body);
    }

    exit_scope(var_scope);
    return pack;
}

// (tuple <elem-cons>)
const Def* RewriteSlotted::convert_tuple(uint32_t id, NodeFFI node) {
    auto elem_ids = get_cons_flat(node.children[0]);

    DefVec elems;
    for (auto elem_id : elem_ids) {
        auto elem = get_def(elem_id);
        elems.push_back(elem);
    }
    auto new_tuple = new_world().tuple(elems);
    return new_tuple;
}

// (extract <tuple> <index>)
const Def* RewriteSlotted::convert_extract(uint32_t id, NodeFFI node) {
    auto tuple       = get_def(node.children[0]);
    auto index       = get_def(node.children[1]);
    auto new_extract = new_world().extract(tuple, index);
    return new_extract;
}

// (ins <tuple> <index> <value>)
const Def* RewriteSlotted::convert_insert(uint32_t id, NodeFFI node) {
    auto tuple      = get_def(node.children[0]);
    auto index      = get_def(node.children[1]);
    auto value      = get_def(node.children[2]);
    auto new_insert = new_world().insert(tuple, index, value);
    return new_insert;
}

// (inj <type> <value>)
const Def* RewriteSlotted::convert_inj(uint32_t id, NodeFFI node) {
    auto type    = get_def(node.children[0]);
    auto value   = get_def(node.children[1]);
    auto new_inj = new_world().inj(type, value);
    return new_inj;
}

// (merge <type> <value-cons>)
const Def* RewriteSlotted::convert_merge(uint32_t id, NodeFFI node) {
    auto type = get_def(node.children[0]);

    auto value_ids = get_cons_flat(node.children[1]);
    DefVec values;
    for (auto value_id : value_ids) {
        auto value = get_def(value_id);
        values.push_back(value);
    }
    auto new_merge = new_world().merge(type, values);
    return new_merge;
}

// (match <op-cons>)
const Def* RewriteSlotted::convert_match(uint32_t id, NodeFFI node) {
    auto op_ids = get_cons_flat(node.children[0]);

    DefVec ops;
    for (auto op_id : op_ids) {
        auto op = get_def(op_id);
        ops.push_back(op);
    }
    auto new_match = new_world().match(ops);
    return new_match;
}

// (proxy <type> <pass> <tag> <op-cons>)
const Def* RewriteSlotted::convert_proxy(uint32_t id, NodeFFI node) {
    auto type = get_def(node.children[0]);
    auto pass = get_num(node.children[1]);
    auto tag  = get_num(node.children[2]);

    auto op_ids = get_cons_flat(node.children[3]);
    DefVec ops;
    for (auto op_id : op_ids) {
        auto op = get_def(op_id);
        ops.push_back(op);
    }
    auto new_proxy = new_world().proxy(type, ops, pass, tag);
    return new_proxy;
}

// (join <type-cons>)
const Def* RewriteSlotted::convert_join(uint32_t id, NodeFFI node) {
    auto type_ids = get_cons_flat(node.children[0]);

    DefVec types;
    for (auto type_id : type_ids) {
        auto type = get_def(type_id);
        types.push_back(type);
    }
    auto new_join = new_world().join(types);
    return new_join;
}

// (meet <type-cons>)
const Def* RewriteSlotted::convert_meet(uint32_t id, NodeFFI node) {
    auto type_ids = get_cons_flat(node.children[0]);

    DefVec types;
    for (auto type_id : type_ids) {
        auto type = get_def(type_id);
        types.push_back(type);
    }
    auto new_meet = new_world().meet(types);
    return new_meet;
}

// (bot <type>)
const Def* RewriteSlotted::convert_bot(uint32_t id, NodeFFI node) {
    auto type    = get_def(node.children[0]);
    auto new_bot = new_world().bot(type);
    return new_bot;
}

// (top <type>)
const Def* RewriteSlotted::convert_top(uint32_t id, NodeFFI node) {
    auto type    = get_def(node.children[0]);
    auto new_top = new_world().top(type);
    return new_top;
}

// (arr $var (scope <arity> <body>))
const Def* RewriteSlotted::convert_arr(uint32_t id, NodeFFI node) {
    dbg_<SCOPES>("\n");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope, true);

    auto arr = get_def(id)->as<Arr>();

    if (auto mut_arr = arr->isa_mut<Arr>()) {
        auto arity = get_def(var_scope.children[0]);
        auto body  = get_def(var_scope.children[1]);

        mut_arr->unset();
        mut_arr->set(arity, body);
    }

    exit_scope(var_scope);
    return arr;
}

// (sigma $var (scope <type-cons> nil))
const Def* RewriteSlotted::convert_sigma(uint32_t id, NodeFFI node) {
    dbg_<SCOPES>("\n");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope, true);

    auto sigma = get_def(id)->as<Sigma>();

    exit_scope(var_scope);
    return sigma;
}

// (pi $var (scope <domain> <codomain>))
const Def* RewriteSlotted::convert_pi(uint32_t id, NodeFFI node) {
    dbg_<SCOPES>("\n");
    auto var_scope = get_node(MimKind::Scope, node.children[0]);
    enter_scope(var_scope, true);

    auto pi = get_def(id)->as<Pi>();

    if (auto mut_pi = pi->isa_mut<Pi>()) {
        auto domain   = get_def(var_scope.children[0]);
        auto codomain = get_def(var_scope.children[1]);

        mut_pi->unset();
        mut_pi->set(domain, codomain);
    }

    exit_scope(var_scope);
    return pi;
}

// (idx <size>)
const Def* RewriteSlotted::convert_idx(uint32_t id, NodeFFI node) {
    auto size    = get_def(node.children[0]);
    auto new_idx = new_world().type_idx(size);
    return new_idx;
}

// (hole <type>)
const Def* RewriteSlotted::convert_hole(uint32_t id, NodeFFI node) {
    auto type_    = get_def(node.children[0]);
    auto new_hole = new_world().mut_hole(type_);
    return new_hole;
}

// (type <level>)
const Def* RewriteSlotted::convert_type(uint32_t id, NodeFFI node) {
    auto level    = get_def(node.children[0]);
    auto new_type = new_world().type(level);
    return new_type;
}

// <u64>
const Def* RewriteSlotted::convert_num(uint32_t id, NodeFFI node) { return nullptr; }

// <string>
const Def* RewriteSlotted::convert_symbol(uint32_t id, NodeFFI node) {
    auto def = get_def(id);
    return def;
}

} // namespace mim::plug::eqsat
