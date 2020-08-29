extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;
// use super::callgraph::Callgraph;
// use super::collector::collect_lockguard_info;
// use super::config::{CrateNameLists, CALLCHAIN_DEPTH};
// use super::genkill::GenKill;
// use super::lock::{DoubleLockInfo, LockGuardId, LockGuardInfo};
use rustc_hir::def_id::{DefId, LocalDefId, LOCAL_CRATE};
use rustc_middle::mir::{BasicBlock, Location};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use std::fmt;
use std::fmt::Write;

use super::db::{CrateLifetimeRanges, Input, LifetimeRanges, Output, read_from_json_file, write_to_json_file};
use serde_json;
mod collector;
use collector::collect_lifetime_info;
mod info;
use info::{CrateLocalId, CrateLocalInfo};
mod callgraph;
use callgraph::Callgraph;
mod genkill;
use genkill::GenKill;
mod range;
use range::{get_fn_range, RangeInFile, RangesAcrossFiles};
use std::collections::{HashMap, HashSet};
pub struct LifetimeVisualizer {
    crate_locals: HashMap<CrateLocalId, CrateLocalInfo>,
    crate_callgraph: Callgraph,
}

fn get_fn_path(tcx: &TyCtxt, def_id: DefId) -> String {
    let mut out = String::new();
    write!(&mut out, "{:?}", tcx.def_path(def_id).data).unwrap();
    out
}

impl LifetimeVisualizer {
    pub fn new() -> Self {
        Self {
            crate_locals: HashMap::new(),
            crate_callgraph: Callgraph::new(),
        }
    }

    pub fn analyze(&mut self, tcx: TyCtxt) {
        let ids = tcx.mir_keys(LOCAL_CRATE);
        let fn_ids: Vec<LocalDefId> = ids
            .clone()
            .into_iter()
            .filter(|id| {
                let hir = tcx.hir();
                hir.body_owner_kind(hir.as_local_hir_id(*id))
                    .is_fn_or_closure()
            })
            .collect();
        println!("fn_ids: {:#?}", fn_ids);
        // for fn_id in &fn_ids {
        //     // println!("{}", get_fn_path(&tcx, fn_id.to_def_id()));
        //     // println!("{}", tcx.item_name(fn_id.to_def_id()).to_string());
        //     println!("{}", tcx.def_path_debug_str(fn_id.to_def_id()));
        // }

        let crate_locals: HashMap<LocalDefId, HashMap<CrateLocalId, CrateLocalInfo>> = fn_ids
            .clone()
            .into_iter()
            .filter_map(|fn_id| {
                // println!("{:?}", fn_id);
                let body = tcx.optimized_mir(fn_id);
                let locals = collect_lifetime_info(fn_id, body);
                if locals.is_empty() {
                    None
                } else {
                    Some((fn_id, locals))
                }
            })
            .collect();
        if crate_locals.is_empty() {
            return;
        }

        for (_, info) in &crate_locals {
            for (id, locs) in info {
                self.crate_locals.insert(*id, locs.clone());
            }
        }

        // println!("crate_local: {:#?}", crate_locals);
        // generate callgraph
        for fn_id in &fn_ids {
            self.crate_callgraph
                .generate(*fn_id, tcx.optimized_mir(*fn_id), &fn_ids);
        }
        // self.crate_callgraph._print();
        let mut total_merged_range: HashMap<CrateLocalId, HashMap<String, Vec<RangeInFile>>> =
            HashMap::new();
        for fn_id in &fn_ids {
            total_merged_range.extend(self.check_fn(&tcx, *fn_id).into_iter());
            // break;  // TOOD(Boqin): remove, only for debug
        }
        let crate_name = tcx.crate_name(LOCAL_CRATE).to_string();
        let mut locals: Vec<LifetimeRanges> = Vec::new();
        for (_, local_infos) in crate_locals {
            for (local_id, local_info) in local_infos {
                let span_str = format!("{:?}", local_info.span);
                if let Some(file_ranges) = total_merged_range.get(&local_id) {
                    let mut ranges: Vec<String> = Vec::new();
                    for (filename, ranges_in_file) in file_ranges {
                        for range_in_file in ranges_in_file {
                            let mut range = filename.clone();
                            range.push_str(":");
                            range.push_str(&range_in_file.to_string());
                            ranges.push(range);
                        }
                    }
                    let lifetime_ranges =
                        LifetimeRanges::new(local_id.to_string(), span_str, ranges);
                    locals.push(lifetime_ranges);
                }
                // skip when local_id not in total_merged_range
            }
        }
        let crate_lifetime_ranges = CrateLifetimeRanges::new(crate_name.clone(), locals);
        // println!("{:#?}", crate_lifetime_ranges);
        // println!("{}", serde_json::to_string(&crate_lifetime_ranges).unwrap());
        let json_file_path = format!("lifetime_{}.info", &crate_name);
        write_to_json_file(crate_lifetime_ranges, &json_file_path);
        // println!("{:#?}", read_from_json_file(&json_file_path));
        // println!("{:#?}", read_from_json_file(&json_file_path));
    }

