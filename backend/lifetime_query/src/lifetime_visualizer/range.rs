extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;
use rustc_hir::def_id::{LocalDefId, LOCAL_CRATE};
use rustc_middle::mir::{BasicBlock, Body, Location, START_BLOCK, TerminatorKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use std::cmp::Ordering;

use std::fmt;

use std::collections::{HashMap, HashSet, LinkedList};
// use crate::lifetime_visualizer::info::{CrateLocalId};

// collect span for BasicBlock
// collect span for Function

// fn merge_location(HashMap<CrateLocalId, HashSet<Location>>) -> HashMap<> {

// }

pub struct Range {
    filename: String,
    begin: (u64, u64),
    end: (u64, u64),
}

impl Range {
    fn new(filename: &str, begin: (u64, u64), end: (u64, u64)) -> Self {
        Self {
            filename: filename.to_string(),
            begin,
            end,
        }
    }

    // fn union(&self, other: &Range) -> Self {

    // }
}

#[derive(Eq, Clone, Copy, Hash, Debug)]
struct PosInFile(u64, u64);

impl PartialEq for PosInFile {
    fn eq(&self, other: &PosInFile) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl PartialOrd for PosInFile {
    fn partial_cmp(&self, other: &PosInFile) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PosInFile {
    fn cmp(&self, other: &PosInFile) -> Ordering {
        if self.0 != other.0 {
            self.0.cmp(&other.0)
        } else {
            self.1.cmp(&other.1)
        }
    }
}
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct RangeInFile(PosInFile, PosInFile);

impl RangeInFile {
    fn union_in_place(&mut self, other: &RangeInFile) -> bool {
        if other.1 < self.0 || self.1 < other.0 {
            false
        } else if self.0 <= other.1 && other.1 <= self.1 {
            if other.0 < self.0 {
                self.0 = other.0;
            }
            true
        } else if self.1 < other.1 {
            if other.0 < self.0 {
                self.0 = other.0;
            }
            self.1 = other.1;
            true
        } else {
            false
        }
    }
}

impl fmt::Display for RangeInFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}:{}", (self.0).0, (self.0).1, (self.1).0, (self.1).1)
    }
}
#[derive(Default, Debug)]
struct RangesInFile {
    ranges: HashSet<RangeInFile>,
}

impl RangesInFile {
    fn new(ranges: HashSet<RangeInFile>) -> Self {
        Self {
            ranges
        }
    }
    fn add(&mut self, range: RangeInFile) {
        self.ranges.insert(range);
    }
    fn merge(self) -> Vec<RangeInFile> {
        let mut result: Vec<RangeInFile> = Vec::new();
        let mut worklist: Vec<RangeInFile> = self.ranges.into_iter().collect();
        while let Some(mut cur) = worklist.pop() {
            let old_len = worklist.len();
            worklist.retain(|r| 
                if cur.union_in_place(r) {
                    false
                } else {
                    true
                }
            );
            if old_len != worklist.len() {
               worklist.push(cur);
            } else {
                result.push(cur);
            }
        }
        result
    }
}

#[derive(Default, Debug)]
pub struct RangesAcrossFiles {
    ranges: HashMap<String, HashSet<RangeInFile>>,
}

impl RangesAcrossFiles {
    pub fn add_locs(&mut self, locs: &HashSet<Location>, body: &Body) {
        for loc in locs {
            let (filename, range) = parse_span(&get_span(loc, body));
            self.ranges.entry(filename).or_insert_with(HashSet::new).insert(range);
        }
    }
    pub fn merge(self) -> HashMap<String, Vec<RangeInFile>> {
        self.ranges.into_iter().map(|(filename, file_ranges)| 
            (filename, RangesInFile::new(file_ranges).merge())
        ).collect()
    }
}

