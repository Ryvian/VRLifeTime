#![feature(rustc_private)]
#![feature(box_patterns)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_hir;

use rustc_driver::Compilation;
use rustc_interface::{interface, Queries};
use rustc_hir::def_id::LOCAL_CRATE;

mod db;
pub mod lifetime_visualizer;
use lifetime_visualizer::LifetimeVisualizer;
struct VRLifetimeCallbacks;

impl rustc_driver::Callbacks for VRLifetimeCallbacks {
    fn after_analysis<'tcx>(
        &mut self,
        compiler: &interface::Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        compiler.session().abort_if_errors();
        queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
           dbg!(tcx.crate_name(LOCAL_CRATE));
           let mut vis = LifetimeVisualizer::new();
           vis.analyze(tcx);
        });
        Compilation::Continue
    }
}

fn compile_time_sysroot() -> Option<String> {
    if option_env!("RUST_STAGE").is_some() {
        return None;
    }
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    Some(match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => option_env!("RUST_SYSROOT")
            .expect("To build VRLifetime without rustup, set the `RUST_SYSROOT` env var at build time")
            .to_owned(),
    })
}

fn main() {
    let mut rustc_args = vec![];
    for arg in std::env::args() {
        rustc_args.push(arg);
    }

    if let Some(sysroot) = compile_time_sysroot() {
        let sysroot_flag = "--sysroot";
        if !rustc_args.iter().any(|e| e == sysroot_flag) {
            // We need to overwrite the default that librustc would compute.
            rustc_args.push(sysroot_flag.to_owned());
            rustc_args.push(sysroot);
        }
    }

    let result = rustc_driver::catch_fatal_errors(move || {
        rustc_driver::run_compiler(&rustc_args, &mut VRLifetimeCallbacks, None, None)
    })
    .and_then(|result| result);

    std::process::exit(result.is_err() as i32);
}
