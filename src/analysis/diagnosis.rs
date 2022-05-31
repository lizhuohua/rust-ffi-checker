use std::fmt;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum BugType {
    UseAfterFree,
    DoubleFree,
    MemoryLeakage,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct BugInfo {
    ffi_known: bool,
    possible_bugs: Vec<BugType>,
    msg: Option<String>,
}

impl BugInfo {
    pub fn new(ffi_known: bool, possible_bugs: Vec<BugType>, msg: Option<String>) -> Self {
        Self {
            ffi_known,
            possible_bugs,
            msg,
        }
    }
}

impl fmt::Debug for BugInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BugType::*;
        let mut msg = if self.ffi_known {
            String::from("LLVM IR of C code is known. Possible bugs: ")
        } else {
            String::from("LLVM IR of C code is unknown. Possible bugs: ")
        };

        for bug_type in &self.possible_bugs {
            let bug_name = match bug_type {
                UseAfterFree => "Use After Free",
                DoubleFree => "Double Free",
                MemoryLeakage => "Memory Leakage",
            };
            msg.push_str(bug_name);
            msg.push_str(", ");
        }

        if let Some(m) = &self.msg {
            msg.push_str(m);
        }
        write!(f, "{}", msg)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd)]
pub enum Seriousness {
    Low,
    Medium,
    High,
}

impl fmt::Debug for BugType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BugType::*;
        let name = match self {
            UseAfterFree => "Use After Free",
            DoubleFree => "Double Free",
            MemoryLeakage => "Memory Leakage",
        };
        write!(f, "{}", name)
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Diagnosis {
    pub seriousness: Seriousness,
    pub bug_info: BugInfo,
    pub function_name: String,
    // pub call_stack: Vec<String>,
}

impl fmt::Debug for Diagnosis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            // "Bug type: {:?}, seriousness: {:?}, call stack: {:?}",
            "Bug info: {:?}, seriousness: {:?}, function: {}",
            self.bug_info, self.seriousness, self.function_name
        )
    }
}
