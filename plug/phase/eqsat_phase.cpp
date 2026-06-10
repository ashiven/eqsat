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
            if (auto arr = lam->codom()->isa<Arr>();
                (arr && Axm::isa<eqsat::Config>(arr->body())) || Axm::isa<eqsat::Config>(lam->codom())) {
                auto body               = lam->as<Lam>()->body();
                DefVec singleton_config = {body};
                auto config_vals        = body->isa<Tuple>() ? body->as<Tuple>()->ops() : Defs(singleton_config);
                for (auto config_val : config_vals)
                    if (Axm::isa<eqsat::slotted>(config_val))
                        slotted = true;
                    else if (Axm::isa<eqsat::egg>(config_val))
                        slotted = false;
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
