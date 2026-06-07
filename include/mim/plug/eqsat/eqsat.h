#pragma once

#include "mim/world.h"

#include "mim/plug/eqsat/autogen.h"

namespace mim::plug::eqsat {

inline std::optional<const Def*> get_config_option(const Def* config_def) {
    if (auto inj_outer = config_def->isa<Inj>()) {
        if (auto match = inj_outer->value()->isa<Match>()) {
            if (auto inj_inner = match->scrutinee()->isa<Inj>()) {
                auto val = inj_inner->value();
                if (auto tuple = val->isa<Tuple>(); tuple && tuple->ops().empty())
                    return std::nullopt;
                else
                    return val;
            }
        }
    }
    return std::nullopt;
}
} // namespace mim::plug::eqsat

namespace mim {

// TODO: implement
inline void eqsat_config(World& world,
                         std::optional<flags_t> impl,
                         std::optional<flags_t> cost_fun,
                         std::optional<std::vector<flags_t>> rulesets,
                         std::optional<DefVec> rules,
                         std::optional<DefVec> reaches,
                         std::optional<DefVec> select) {}

// lam extern _impl(): %eqsat.Impl =
//     <impl>;
inline void eqsat_impl(World& world, flags_t impl) {
    auto Impl     = world.annex<plug::eqsat::Impl>();
    auto impl_axm = world.annex(impl);
    auto _impl    = world.mut_lam({}, Impl)->set("_impl");
    _impl->set_filter(false);
    _impl->set_body(impl_axm);
    _impl->externalize();
}

// lam extern _cost_fun(): %eqsat.CostFun =
//     <cost_fun>;
inline void eqsat_cost_fun(World& world, flags_t cost_fun) {
    auto CostFun      = world.annex<plug::eqsat::CostFun>();
    auto cost_fun_axm = world.annex(cost_fun);
    auto _cost_fun    = world.mut_lam({}, CostFun)->set("_cost_fun");
    _cost_fun->set_filter(false);
    _cost_fun->set_body(cost_fun_axm);
    _cost_fun->externalize();
}

// lam extern _rulesets(): %eqsat.Ruleset =
//     %eqsat.rulesets (<rulesets>,);
inline void eqsat_rulesets(World& world, std::vector<flags_t> rulesets) {
    auto Ruleset = world.annex<plug::eqsat::Ruleset>();

    DefVec ruleset_axms;
    for (auto ruleset : rulesets) {
        auto ruleset_axm = world.annex(ruleset);
        ruleset_axms.push_back(ruleset_axm);
    }
    auto ruleset_tuple = world.tuple(ruleset_axms);
    auto rulesets_app  = world.call(world.annex<plug::eqsat::rulesets>(), ruleset_tuple);

    auto _rulesets = world.mut_lam({}, Ruleset)->set("_rulesets");
    _rulesets->set_filter(false);
    _rulesets->set_body(rulesets_app);
    _rulesets->externalize();
}

// lam extern _rules(): %eqsat.Rules =
//     %eqsat.rules (<rules>,);
inline void eqsat_rules(World& world, DefVec rules) {
    auto Rules = world.annex<plug::eqsat::Rules>();

    auto rules_tuple = world.tuple(rules);
    auto rules_app   = world.call(world.annex<plug::eqsat::rules>(), rules_tuple);

    auto _rules = world.mut_lam({}, Rules)->set("_rules");
    _rules->set_filter(false);
    _rules->set_body(rules_app);
    _rules->externalize();
}

} // namespace mim
