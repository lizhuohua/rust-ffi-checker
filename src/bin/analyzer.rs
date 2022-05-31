/// The static analyzer and bug detector
use rust_ffi_checker::analysis::context::GlobalContext;
use rust_ffi_checker::analysis::option::AnalysisOption;
use rust_ffi_checker::analysis::taint_analysis::StaticAnalysis;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

fn main() {
    // Initialize logger
    pretty_env_logger::init();

    // Get analysis options from command line arguments
    // E.g., the entry point, the LLVM bitcode, etc.
    let options = AnalysisOption::from_args(env::args());

    // Initialize global context
    // Use `Rc` and `RefCell` to allow mutable aliases, since the context is shared by multiple instances
    let context = Rc::new(RefCell::new(GlobalContext::new(options)));

    // Start analysis from the entry point
    let mut analysis = StaticAnalysis::new(context);
    analysis.run();

    // Output diagnostic messages
    analysis.output_diagnoses();
}
