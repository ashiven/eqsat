#include <mim/plug/eqsat/eqsat.h>
#include <mim/plug/eqsat/phase/eqsat_phase.h>
#include <mim/plug/eqsat/phase/rewrite_egg.h>
#include <mim/plug/eqsat/phase/rewrite_slotted.h>

namespace mim::plug::eqsat {

void EqsatPhase::start() {
    bool slotted = true;

    // Infers whether to use 'egg' or 'slotted-egraphs' based on a
    // config lambda with the signature '[] -> %eqsat.Configs'
    // Each rewrite phase will further infer config values from
    // config functions and internalize all of them, including this one.
    for (auto def : world().externals().mutate()) {
        if (auto lam = def->isa<Lam>()) {
            if (lam->codom()->sym().str() == "%eqsat.Configs") {
                auto body        = lam->as<Lam>()->body();
                auto config_defs = body->as<Tuple>()->ops();
                for (auto config_def : config_defs) {
                    auto config_opt = get_config_option(config_def);
                    if (config_opt.has_value()) {
                        auto config_val = config_opt.value();
                        if (Axm::isa<eqsat::slotted>(config_val))
                            slotted = true;
                        else if (Axm::isa<eqsat::egg>(config_val))
                            slotted = false;
                    }
                }
            }
        }
    }

    if (slotted) {
        RewriteSlotted rewrite_slotted(world(), "rewrite_slotted");
        rewrite_slotted.start();
    } else {
        RewriteEgg rewrite_egg(world(), "rewrite_egg");
        rewrite_egg.start();
    }
}

}; // namespace mim::plug::eqsat
