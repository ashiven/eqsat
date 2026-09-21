#include "mim/plug/eqsat/eqsat.h"

#include <mim/phase.h>
#include <mim/plugin.h>

#include "mim/plug/eqsat/phase/eqsat_phase.h"
#include "mim/plug/eqsat/phase/rewrite_egg.h"
#include "mim/plug/eqsat/phase/rewrite_slotted.h"

using namespace mim;
using namespace mim::plug;

static void reg_phases(Flags2Phases& phases) {
    Phase::hook<eqsat::eqsat_phase, eqsat::EqsatPhase>(phases);
    Phase::hook<eqsat::rewrite_egg, eqsat::RewriteEgg>(phases);
    Phase::hook<eqsat::rewrite_slotted, eqsat::RewriteSlotted>(phases);
}

MIM_PLUGIN_ENTRY(eqsat) { plugin.register_phases = reg_phases; }
