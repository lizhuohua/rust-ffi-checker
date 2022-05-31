use crate::analysis::diagnosis::Diagnosis;
use crate::analysis::known_names::KnownNames;
use crate::analysis::option::AnalysisOption;
use crate::analysis::summary::SummaryCache;
use llvm_ir::Function;
use llvm_ir::Module;
use std::collections::{HashMap, HashSet};

// Stores all the information that the static analyzer needs
pub struct GlobalContext {
    // A map that stores all the functions that may be analyzed
    pub functions: HashMap<String, Function>,
    // Stores the function summary to avoid re-computation
    pub summary_cache: SummaryCache,
    // Stores analysis options read from files and command line arguments
    pub analysis_option: AnalysisOption,
    // Stores the name of tainted sources and sinks
    pub known_names: KnownNames,
    // Diagnoses
    pub diagnoses: HashSet<Diagnosis>,
    // Current call stack, this will be emptied when starting a new function analysis
    // pub call_stack: Vec<String>,
}

impl GlobalContext {
    pub fn new(options: AnalysisOption) -> Self {
        let mut functions = HashMap::new();

        for filepath in &options.bitcode_file_paths {
            // Get LLVM bitcode files from command line arguments
            let module = Module::from_bc_path(filepath).expect("LLVM bitcode file not found!");

            // Get all the functions from modules
            for func in module.functions {
                functions.insert(func.name.clone(), func);
            }
        }

        Self {
            functions: functions,
            summary_cache: SummaryCache::default(),
            analysis_option: options,
            known_names: KnownNames::default(),
            diagnoses: HashSet::new(),
            // call_stack: Vec::new(),
        }
    }
}
