use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::fs::File;
use std::io::prelude::*;
use std::fmt;
use std::cmp::Ordering;


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Input {
    pub root: String,
    pub file: String,
    pub pos: String,
}

pub struct Output {}
// #[derive(Serialize, Deserialize)]
// pub struct Output {
//     "src/utils.rs": "12:25: 15:14, 19:12: 22:19",
// 	"src/backend.rs": "222:14: 229:10"
// }

#[cfg(test)]
mod test {
    #[test]
    fn test_input() {
        use super::*;
        let input_json = r#"
        {
	        "root": "/home/user/rust_projects/tikv",
	        "file": "src/utils.rs",
	        "pos": "12:10: 12:31"
        }"#;
        let input_struct: Input = serde_json::from_str(input_json).unwrap();
        let input_struct_2 = Input {
            root: "/home/user/rust_projects/tikv".to_string(),
            file: "src/utils.rs".to_string(),
            pos: "12:10: 12:31".to_string(),
        };
        assert_eq!(input_struct, input_struct_2);
    }

    #[test]
    fn test_lifetime_ranges() {
        use super::*;
        let lifetime_ranges_json = r#"
        {
	        "fn_id_local": "main, _1",
	        "span": "src/utils.rs:12:10: 12:31",
	        "ranges": [
                "src/utils.rs:12:10: 12:31",
                "src/utils.rs:15:10: 18:29",
                "others/src/xyz.rs:19:10: 25.19"
            ]
        }"#;
        let lifetime_ranges: LifetimeRanges = serde_json::from_str(lifetime_ranges_json).unwrap();
        // println!("{:#?}", lifetime_ranges);
        // println!("{:?}", serde_json::to_string(&lifetime_ranges).unwrap());
        // let input_struct_2 = Input {
        //     root: "/home/user/rust_projects/tikv".to_string(),
        //     file: "src/utils.rs".to_string(),
        //     pos: "12:10: 12:31".to_string(),
        // };
        // assert_eq!(input_struct, input_struct_2);
        let crate_lifetime_ranges = CrateLifetimeRanges {
            crate_name: "tikv".to_string(),
            locals: vec![lifetime_ranges.clone(), lifetime_ranges.clone(), lifetime_ranges.clone()],
        };
        println!("{:?}", serde_json::to_string(&crate_lifetime_ranges).unwrap());
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LifetimeRanges {
    fn_id_local: String,
    span: String,
    ranges: Vec<String>,
}
impl LifetimeRanges {
    pub fn new(fn_id_local: String, span: String, ranges: Vec<String>) -> Self  {
        Self {
            fn_id_local,
            span,
            ranges,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CrateLifetimeRanges {
    crate_name: String,
    locals: Vec<LifetimeRanges>
}

impl CrateLifetimeRanges {
    pub fn new(crate_name: String, locals: Vec<LifetimeRanges>) -> Self {
        Self {
            crate_name, 
            locals,
        }
    }
}

pub fn write_to_json_file(crate_lifetime_ranges: CrateLifetimeRanges, file_path: &str) {
    let ranges_json_str = serde_json::to_string(&crate_lifetime_ranges).unwrap();
    let mut f = File::create(file_path).unwrap();
    f.write_all(ranges_json_str.as_bytes()).unwrap();
    f.sync_all().unwrap();
}

pub fn read_from_json_file(file_path: &str) -> CrateLifetimeRanges {
    let mut file = File::open(file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    serde_json::from_str(&contents).unwrap()
}

#[derive(Debug, Clone)]
pub struct LifetimeRangesForQuery {
    fn_id_local: String,
    span: SpanRange,
    ranges: Vec<String>,
}

impl LifetimeRangesForQuery {
    fn from_lifetime_ranges(lifetime_ranges: LifetimeRanges) -> Self {
        Self {
            fn_id_local: lifetime_ranges.fn_id_local,
            span: SpanRange::from_str(&lifetime_ranges.span),
            ranges: lifetime_ranges.ranges,
        }
    }
    fn strict_eq(&self, span: &SpanRange) -> bool {
        self.span == *span
    }

    fn contained_by_span(&self, span: &SpanRange) -> bool {
        self.span.contained_by_span(span)
    }

    pub fn get_ranges(&self) -> &Vec<String> {
        &self.ranges
    }
}
#[derive(Debug)]
pub struct CrateLifetimeRangesForQuery {
    crate_name: String,
    locals_for_query: Vec<LifetimeRangesForQuery>,
}

impl CrateLifetimeRangesForQuery {
    pub fn from_crate_lifetime_ranges(crate_lifetime_ranges: CrateLifetimeRanges) -> Self {
        Self {
            crate_name: crate_lifetime_ranges.crate_name,
            locals_for_query: crate_lifetime_ranges.locals.into_iter().map(|r| LifetimeRangesForQuery::from_lifetime_ranges(r)).collect(),
        }
    }
    pub fn filter_by_span(&self, span: &SpanRange, is_strict: bool) -> Vec<LifetimeRangesForQuery> {
        self.locals_for_query.clone().into_iter().filter_map(|r|
            if !is_strict {
                if r.contained_by_span(span) {
                    Some(r)
                } else {
                    None
                }
            } else {
                if r.strict_eq(span) {
                    Some(r)
                } else {
                    None
                }
            }
        ).collect()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpanRange {
    filename: String,
    begin: LineCol,
    end: LineCol,
}

impl fmt::Display for SpanRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}: {}:{})", self.filename, self.begin.0, self.begin.1, self.end.0, self.end.1)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct LineCol(u64, u64);

impl PartialOrd for LineCol {
    fn partial_cmp(&self, other: &LineCol) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LineCol {
    fn cmp(&self, other: &LineCol) -> Ordering {
        if self.0 != other.0 {
            self.0.cmp(&other.0)
        } else {
            self.1.cmp(&other.1)
        }
    }
}

impl SpanRange {
    pub fn from_str(span_str: &str) -> Self {
        let labels: Vec<&str> = span_str.split(":").collect();
        assert!(labels.len() == 5);
        let filename = labels[0];
        let line_0: u64 = labels[1].parse().unwrap();
        let col_0: u64 = labels[2].parse().unwrap();
        let line_1: u64 = labels[3][1..].parse().unwrap();
        let col_1: u64 = labels[4].parse().unwrap();
        Self { filename: filename.to_string(), begin: LineCol(line_0, col_0), end: LineCol(line_1, col_1) }
    }

    fn contained_by_span(&self, other: &SpanRange) -> bool {
        if self.filename != other.filename {
            false
        } else {
            other.begin <= self.begin && self.end <= other.end
        }
    }

    fn contained_by_str(&self, span_str: &str) -> bool {
        let other = SpanRange::from_str(span_str);
        self.contained_by_span(&other)
    }

    pub fn split(&self) -> (String, String) {
        (self.filename.clone(), format!("{}:{}: {}:{}", self.begin.0, self.begin.1, self.end.0, self.end.1))
    }
}

