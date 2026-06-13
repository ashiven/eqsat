#include "mim/plug/eqsat/eqsat.h"

#include <mim/plugin.h>

#include "mim/plug/eqsat/phase/eqsat_phase.h"
#include "mim/plug/eqsat/phase/rewrite_egg.h"
#include "mim/plug/eqsat/phase/rewrite_slotted.h"

using namespace mim;
using namespace mim::plug;

void reg_stages(Flags2Stages& stages) {
    Stage::hook<eqsat::eqsat_phase, eqsat::EqsatPhase>(stages);
    Stage::hook<eqsat::rewrite_egg, eqsat::RewriteEgg>(stages);
    Stage::hook<eqsat::rewrite_slotted, eqsat::RewriteSlotted>(stages);
}

/// Registers normalizers as well as Phase%s and Pass%es for the Axm%s of this Plugin.
extern "C" MIM_EXPORT Plugin mim_get_plugin() { return {"eqsat", MIM_VERSION, nullptr, reg_stages}; }
