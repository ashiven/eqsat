#pragma once

#include "mim/world.h"

#include "mim/plug/compile/autogen.h"
#include "mim/plug/eqsat/autogen.h"

namespace mim {

inline const Def* eqsat_rulesets(World& world, std::vector<flags_t> rulesets) {
    DefVec ruleset_axms;
    for (auto ruleset : rulesets) {
        auto ruleset_axm = world.annex(ruleset);
        ruleset_axms.push_back(ruleset_axm);
    }
    auto ruleset_tuple = world.tuple(ruleset_axms);
    auto rulesets_app  = world.call(world.annex<plug::eqsat::rulesets>(), ruleset_tuple);

    return rulesets_app;
}

inline const Def* eqsat_rules(World& world, DefVec rules) {
    auto rules_tuple = world.tuple(rules);
    auto rules_app   = world.call(world.annex<plug::eqsat::rules>(), rules_tuple);

    return rules_app;
}

inline const Def* eqsat_reaches(World& world, DefVec reaches) {
    auto reaches_tuple = world.tuple(reaches);
    auto reaches_app   = world.call(world.annex<plug::eqsat::reaches>(), reaches_tuple);

    return reaches_app;
}

inline const Def* eqsat_select(World& world, DefVec select) {
    auto select_tuple = world.tuple(select);
    auto select_app   = world.call(world.annex<plug::eqsat::select>(), select_tuple);

    return select_app;
}

// lam extern _config() =
//     %eqsat.config (
//         <impl>,
//         <cost_fun>,
//         <rulesets>,
//         <rules>,
//         <reaches>,
//         <select>,
//     );
inline void eqsat_config(World& world,
                         flags_t impl,
                         flags_t cost_fun,
                         const Def* rulesets = nullptr,
                         const Def* rules    = nullptr,
                         const Def* reaches  = nullptr,
                         const Def* select   = nullptr) {
    auto eqsat_config = world.externals()[world.sym("%eqsat.config")];
    auto codom        = eqsat_config->as<Lam>()->codom();
    auto _config      = world.mut_lam(world.sigma(), codom)->set("_config");

    auto impl_v     = world.annex(impl);
    auto cost_fun_v = world.annex(cost_fun);

    auto config_tuple     = world.tuple({impl_v, cost_fun_v, rulesets, rules, reaches, select});
    auto eqsat_config_app = world.app(eqsat_config, config_tuple);

    _config->set_filter(false);
    _config->set_body(eqsat_config_app);
    _config->externalize();
}

inline void eqsat_pipeline(World& world) {
    auto _compile = world.mut_lam(world.sigma(), world.annex<plug::compile::Phase>());

    auto body = world.call(world.annex<plug::compile::phases>(), world.lit_ff(),
                           world.tuple({world.annex<plug::eqsat::eqsat_phase>()}));

    _compile->set_filter(false);
    _compile->set_body(body);
    _compile->externalize();
}

} // namespace mim
