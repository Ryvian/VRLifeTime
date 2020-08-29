extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_mir;

use rustc_hir::def_id::{LocalDefId, LOCAL_CRATE};
use rustc_middle::mir::visit::{
    MutatingUseContext, NonMutatingUseContext, NonUseContext, PlaceContext,
};
use rustc_middle::mir::{Body, BasicBlock, Location, Local, LocalInfo, Place, ProjectionElem, TerminatorKind, StatementKind};
use rustc_mir::util::def_use::DefUseAnalysis;
use std::collections::{HashMap, HashSet};

use crate::lifetime_visualizer::info::{CrateLocalId, CrateLocalInfo};

pub fn collect_lifetime_info(fn_id: LocalDefId, body: &Body) -> HashMap<CrateLocalId, CrateLocalInfo> {
    let mut crate_locals: HashMap<CrateLocalId, CrateLocalInfo> = HashMap::new();
    for (local, local_decl) in body.local_decls.iter_enumerated() {
        let crate_local_id = CrateLocalId::new(fn_id, local);
        let crate_local_info = CrateLocalInfo {
            span: local_decl.source_info.span,
            live_locs: HashSet::new(),
            dead_locs: HashSet::new(),
            drop_locs: HashSet::new(),
            move_locs: HashSet::new(),
        };
        crate_locals.insert(crate_local_id, crate_local_info);
    }
    let mut def_use_analysis = DefUseAnalysis::new(body);
    def_use_analysis.analyze(body);
    collect_gen_kill_bbs(crate_locals, body, &def_use_analysis)
}

fn collect_gen_kill_bbs(
    crate_locals: HashMap<CrateLocalId, CrateLocalInfo>,
    _body: &Body,
    def_use_analysis: &DefUseAnalysis,
) -> HashMap<CrateLocalId, CrateLocalInfo> {
    if crate_locals.is_empty() {
        return crate_locals;
    }
    crate_locals
        .into_iter()
        .map(|(id, mut info)| {
            let use_info = def_use_analysis.local_info(id.local);
            for u in &use_info.defs_and_uses {
                match u.context {
                    PlaceContext::NonUse(context) => match context {
                        NonUseContext::StorageLive => { info.live_locs.insert(u.location); },
                        NonUseContext::StorageDead => { info.dead_locs.insert(u.location); },
                        _ => {}
                    },
                    PlaceContext::NonMutatingUse(context) => {
                        if let NonMutatingUseContext::Move = context {
                            info.move_locs.insert(u.location);
                        }
                    },
                    PlaceContext::MutatingUse(context) => {
                        if let MutatingUseContext::Drop = context {
                            info.drop_locs.insert(u.location);
                        }
                    },
                }
            }
            (id, info)
        })
        .collect::<HashMap<_, _>>()
}

fn is_terminator_location(location: &Location, body: &Body) -> bool {
    location.statement_index >= body.basic_blocks()[location.block].statements.len()
}
