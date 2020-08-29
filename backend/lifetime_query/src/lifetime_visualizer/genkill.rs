extern crate rustc_hir;
extern crate rustc_middle;

use crate::lifetime_visualizer::info::{CrateLocalId, CrateLocalInfo};
use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::{BasicBlock, Body, Location, TerminatorKind, START_BLOCK};
use std::collections::HashMap;
use std::collections::HashSet;

const RUN_LIMIT: u32 = 10000;
pub struct GenKill {
    gen: HashMap<Location, HashSet<CrateLocalId>>,
    kill: HashMap<Location, HashSet<CrateLocalId>>,
    before: HashMap<Location, HashSet<CrateLocalId>>,
    after: HashMap<Location, HashSet<CrateLocalId>>,
    worklist: Vec<Location>,
}

impl GenKill {
    pub fn new(
        fn_id: LocalDefId,
        body: &Body,
        crate_locals: &HashMap<CrateLocalId, CrateLocalInfo>,
    ) -> GenKill {
        let mut gen: HashMap<Location, HashSet<CrateLocalId>> = HashMap::new();
        let mut kill: HashMap<Location, HashSet<CrateLocalId>> = HashMap::new();
        let mut before: HashMap<Location, HashSet<CrateLocalId>> = HashMap::new();
        let mut after: HashMap<Location, HashSet<CrateLocalId>> = HashMap::new();
        let mut worklist = Vec::new();
        for (id, info) in crate_locals {
            if id.fn_id != fn_id {
                continue;
            }
            for loc in info.live_locs.iter() {
                gen.entry(*loc).or_insert_with(HashSet::new).insert(*id);
            }
            for loc in info.dead_locs.iter() {
                kill.entry(*loc).or_insert_with(HashSet::new).insert(*id);
            }
            for loc in info.drop_locs.iter() {
                kill.entry(*loc).or_insert_with(HashSet::new).insert(*id);
            }
            for loc in info.move_locs.iter() {
                kill.entry(*loc).or_insert_with(HashSet::new).insert(*id);
            }
        }
        for (bb, bb_data) in body.basic_blocks().iter_enumerated() {
            let statements_len = bb_data.statements.len();
            for ii in 0..statements_len {
                let loc = Location {
                    block: bb,
                    statement_index: ii,
                };
                before.insert(loc, HashSet::new());
                after.insert(loc, HashSet::new());
            }
            if let Some(_) = bb_data.terminator {
                let loc = Location {
                    block: bb,
                    statement_index: statements_len,
                };
                before.insert(loc, HashSet::new());
                after.insert(loc, HashSet::new());
            }
        }
        // for (loc, ids) in &gen {
        //     if loc.block == START_BLOCK {
        //         after.get_mut(loc).unwrap().extend(ids.iter());
        //     }
        // }
        worklist.push(Location::START);
        // println!("init_before: {:#?}", before);
        Self {
            gen,
            kill,
            before,
            after,
            worklist,
        }
    }

