use crate::utils;

/// Categorize different types of functions that should be handled differently
pub enum KnownNameType {
    /// Memory allocation sources
    AllocSource,
    /// Borrowing ownership
    FreeSink,
    /// FFI sinks
    FFISink,
    /// Functions that should be ignored. E.g., `llvm.dbg.declare`
    Ignore,
    /// LLVM intrinsic or Rust standard library functions that should be handled manually. E.g., `llvm.memcpy`
    Intrinsic(Intrinsic),
    /// Functions that should be analyzed normally
    Normal,
}

/// LLVM intrinsic functions and Rust standard library functions
// TODO: Do we need to enumerate them? Or simply define for example "FirstArgToRet"?
pub enum Intrinsic {
    Memcpy,
    IntoVec,
    Deref,
    RcNew,
    Unwrap,
    CStringIntoRaw,
    CStringAsCStr,
    Forget,
    VecIntoRawParts,
    VecAsPtr,
    VecFromRawParts,
    VecPush,
    BoxIntoRaw,
}

/// Some hard-coded function names that are used to distinguish different `KnownNameType`
pub struct KnownNames {
    alloc_sources: Vec<&'static str>,
    free_sinks: Vec<&'static str>,
    should_ignore: Vec<&'static str>,
}

impl Default for KnownNames {
    fn default() -> Self {
        let alloc_sources = vec![
            "alloc::alloc::exchange_malloc",
            "__rust_alloc",
            "__rust_realloc",
            "__rust_alloc_zeroed",
            "<alloc::alloc::Global as core::alloc::AllocRef>::alloc",
        ];

        let free_sinks = vec!["free", "free_rbox_raw"];

        let should_ignore = vec!["llvm.dbg", "__rust_dealloc", "lang_start_internal"];

        Self {
            alloc_sources,
            free_sinks,
            should_ignore,
            // intrinsics,
        }
    }
}

impl KnownNames {
    pub fn get_type(&self, func_name: &String) -> KnownNameType {
        if self.is_alloc_source(func_name) {
            KnownNameType::AllocSource
        } else if self.is_free_sink(func_name) {
            KnownNameType::FreeSink
        } else if self.should_ignore(func_name) {
            KnownNameType::Ignore
        } else if let Some(intrinsic) = self.get_intrinsic(func_name) {
            KnownNameType::Intrinsic(intrinsic)
        } else {
            KnownNameType::Normal
        }
    }

    /// Return whether `func_name` is a source of memory allocation
    pub fn is_alloc_source(&self, func_name: &String) -> bool {
        // `func_name` is mangled so we demangle it here
        let demangled_name = utils::demangle_name(func_name);
        self.alloc_sources.contains(&demangled_name.as_str())
    }

    /// Return whether `func_name` is a sink of memory deallocation
    pub fn is_free_sink(&self, func_name: &String) -> bool {
        // `func_name` is mangled so we demangle it here
        let demangled_name = utils::demangle_name(func_name);
        self.free_sinks.contains(&demangled_name.as_str())
    }

    /// Determines whether `func_name` should be skipped during the analysis
    /// Note that the logic is different from the above two functions.
    pub fn should_ignore(&self, func_name: &String) -> bool {
        for ignore in &self.should_ignore {
            if func_name.contains(ignore) {
                return true;
            }
        }
        return false;
    }

    /// If `func_name` is a LLVM intrinsic, return the `LLVMIntrinsic` variant. Otherwise return `None`
    pub fn get_intrinsic(&self, func_name: &String) -> Option<Intrinsic> {
        if func_name.starts_with("llvm.memcpy") {
            Some(Intrinsic::Memcpy)
        } else if func_name.ends_with("into_vec") {
            Some(Intrinsic::IntoVec)
        } else if func_name.ends_with("::deref_mut") || func_name.ends_with("::deref") {
            Some(Intrinsic::Deref)
        } else if func_name == "alloc::rc::Rc<T>::new" {
            Some(Intrinsic::RcNew)
        } else if func_name == "core::result::Result<T,E>::unwrap" {
            Some(Intrinsic::Unwrap)
        } else if func_name == "std::ffi::c_str::CString::into_raw" {
            Some(Intrinsic::CStringIntoRaw)
        } else if func_name == "std::ffi::c_str::CString::as_c_str"
            || func_name == "std::ffi::c_str::CString::as_bytes"
            || func_name == "std::ffi::c_str::CString::as_bytes_with_nul"
        {
            Some(Intrinsic::CStringAsCStr)
        } else if func_name == "alloc::boxed::Box<T,A>::into_raw" {
            Some(Intrinsic::BoxIntoRaw)
        } else if func_name == "core::mem::forget" {
            Some(Intrinsic::Forget)
        } else if func_name == "alloc::vec::Vec<T,A>::into_raw_parts"
            || func_name == "alloc::vec::Vec<T,A>::into_raw_parts_with_alloc"
        {
            Some(Intrinsic::VecIntoRawParts)
        } else if func_name == "alloc::vec::Vec<T,A>::as_mut_ptr"
            || func_name == "alloc::vec::Vec<T,A>::as_ptr"
            || func_name == "alloc::vec::Vec<T,A>::as_mut_slice"
            || func_name == "alloc::vec::Vec<T,A>::as_slice"
        {
            Some(Intrinsic::VecAsPtr)
        } else if func_name.ends_with("from_raw_parts") || func_name.ends_with("from_raw_parts_mut")
        {
            Some(Intrinsic::VecFromRawParts)
        } else if func_name == "alloc::vec::Vec<T,A>::push" {
            Some(Intrinsic::VecPush)
        } else {
            None
        }
    }
}