// get fn ranges: 
// begin: START_BLOCK
// end: all the terminators
// merge them
// Can a function spans across multiple files? Need to be verified. I assume it cannot for now.
pub fn get_fn_range(body: &Body) -> (String, RangeInFile) {
    let mut term_spans: Vec<Span> = Vec::new();
    for (_, bb_data) in body.basic_blocks().iter_enumerated() {
        let term = bb_data.terminator();
        match term.kind {
            TerminatorKind::Resume | TerminatorKind::Unreachable => { continue; },
            _ => {},
        }
        term_spans.push(bb_data.terminator().source_info.span);
    }
    let mut end_pos_across_files: HashMap<String, PosInFile> = HashMap::new();
    for term_span in term_spans {
        let (filename, term_range) = parse_span(&term_span);
        let term_end_pos = term_range.1;
        if let Some(end_pos) = end_pos_across_files.get_mut(&filename) {
            if *end_pos < term_end_pos {
                *end_pos = term_end_pos;
            }
        } else {
            end_pos_across_files.insert(filename, term_end_pos);
        }
    }
    let (filename, start_range) = parse_span(&body.basic_blocks()[START_BLOCK].statements[0].source_info.span);
    let start_begin_pos = start_range.0;
    let range_in_file = RangeInFile(start_begin_pos, *end_pos_across_files.get(&filename).unwrap());
    (filename, range_in_file)
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_ranges_in_file() {
        let rg1 = RangeInFile(PosInFile(4, 13), PosInFile(7, 6));
        let rg2 = RangeInFile(PosInFile(8, 5), PosInFile(8, 23));
        let rg3 = RangeInFile(PosInFile(4, 9), PosInFile(4, 10));
        let rg4 = RangeInFile(PosInFile(9, 1), PosInFile(9, 2));
        let mut ranges_in_file: RangesInFile = Default::default();
        ranges_in_file.add(rg1);
        ranges_in_file.add(rg2);
        ranges_in_file.add(rg3);
        ranges_in_file.add(rg4);
        let ranges = ranges_in_file.merge();
        println!("{:#?}", ranges);
    }

    #[test]
    fn test_parse_span_str() {
        assert_eq!(parse_span_str("src/main.rs:8:14: 8:20"), RangeInFile(PosInFile(8, 14), PosInFile(8, 20)));
    }
    
    #[test]
    fn test_ranges_in_file_many() {
        let mut ranges_in_file: RangesInFile = Default::default();
        let spans_str = ["src/main.rs:8:14: 8:20","src/main.rs:8:18: 8:19","src/main.rs:5:25: 5:26","src/main.rs:6:17: 6:32","src/main.rs:5:39: 5:40","src/main.rs:8:20: 8:21","src/main.rs:4:9: 4:10","src/main.rs:8:5: 8:23","src/main.rs:5:9: 5:16","src/main.rs:5:25: 5:26","src/main.rs:4:19: 4:23","src/main.rs:5:20: 5:40","src/main.rs:5:39: 5:40","src/main.rs:8:14: 8:20","src/main.rs:5:14: 5:15","src/main.rs:9:1: 9:2","src/main.rs:5:40: 5:41","src/main.rs:8:18: 8:19","src/main.rs:5:9: 5:16","src/main.rs:5:14: 5:15","src/main.rs:5:39: 5:40","src/main.rs:8:19: 8:20","src/main.rs:5:39: 5:40","src/main.rs:4:13: 7:6"];
        for span_str in &spans_str {
            let range = parse_span_str(span_str);
            ranges_in_file.add(range);
        }
        println!("{:#?}", ranges_in_file.merge());
    }
}

pub fn merge_spans(locs: &HashSet<Location>, body: &Body) {
    let mut spans: HashSet<Span> = HashSet::new();
    for loc in locs {
        spans.insert(get_span(loc, body));
    }
    println!("{:#?}", spans);
}

// e.g.
// src/main.rs:4:13: 7:6
fn parse_span_str(span_str: &str) -> RangeInFile {
    let labels: Vec<&str> = span_str.split(":").collect();
    assert!(labels.len() == 5);
    let filename = labels[0];
    let line_0: u64 = labels[1].parse().unwrap();
    let col_0: u64 = labels[2].parse().unwrap();
    let line_1: u64 = labels[3][1..].parse().unwrap();
    let col_1: u64 = labels[4].parse().unwrap();
    RangeInFile(PosInFile(line_0, col_0), PosInFile(line_1, col_1))
}

pub fn parse_span(span: &Span) -> (String, RangeInFile) {
    let span_str = format!("{:?}", span);
    let labels: Vec<&str> = span_str.split(":").collect();
    assert!(labels.len() == 5);
    let filename = labels[0];
    let line_0: u64 = labels[1].parse().unwrap();
    let col_0: u64 = labels[2].parse().unwrap();
    let line_1: u64 = labels[3][1..].parse().unwrap();
    let col_1: u64 = labels[4].parse().unwrap();
    (filename.to_string(), RangeInFile(PosInFile(line_0, col_0), PosInFile(line_1, col_1)))
}

fn get_span(loc: &Location, body: &Body) -> Span {
    let bb_data = &body.basic_blocks()[loc.block];
    if loc.statement_index < bb_data.statements.len() {
        bb_data.statements[loc.statement_index].source_info.span
    } else {
        bb_data.terminator().source_info.span
    }
}
