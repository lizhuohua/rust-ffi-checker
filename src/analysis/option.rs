// Parsing command line options

use crate::analysis::diagnosis::Seriousness;
use log::{info, warn};
use std::env::Args;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct AnalysisOption {
    pub crate_names: Vec<String>,
    pub entry_points: Vec<String>,
    pub ffi_functions: Vec<String>,
    pub bitcode_file_paths: Vec<String>,
    pub precision_threshold: Seriousness,
}

impl Default for AnalysisOption {
    // By default, get entry points from `target/entry_points/crate_name`
    // and get bitcode paths from `target/bitcode_paths`
    fn default() -> Self {
        let mut crate_names = vec![];
        let mut entry_points = vec![];
        let mut ffi_functions = vec![];
        let mut bitcode_file_paths = vec![];

        let entry_points_path = Path::new("target/entry_points");

        // Get all the crate names
        for dir_entry in WalkDir::new(entry_points_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            // Ignore the root itself, which has depth 0
            if dir_entry.depth() == 1 {
                crate_names.push(dir_entry.file_name().to_string_lossy().into_owned());
            }
        }

        for dir_entry in WalkDir::new(entry_points_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if dir_entry.file_type().is_file() {
                let file = File::open(dir_entry.path()).unwrap();

                for line in io::BufReader::new(file).lines() {
                    if let Ok(line_str) = line {
                        if line_str.starts_with("Entry: ") {
                            // Skip "Entry: "
                            entry_points.push(line_str.chars().skip(7).collect())
                        } else if line_str.starts_with("FFI: ") {
                            // Skip "FFI: "
                            ffi_functions.push(line_str.chars().skip(5).collect())
                        }
                    }
                }
            }
        }

        let file = File::open(Path::new("target/bitcode_paths")).unwrap();
        for line in io::BufReader::new(file).lines() {
            if let Ok(line_str) = line {
                bitcode_file_paths.push(line_str);
            }
        }

        info!("Crate names: {:?}", crate_names);
        info!("Entry points: {:?}", entry_points);
        info!("FFI functions: {:?}", ffi_functions);
        info!("Bitcode paths: {:?}", bitcode_file_paths);

        Self {
            crate_names,
            entry_points,
            ffi_functions,
            bitcode_file_paths,
            precision_threshold: Seriousness::Low,
        }
    }
}

impl AnalysisOption {
    pub fn from_args(args: Args) -> Self {
        let args = args.enumerate().map(|(_i, arg)| arg).collect::<Vec<_>>();
        let mut res = Self::default();
        for (i, arg) in args.iter().enumerate() {
            if arg.starts_with("--") {
                match &arg[2..] {
                    "entry" => {
                        res.entry_points.push(args[i + 1].clone());
                    }
                    "bitcode" => {
                        res.bitcode_file_paths.push(args[i + 1].clone());
                    }
                    "precision_filter" => {
                        let threshold = match &*args[i + 1] {
                            "high" => Seriousness::High,
                            "mid" => Seriousness::Medium,
                            "low" => Seriousness::Low,
                            _ => {
                                warn!(
                                    "Unrecognized precision filter threshold, using default: Low"
                                );
                                Seriousness::Low
                            }
                        };
                        res.precision_threshold = threshold;
                    }
                    _ => {}
                }
            }
        }
        res
    }
}