    fn check_fn(
        &mut self,
        tcx: &TyCtxt,
        fn_id: LocalDefId,
    ) -> HashMap<CrateLocalId, HashMap<String, Vec<RangeInFile>>> {
        let body = tcx.optimized_mir(fn_id);
        let mut genkill = GenKill::new(fn_id, body, &self.crate_locals);
        let local_live_locs = genkill.analyze(body);
        let transitive = self.crate_callgraph.gen_transitive();
        let mut local_live_fns: HashMap<CrateLocalId, HashSet<LocalDefId>> = HashMap::new();
        if let Some(callsites) = self.crate_callgraph.get(&fn_id) {
            for (local, locs) in &local_live_locs {
                // Call is the terminator
                let direct_callees: HashSet<_> = callsites
                    .iter()
                    .filter_map(|(bb, callee)| {
                        let term_index = body.basic_blocks()[*bb].statements.len();
                        let term_loc = Location {
                            block: *bb,
                            statement_index: term_index,
                        };
                        if locs.contains(&term_loc) {
                            Some(callee)
                        } else {
                            None
                        }
                    })
                    .collect();
                for callee in direct_callees {
                    let entry = local_live_fns.entry(*local).or_insert_with(HashSet::new);
                    entry.insert(*callee);
                    if let Some(trans) = transitive.get(callee) {
                        for tran in trans {
                            entry.insert(*tran);
                        }
                    }
                }
            }
        }
        // println!("BBs: {:#?}", local_live_locs);
        let mut intra_merged_ranges: HashMap<CrateLocalId, HashMap<String, Vec<RangeInFile>>> =
            HashMap::new();
        for (id, locs) in &local_live_locs {
            let mut ranges_across_files: RangesAcrossFiles = Default::default();
            ranges_across_files.add_locs(locs, body);
            let merged_ranges = ranges_across_files.merge();
            intra_merged_ranges.insert(*id, merged_ranges);
        }
        // println!("FNs: {:#?}", local_live_fns);
        let mut inter_merged_ranges: HashMap<CrateLocalId, HashMap<String, Vec<RangeInFile>>> =
            HashMap::new();
        for (id, fn_ids) in &local_live_fns {
            for fn_id in fn_ids {
                let body = &tcx.optimized_mir(*fn_id);
                let (filename, range_in_file) = get_fn_range(body);
                inter_merged_ranges
                    .entry(*id)
                    .or_insert_with(HashMap::new)
                    .entry(filename)
                    .or_insert_with(Vec::new)
                    .push(range_in_file);
            }
        }
        let mut total_merged_ranges: HashMap<CrateLocalId, HashMap<String, Vec<RangeInFile>>> =
            intra_merged_ranges;
        for (id, inter_ranges) in inter_merged_ranges {
            let ranges_ref = total_merged_ranges.entry(id).or_insert_with(HashMap::new);
            for (filename, ranges) in inter_ranges {
                ranges_ref
                    .entry(filename)
                    .or_insert_with(Vec::new)
                    .extend(ranges.into_iter());
            }
        }
        // println!("Ranges: {:#?}", total_merged_ranges);
        total_merged_ranges
    }

    pub fn visualize(&self, input: Input) -> Output {
        Output {}
    }
}
