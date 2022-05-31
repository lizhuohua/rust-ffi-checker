#![feature(cell_leak)]
#![feature(rustc_private)]
#![feature(box_patterns)]

extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

pub mod utils;

pub mod entry_collection {
    pub mod callback;
}

pub mod analysis {
    pub mod abstract_domain;
    pub mod block_visitor;
    pub mod context;
    pub mod diagnosis;
    pub mod known_names;
    pub mod option;
    pub mod summary;
    pub mod taint_analysis;
}