    pub fn analyze(&mut self, body: &Body) -> HashMap<CrateLocalId, HashSet<Location>> {
        let mut count: u32 = 0;
        while !self.worklist.is_empty() && count <= RUN_LIMIT {
            count += 1;
            let cur = self.worklist.pop().unwrap();
            let mut new_before: HashSet<CrateLocalId> = HashSet::new();
            // copy after[prev] to new_before
            let prevs = get_predecessors(&cur, body);
            if !prevs.is_empty() {
                for prev in prevs {
                    new_before.extend(self.after[&prev].iter().clone());
                    self.before
                        .get_mut(&cur)
                        .unwrap()
                        .extend(new_before.iter().clone());
                }
            } else {
                new_before.extend(self.before[&cur].iter().clone());
            }
            if let Some(infos) = self.gen.get(&cur) {
                self.union_gen_set(&mut new_before, infos);
            }
            if let Some(infos) = self.kill.get(&cur) {
                self.kill_kill_set(&mut new_before, infos);
            }
            if !self.compare_infos(&new_before, &self.after[&cur]) {
                self.after.insert(cur, new_before);
                self.worklist.extend(get_successors(&cur, body).into_iter());
            }
        }
        assert!(count <= RUN_LIMIT);
        let mut crate_local_live_locs: HashMap<CrateLocalId, HashSet<Location>> = HashMap::new();
        for (bb, bb_data) in body.basic_blocks().iter_enumerated() {
            let statements_len = bb_data.statements.len();
            for ii in 0..statements_len {
                let loc = Location {
                    block: bb,
                    statement_index: ii,
                };
                if let Some(context) = self.get_live_infos(&loc) {
                    for id in context {
                        crate_local_live_locs
                            .entry(*id)
                            .or_insert_with(HashSet::new)
                            .insert(loc);
                    }
                }
            }
            if let Some(ref term) = bb_data.terminator {
                // neglect resume and unreachable
                match term.kind {
                    TerminatorKind::Resume | TerminatorKind::Unreachable => {
                        continue;
                    }
                    _ => {}
                }
                let loc = Location {
                    block: bb,
                    statement_index: statements_len,
                };
                if let Some(context) = self.get_live_infos(&loc) {
                    for id in context {
                        crate_local_live_locs
                            .entry(*id)
                            .or_insert_with(HashSet::new)
                            .insert(loc);
                    }
                }
            }
        }
        for (loc, ids) in &self.gen {
            for id in ids {
                crate_local_live_locs
                    .entry(*id)
                    .or_insert_with(HashSet::new)
                    .insert(*loc);
            }
        }
        crate_local_live_locs
    }

    pub fn get_live_infos(&self, loc: &Location) -> Option<&HashSet<CrateLocalId>> {
        if let Some(context) = self.before.get(loc) {
            if !context.is_empty() {
                return Some(context);
            } else {
                return None;
            }
        }
        None
    }

    fn union_gen_set(&self, new_before: &mut HashSet<CrateLocalId>, infos: &HashSet<CrateLocalId>) {
        new_before.extend(infos.iter().clone());
    }

    fn kill_kill_set(&self, new_before: &mut HashSet<CrateLocalId>, infos: &HashSet<CrateLocalId>) {
        new_before.retain(move |b| !infos.contains(b));
    }

    fn compare_infos(&self, lhs: &HashSet<CrateLocalId>, rhs: &HashSet<CrateLocalId>) -> bool {
        lhs == rhs
        // if lhs.len() != rhs.len() {
        //     return false;
        // }
        // let rhs_info = rhs
        //     .iter()
        //     .map(|r| self.crate_locals.get(r).unwrap())
        //     .collect::<Vec<_>>();
        // lhs.iter()
        //     .map(move |l| self.crate_locals.get(l).unwrap())
        //     .all(move |li| rhs_info.contains(&li))
    }
}

fn get_predecessors(loc: &Location, body: &Body) -> Vec<Location> {
    if loc.statement_index > 0 {
        vec![Location {
            block: loc.block,
            statement_index: loc.statement_index - 1,
        }]
    } else {
        // start inst of BB
        let mut preds: Vec<Location> = Vec::new();
        for prev_bb in &body.predecessors()[loc.block] {
            let prev_bb_data = &body.basic_blocks()[*prev_bb];
            if let Some(_) = prev_bb_data.terminator {
                preds.push(Location {
                    block: *prev_bb,
                    statement_index: prev_bb_data.statements.len(),
                })
            }
        }
        preds
    }
}

fn get_successors(loc: &Location, body: &Body) -> Vec<Location> {
    let statments_len = body.basic_blocks()[loc.block].statements.len();
    if loc.statement_index < statments_len {
        vec![Location {
            block: loc.block,
            statement_index: loc.statement_index + 1,
        }]
    } else {
        body.basic_blocks()[loc.block]
            .terminator()
            .successors()
            .map(|b| Location {
                block: *b,
                statement_index: 0,
            })
            .collect::<_>()
    }
}
