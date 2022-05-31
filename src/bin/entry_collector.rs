// The customized compiler that adds an additional analysis pass.
// Used for extracting appropriate entry points from Rust HIR.

#![feature(rustc_private)]
#![feature(box_patterns)]
use std::env;
use std::process::{self};

use rust_ffi_checker::entry_collection::callback;
use rust_ffi_checker::utils;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;

// use log::info;

use rustc_session::config::ErrorOutputType;
use rustc_session::early_error;

/// Exit status code used for successful compilation and help output.
pub const EXIT_SUCCESS: i32 = 0;

/// Exit status code used for compilation failures and invalid flags.
pub const EXIT_FAILURE: i32 = 1;

fn main() {
    // Initialize logger
    pretty_env_logger::init();

    let result = rustc_driver::catch_fatal_errors(move || {
        let mut rustc_args = env::args_os()
            .enumerate()
            .map(|(i, arg)| {
                arg.into_string().unwrap_or_else(|arg| {
                    early_error(
                        ErrorOutputType::default(),
                        &format!("Argument {} is not valid Unicode: {:?}", i, arg),
                    )
                })
            })
            .collect::<Vec<_>>();

        if let Some(sysroot) = utils::compile_time_sysroot() {
            let sysroot_flag = "--sysroot";
            if !rustc_args.iter().any(|e| e == sysroot_flag) {
                // We need to overwrite the default that librustc would compute.
                rustc_args.push(sysroot_flag.to_owned());
                rustc_args.push(sysroot);
            }
        }

        let mut callbacks = callback::EntryCollectorCallbacks::new();

        let run_compiler = rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks);
        run_compiler.run()
    })
    .and_then(|result| result);

    let exit_code = match result {
        Ok(_) => EXIT_SUCCESS,
        Err(_) => EXIT_FAILURE,
    };

    process::exit(exit_code);
}
