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
//         %option.Opt <impl>
//         %option.Opt <cost_fun>
//         %option.Opt <rulesets>
//         %option.Opt <rules>
//         %option.Opt <reaches>
//         %option.Opt <select>
//     );
inline void eqsat_config(World& world,
                         std::optional<flags_t> impl,
                         std::optional<flags_t> cost_fun,
                         std::optional<std::vector<flags_t>> rulesets,
                         std::optional<DefVec> rules,
                         std::optional<DefVec> reaches,
                         std::optional<DefVec> select) {
    auto Configs = world.externals()[world.sym("%eqsat.Configs")];
    auto _config = world.mut_lam(world.sigma(), Configs)->set("_config");

    auto option_some = world.externals()[world.sym("%option.some")];
    auto option_none = world.externals()[world.sym("%option.none")];

    auto impl_v     = impl.has_value() ? world.app(option_some, world.annex(impl.value())) : option_none;
    auto cost_fun_v = cost_fun.has_value() ? world.app(option_some, world.annex(cost_fun.value())) : option_none;
    auto rulesets_v
        = rulesets.has_value() ? world.app(option_some, eqsat_rulesets(world, rulesets.value())) : option_none;
    auto rules_v   = rules.has_value() ? world.app(option_some, eqsat_rules(world, rules.value())) : option_none;
    auto reaches_v = reaches.has_value() ? world.app(option_some, eqsat_reaches(world, reaches.value())) : option_none;
    auto select_v  = select.has_value() ? world.app(option_some, eqsat_select(world, select.value())) : option_none;

    auto eqsat_config     = world.externals()[world.sym("%eqsat.config")];
    auto config_tuple     = world.tuple({impl_v, cost_fun_v, rulesets_v, rules_v, reaches_v, select_v});
    auto eqsat_config_app = world.app(eqsat_config, config_tuple);

    _config->set_filter(false);
    _config->set_body(eqsat_config_app);
    _config->externalize();
}

} // namespace mim
