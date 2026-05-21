#include <mim/plug/eqsat/eqsat.h>
#include <mim/plug/eqsat/phase/eqsat_phase.h>
#include <mim/plug/eqsat/phase/rewrite_egg.h>
#include <mim/plug/eqsat/phase/rewrite_slotted.h>

namespace mim::plug::eqsat {

void EqsatPhase::start() {
    bool slotted = true;

    // Infers whether to use 'egg' or 'slotted-egraphs' based on a
    // config lambda with the signature '[] -> %eqsat.Impl'
    // Each rewrite phase will further infer config values from
    // config functions and internalize all of them, including this one.
    for (auto def : world().externals().mutate()) {
        if (auto lam = def->isa<Lam>()) {
            if (Axm::isa<eqsat::Impl>(lam->codom())) {
                auto body = lam->as<Lam>()->body();
                if (Axm::isa<eqsat::slotted>(body))
                    slotted = true;
                else if (Axm::isa<eqsat::egg>(body))
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
