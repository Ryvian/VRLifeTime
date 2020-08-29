extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::fmt;

use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::mir::{BasicBlock, Local, Location};
use rustc_middle::ty::Ty;
use rustc_span::Span;

use std::collections::HashSet;

use std::hash::Hash;
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct CrateLocalId {
    pub fn_id: LocalDefId,
    pub local: Local,
}

impl fmt::Display for CrateLocalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}, {:?})", self.fn_id, self.local)
    }
}

impl CrateLocalId {
    pub fn new(fn_id: LocalDefId, local: Local) -> Self {
        Self { fn_id, local }
    }
}

#[derive(Debug, Clone)]
pub struct CrateLocalInfo {
    pub span: Span,
    pub live_locs: HashSet<Location>,
    pub dead_locs: HashSet<Location>,
    pub drop_locs: HashSet<Location>,
    pub move_locs: HashSet<Location>,
}