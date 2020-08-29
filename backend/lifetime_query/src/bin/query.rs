use vrlifetime_backend::db::{read_from_json_file, CrateLifetimeRanges, CrateLifetimeRangesForQuery, Input, SpanRange};
use std::env;
use std::{fs, io};
use std::fs::{File, read_dir};
use std::path::MAIN_SEPARATOR;
use std::collections::HashMap;

fn main() {
    // let crate_name = "vec_uaf";
    let args: Vec<String> = env::args().collect();
    // let input_json = r#"
    //     {
	//         "root": "/home/boqin/Projects/HackRust/vrlifetime-backend/examples/vec-uaf",
	//         "file": "src/main.rs",
	//         "pos": "4:9: 4:10"
    //     }"#;
    // println!("{}", &args[1]);
    let input_struct: Input = serde_json::from_str(&args[1]).unwrap();
    // println!("{:?}", input_struct);

    // let crate_lifetime_ranges = read_from_json_file(&args[1]);
    // let query_db = CrateLifetimeRangesForQuery::from_crate_lifetime_ranges(crate_lifetime_ranges);
    
    let root_dir = input_struct.root;
    let lifetime_infos = filter_query_db_files(&root_dir);
    let query_dbs: Vec<CrateLifetimeRangesForQuery> = lifetime_infos.into_iter().map(|lifetime_info| {
        let lifetime_info_path = format!("{}{}{}", root_dir, MAIN_SEPARATOR, lifetime_info);
        CrateLifetimeRangesForQuery::from_crate_lifetime_ranges(read_from_json_file(&lifetime_info_path))
    }).collect();
    // println!("{:#?}", query_dbs);
    let query_span = SpanRange::from_str(&format!("{}:{}", input_struct.file, input_struct.pos));
    let mut results = Vec::new();
    for query_db in query_dbs {
        results.push(query_db.filter_by_span(&query_span, true));
    }

    let mut final_results = Vec::new();
    for result in results {
        for res in result {
            let ranges = res.get_ranges().clone();
            final_results.extend(ranges.into_iter());
        }
    }
    // println!("{:#?}", final_results);
    // println!("{:#?}", merge_ranges(final_results));
    let merged = merge_ranges(final_results);
    let mut output_str: String = "{\n".to_string();
    
    for (filename, ranges) in merged {
        let line_str = ranges.into_iter().fold(String::new(), |s, r| s + &r + ", ");
        output_str.push_str(&format!("\t\"{}\":\"{}\",\n", &filename, &line_str[..line_str.len()-2]));
    }

    output_str.pop();
    output_str.pop();
    output_str.push_str("\n}");
    println!("{}", output_str);
}

fn merge_ranges(ranges: Vec<String>) -> HashMap<String, Vec<String>> {
    let mut res = HashMap::new();
    for range in ranges {
        let (filename, new_range) = SpanRange::from_str(&range).split();
        res.entry(filename).or_insert_with(Vec::new).push(new_range);
    }
    res
}

fn filter_query_db_files(root_dir: &str) -> Vec<String> {
    // let mut result = Vec::new();
    fs::read_dir(root_dir).unwrap()
    .filter_map(|res| {
        if let Ok(entry) = res {
            if let Ok(file_name) = entry.file_name().into_string() {
                if file_name.starts_with("lifetime_") && file_name.ends_with(".info") {
                    return Some(file_name);
                }
            }
        }
        None
    })
    .collect::<Vec<String>>()
}