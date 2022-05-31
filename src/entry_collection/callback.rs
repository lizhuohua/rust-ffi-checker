use crate::utils;
use log::debug;
use rustc_driver::Compilation;
use rustc_interface::interface;
use rustc_interface::Queries;
use rustc_middle::ty::TyCtxt;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub struct EntryCollectorCallbacks {
    // If we are compiling a dependency crate, only collect FFI functions
    // If we are compiling a top crate, collect both FFI functions and public functions
    is_dependency: bool,
}

impl EntryCollectorCallbacks {
    pub fn new() -> Self {
        if std::env::var_os("FFI_CHECKER_IS_DEPS").is_some() {
            Self {
                is_dependency: true,
            }
        } else {
            Self {
                is_dependency: false,
            }
        }
    }
}

impl rustc_driver::Callbacks for EntryCollectorCallbacks {
    /// Called after analysis. Return value instructs the compiler whether to
    /// continue the compilation afterwards (defaults to `Compilation::Continue`)
    fn after_analysis<'compiler, 'tcx>(
        &mut self,
        compiler: &'compiler interface::Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        queries
            .global_ctxt()
            .unwrap()
            .peek_mut()
            .enter(|tcx| self.run_analysis(compiler, tcx));
        Compilation::Continue
    }
}

impl EntryCollectorCallbacks {
    fn run_analysis<'tcx, 'compiler>(
        &mut self,
        _compiler: &'compiler interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) {
        // Skip some crates that we are not interested in
        let crate_name =
            utils::get_arg_flag_value("--crate-name").expect("Argument --crate-name not found");
        let should_skip = vec!["build_script_build"];
        if should_skip.contains(&crate_name.as_str()) {
            return;
        }

        // Public functions and FFI functions are globally visible, so their names should be unique
        let mut pub_funcs = HashSet::new();
        let mut ffi_funcs = HashSet::new();

        // If the crate is a binary, add the entry function
        if let Some((entry_def_id, _)) = tcx.entry_fn(()) {
            let item_name = tcx.item_name(entry_def_id).to_ident_string();
            pub_funcs.insert(item_name);
        }

        // Initialize global analysis context
        let hir = tcx.hir();
        for item in hir.items() {
            // If it is a top crate, collect all the public functions/methods
            if !self.is_dependency {
                match &item.kind {
                    rustc_hir::ItemKind::Fn { .. } => {
                        if item.vis.node.is_pub() {
                            debug!("Public Fn: {:?}, {:?}", item.def_id, item.ident);
                            pub_funcs.insert(String::from(&*(item.ident.as_str())));
                        }
                    }
                    rustc_hir::ItemKind::Impl(impl_inner) => {
                        for item_ref in impl_inner.items {
                            if matches!(item_ref.kind, rustc_hir::AssocItemKind::Fn { .. }) {
                                // The visibility of an `Impl` is stored in `ImplItem`, so we get it through its id
                                let impl_item_id = item_ref.id;
                                let impl_item = hir.impl_item(impl_item_id);
                                if impl_item.vis.node.is_pub() {
                                    let defpath = tcx.def_path(item_ref.id.def_id.to_def_id());
                                    // for (parent_hir_id, _) in hir.parent_owner_iter(item.hir_id()) {
                                    //     let parent_item =
                                    //         hir.expect_item(hir.local_def_id(parent_hir_id));
                                    //     debug!("Parent: {:?}", parent_item.ident);
                                    // }
                                    debug!(
                                        "Public Impl Fn: {:?}, {:?}, {}",
                                        item_ref.id,
                                        item_ref.ident,
                                        defpath.to_filename_friendly_no_crate()
                                    );
                                    pub_funcs.insert(String::from(&*(item_ref.ident.as_str())));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let rustc_hir::ItemKind::ForeignMod { abi: _, items } = item.kind {
                for itemref in items {
                    debug!("FFI: {:?}, {:?}", itemref.id, itemref.ident);
                    ffi_funcs.insert(String::from(&*(itemref.ident.as_str())));

                    // The visibility of a foreign function is stored in `ForeignItem`, so we get it through its id
                    let foreign_item_id = itemref.id;
                    let foreign_item = hir.foreign_item(foreign_item_id);
                    if foreign_item.vis.node.is_pub() {
                        debug!("Public FFI Fn: {:?}, {:?}", itemref.id, itemref.ident);
                        pub_funcs.insert(String::from(&*(itemref.ident.as_str())));
                    }
                }
            }
        }

        // If we collect some entry points and FFI functions, write them to files
        // Note that to get more results, we only consider whether entry points are found,
        // even if there is no FFI called, we still continue the analysis
        if !pub_funcs.is_empty() {
            // Create directory `entry_points` if not exists
            if !Path::new("target/entry_points").exists() {
                std::fs::create_dir_all("target/entry_points")
                    .expect("Failed to create `entry_points` directory");
            }

            let file_path = Path::new("target/entry_points").join(crate_name);

            if !file_path.exists() {
                let mut file = File::create(file_path).expect("Failed to create file");
                for entry in pub_funcs {
                    file.write_all(format!("Entry: {}\n", entry).as_bytes())
                        .unwrap();
                }
                for ffi in ffi_funcs {
                    file.write_all(format!("FFI: {}\n", ffi).as_bytes())
                        .unwrap();
                }
            }
        }
    }
}
