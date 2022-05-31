use crate::analysis::abstract_domain::{BlockState, MemoryState};
use crate::analysis::diagnosis::Seriousness;
use crate::analysis::diagnosis::{BugInfo, BugType, Diagnosis};
use crate::analysis::known_names::{Intrinsic, KnownNameType};
use crate::analysis::summary::Summary;
use crate::analysis::taint_analysis::FuncAnalysis;
use crate::utils;
use either::Either;
use llvm_ir::constant::Constant;
use llvm_ir::function::Function;
use llvm_ir::function::ParameterAttribute;
use llvm_ir::instruction::{
    AddrSpaceCast, Alloca, BitCast, Call, ExtractElement, ExtractValue, GetElementPtr,
    InsertElement, InsertValue, IntToPtr, Load, Phi, PtrToInt, SExt, ShuffleVector, Store, Trunc,
    ZExt,
};
use llvm_ir::terminator::{CallBr, Invoke, Ret};
use llvm_ir::types::Type;
use llvm_ir::{BasicBlock, Instruction, Name, Operand, Terminator};
use log::debug;

pub struct BlockVisitor<'a> {
    // The reference to upper layer `FuncAnlaysis`, used to get information about the anlaysis
    // E.g., the current depth of function call
    func_analysis: &'a mut FuncAnalysis,
    // The current abstract state of the basic block
    state: BlockState,
    // The basic block to be analyzed
    bb: BasicBlock,
}

impl<'a> BlockVisitor<'a> {
    /// Initialize a `BlockVisitor`. `pre` specifies the state immediately before analyzing `bb`
    pub fn new(func_analysis: &'a mut FuncAnalysis, pre: &BlockState, bb: BasicBlock) -> Self {
        Self {
            func_analysis,
            state: pre.clone(),
            bb,
        }
    }

    /// Start the analysis for the `BlockVisitor`, and return a `BlockState`
    pub fn analyze(&mut self) -> BlockState {
        let instrs = self.bb.instrs.clone(); // Clone to avoid multiple references for `self`

        // Analyze all the instructions in the basic block
        for instr in &instrs {
            debug!("Analyzing instruction: {:?}", instr);
            self.analyze_instruction(instr);
        }
        // Analyze the terminator
        debug!("Analyzing terminator: {:?}", self.bb.term);
        self.analyze_terminator(&self.bb.term.clone());

        self.state.clone()
    }

    /// Dispatch each instruction to its corresponding transfer function
    fn analyze_instruction(&mut self, instr: &Instruction) {
        use llvm_ir::instruction;
        use Instruction::*;
        match instr {
            Load(load) => self.analyze_load(load),
            Store(store) => self.analyze_store(store),
            Call(call) => self.analyze_call(call),
            BitCast(bitcast) => self.analyze_bitcast(bitcast),
            InsertElement(insertelement) => self.analyze_insertelement(insertelement),
            ExtractElement(insertelement) => self.analyze_extractelement(insertelement),
            ShuffleVector(shufflevector) => self.analyze_shufflevector(shufflevector),
            ExtractValue(extractvalue) => self.analyze_extractvalue(extractvalue),
            InsertValue(insertvalue) => self.analyze_insertvalue(insertvalue),
            GetElementPtr(getelementptr) => self.analyze_getelementptr(getelementptr),
            Trunc(trunc) => self.analyze_trunc(trunc),
            ZExt(zext) => self.analyze_zext(zext),
            SExt(sext) => self.analyze_sext(sext),
            PtrToInt(ptrtoint) => self.analyze_ptrtoint(ptrtoint),
            IntToPtr(inttoptr) => self.analyze_inttoptr(inttoptr),
            AddrSpaceCast(addrspacecast) => self.analyze_addrspacecast(addrspacecast),
            Phi(phi) => self.analyze_phi(phi),
            Alloca(alloca) => self.analyze_alloca(alloca),
            Add(instruction::Add {
                operand0,
                operand1,
                dest,
                ..
            })
            | Sub(instruction::Sub {
                operand0,
                operand1,
                dest,
                ..
            })
            | Mul(instruction::Mul {
                operand0,
                operand1,
                dest,
                ..
            })
            | UDiv(instruction::UDiv {
                operand0,
                operand1,
                dest,
                ..
            })
            | SDiv(instruction::SDiv {
                operand0,
                operand1,
                dest,
                ..
            })
            | URem(instruction::URem {
                operand0,
                operand1,
                dest,
                ..
            })
            | SRem(instruction::SRem {
                operand0,
                operand1,
                dest,
                ..
            })
            | And(instruction::And {
                operand0,
                operand1,
                dest,
                ..
            })
            | Or(instruction::Or {
                operand0,
                operand1,
                dest,
                ..
            })
            | Xor(instruction::Xor {
                operand0,
                operand1,
                dest,
                ..
            })
            | Shl(instruction::Shl {
                operand0,
                operand1,
                dest,
                ..
            })
            | LShr(instruction::LShr {
                operand0,
                operand1,
                dest,
                ..
            })
            | AShr(instruction::AShr {
                operand0,
                operand1,
                dest,
                ..
            }) => self.analyze_arithmetic(operand0, operand1, dest),
            _ => (),
        }
    }

    /// Dispatch each terminator to its corresponding transfer function
    fn analyze_terminator(&mut self, term: &Terminator) {
        use Terminator::*;
        match term {
            CallBr(callbr) => self.analyze_callbr(callbr),
            Invoke(invoke) => self.analyze_invoke(invoke),
            Ret(ret) => self.analyze_ret(ret),
            _ => (),
        }
    }

    /// Handle arithmetic operations such as `Add`, `Sub`, etc.
    fn analyze_arithmetic(&mut self, _operand0: &Operand, _operand1: &Operand, _dest: &Name) {
        // TODO: Intentionally left empty for the time being... Will decide whether we need to implement this later
        // match (operand0, operand1) {
        //     (Operand::LocalOperand { name: op0, .. }, Operand::LocalOperand { name: op1, .. }) => {
        //         if self.state.is_tainted(op0) || self.state.is_tainted(op1) {
        //             self.state.set_tainted(dest, true);
        //         } else {
        //             self.state.set_tainted(dest, false);
        //         }
        //     }
        //     _ => (),
        // }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#load-instruction
    fn analyze_load(&mut self, load: &Load) {
        // dest <- address
        let address = &load.address;
        let dest = &load.dest;
        match address {
            Operand::LocalOperand { name, .. } => {
                self.state.propagate_taint(name, dest);
            }
            _ => (),
        }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#store-instruction
    fn analyze_store(&mut self, store: &Store) {
        // address <- value
        let address = &store.address;
        let value = &store.value;
        match (address, value) {
            (
                Operand::LocalOperand {
                    name: address_name, ..
                },
                Operand::LocalOperand {
                    name: value_name, ..
                },
            ) => {
                self.state.propagate_taint(value_name, address_name);
            }
            _ => (),
        }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#i-bitcast
    fn analyze_bitcast(&mut self, bitcast: &BitCast) {
        let operand = &bitcast.operand;
        let dest = &bitcast.dest;
        match operand {
            Operand::LocalOperand { name, .. } => {
                self.state.propagate_taint(name, dest);
            }
            _ => (),
        }
    }

    /// Get the function name from `llvm_ir::instruction::Call`
    /// This is just a wrapper function that avoid boilerplate
    fn get_func_name_from_call(call: &Call) -> Option<String> {
        if let Either::Right(operand) = &call.function {
            if let Operand::ConstantOperand(constant_ref) = operand {
                if let Constant::GlobalReference { name, .. } = constant_ref.as_ref() {
                    if let Name::Name(box func_name) = name {
                        return Some(func_name.clone());
                    }
                    return None;
                }
                return None;
            }
            return None;
        }
        return None;
    }

    /// Get the function name from `llvm_ir::terminator::Invoke`
    /// This is just a wrapper function that avoid boilerplate
    fn get_func_name_from_invoke(invoke: &Invoke) -> Option<String> {
        if let Either::Right(operand) = &invoke.function {
            if let Operand::ConstantOperand(constant_ref) = operand {
                if let Constant::GlobalReference { name, .. } = constant_ref.as_ref() {
                    if let Name::Name(box func_name) = name {
                        return Some(func_name.clone());
                    }
                    return None;
                }
                return None;
            }
            return None;
        }
        return None;
    }

    /// Determine whether a function is an FFI function
    /// All the FFI functions' names are collected from Rust HIR
    /// This is just a wrapper function that avoid boilerplate
    fn is_ffi_function(&self, func_name: &String) -> bool {
        self.func_analysis
            .context
            .borrow()
            .analysis_option
            .ffi_functions
            .contains(func_name)
    }

    /// Get the type of a function
    /// This is just a wrapper function that avoid boilerplate
    // TODO: should we integrate `is_ffi_function` into here?
    fn get_function_type(&self, func_name: &String) -> KnownNameType {
        if self.is_ffi_function(func_name) {
            KnownNameType::FFISink
        } else {
            self.func_analysis
                .context
                .borrow()
                .known_names
                .get_type(&utils::demangle_name(func_name))
        }
    }

    /// Get the LLVM IR of a function. If it is not found, return `None`
    /// This is just a wrapper function that avoid boilerplate
    fn get_llvm_ir(&self, func_name: &String) -> Option<llvm_ir::Function> {
        self.func_analysis
            .context
            .borrow()
            .functions
            .get(func_name)
            .cloned()
    }

    /// Perform static analysis on a normal function
    /// This will be used after we have distinguished the function type, specifically, we analyze a function
    /// if it is of type `KnownNameType::Normal`, or it is of type `KnownNameType::FFISink` and its LLVM IR is known.
    fn analyze_normal_function(
        &mut self,
        func_name: &String,
        arguments: &Vec<(Operand, Vec<ParameterAttribute>)>, // caller's arguments
        dest: &Option<Name>,
    ) {
        debug!(
            "visit function: {}, arguments: {:?}, dest: {:?}",
            utils::demangle_name(&func_name),
            arguments,
            dest
        );
        debug!("Current state: {:?}", self.state);

        // Get LLVM IR of the function
        let function = match self.get_llvm_ir(func_name) {
            Some(function) => function,
            // If the LLVM IR cannot be found through function name, just return
            None => return,
        };

        // Push the function name into call stack
        // self.func_analysis
        //     .context
        //     .borrow_mut()
        //     .call_stack
        //     .push(utils::demangle_name(&func_name));

        // Construct a `Vec<Option<bool>>` as part of the key to get function summary
        let init = arguments
            .iter()
            .map(|(operand, _)| match operand {
                Operand::LocalOperand { name, .. } => Some(self.state.get_memory_state(name)),
                _ => None,
            })
            .collect::<Vec<_>>();

        // Handle side effects of the function call
        if let Some((end_state, ret_state)) = self.get_function_summary(&function, init) {
            // First, set the state of the return value
            if let Some(name) = dest {
                self.state.set_tainted(&name, ret_state);
            }

            // Second, propagate the state of parameters
            let callee_params = function.parameters.iter().map(|param| param.name.clone());

            // `arg_map` has type `Vec<(Option<&Name>, Name)>`, which maps caller's arguments to callee's
            let arg_map = arguments
                .iter()
                .map(|(operand, _)| match operand {
                    Operand::LocalOperand { name, .. } => Some(name),
                    _ => None,
                })
                .zip(callee_params)
                .collect::<Vec<_>>();

            // Update variables that are passed to the function
            for (caller_arg, callee_arg) in arg_map {
                if let Some(caller_arg_name) = caller_arg {
                    self.state
                        .set_tainted(caller_arg_name, end_state.get_memory_state(&callee_arg));
                }
            }
        }
    }

    /// Distinguish the type of a function and handle it according to its type
    fn analyze_function(
        &mut self,
        func_name: &String,
        arguments: &Vec<(Operand, Vec<ParameterAttribute>)>, // caller's arguments
        dest: &Option<Name>,
    ) {
        match self.get_function_type(func_name) {
            // If the function is an FFI function, and the LLVM IR of it is unknown
            // E.g., it is linked from shared libraries. We cannot do further analysis so we generate diagnosis directly
            // If the LLVM IR is known, we continue to analyze it
            KnownNameType::FFISink => {
                match self.get_llvm_ir(func_name) {
                    Some(_) => {
                        self.analyze_normal_function(func_name, arguments, dest);
                        // After the analysis finishes, check whether any arguments are still in state "forgotten",
                        // if yes, meaning that there is a potential memory leak
                        for (operand, _) in arguments {
                            if let Operand::LocalOperand { name, .. } = operand {
                                match self.state.get_memory_state(name) {
                                    MemoryState::Unknown => self.generate_diagnosis(
                                        BugInfo::new(
                                            true,
                                            vec![BugType::MemoryLeakage],
                                            Some(format!("After FFI {} finishes, argument {} is still in state `Unknown`", func_name, name)),
                                        ),
                                        Seriousness::Medium,
                                    ),
                                    MemoryState::Forgotten => self.generate_diagnosis(
                                        BugInfo::new(
                                            true,
                                            vec![BugType::MemoryLeakage],
                                            Some(format!("After FFI {} finishes, argument {} is still in state `Forgotten`", func_name, name)),
                                        ),
                                        Seriousness::Medium,
                                    ),
                                    // MemoryState::Tainted => {
                                    //     self.generate_diagnosis(
                                    //         BugInfo::new(
                                    //             true,
                                    //             vec![BugType::MemoryLeakage],
                                    //             Some(format!("After FFI {} finishes, argument {} is still in state `Tainted`", func_name, name)),
                                    //         ),
                                    //         Seriousness::Low,
                                    //     );
                                    // },
                                    // MemoryState::Borrowed => {
                                    //     self.generate_diagnosis(
                                    //         BugInfo::new(
                                    //             true,
                                    //             vec![BugType::MemoryLeakage],
                                    //             Some(format!("After FFI {} finishes, argument {} is still in state `Borrowed`", func_name, name)),
                                    //         ),
                                    //         Seriousness::Low,
                                    //     );
                                    // },
                                    _ => {
                                        // Do nothing
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        // Check whether any argument is tainted, and generate diagnosis with different seriousness
                        for (operand, _) in arguments {
                            if let Operand::LocalOperand { name, .. } = operand {
                                match self.state.get_memory_state(name) {
                                    MemoryState::Unknown => self.generate_diagnosis(
                                        BugInfo::new(
                                            false,
                                            vec![BugType::MemoryLeakage],
                                            Some(format!(
                                                "FFI {} is unknown, argument is in state `Unknown`.", func_name
                                            )),
                                        ),
                                        Seriousness::Medium,
                                    ),
                                    MemoryState::Borrowed => self.generate_diagnosis(
                                        BugInfo::new(
                                            false,
                                            vec![BugType::UseAfterFree],
                                            Some(format!(
                                                "FFI {} is unknown, argument is in state `Borrowed`.", func_name
                                            )),
                                        ),
                                        Seriousness::Low,
                                    ),
                                    MemoryState::Forgotten => self.generate_diagnosis(
                                        BugInfo::new(
                                            false,
                                            vec![BugType::MemoryLeakage],
                                            Some(format!(
                                                "FFI {} is unknown, argument is in state `Forgotten`.", func_name
                                            )),
                                        ),
                                        Seriousness::Medium,
                                    ),
                                    MemoryState::Tainted => self.generate_diagnosis(
                                        BugInfo::new(
                                            false,
                                            vec![BugType::MemoryLeakage],
                                            Some(format!(
                                                "FFI {} is unknown, argument is in state `Tainted`.", func_name
                                            )),
                                        ),
                                        Seriousness::Low,
                                    ),
                                    _ => {
                                        // Do nothing
                                    }
                                }
                            }
                        }
                    }
                }
            }
            KnownNameType::AllocSource => {
                debug!(
                    "Find allocation source: {}, arguments: {:?}, dest: {:?}",
                    utils::demangle_name(&func_name),
                    arguments,
                    dest
                );
                if let Some(name) = dest {
                    self.state.set_tainted(&name, MemoryState::Tainted);
                }
            }
            KnownNameType::FreeSink => {
                debug!(
                    "Find deallocation sink: {}, arguments: {:?}, dest: {:?}",
                    func_name, arguments, dest
                );
                // Check whether any argument is tainted
                if arguments.iter().any(|(operand, _)| match operand {
                    Operand::LocalOperand { name, .. } => self.state.is_tainted(&name),
                    _ => false,
                }) {
                    self.generate_diagnosis(
                        BugInfo::new(
                            true,
                            vec![BugType::UseAfterFree, BugType::DoubleFree],
                            Some(String::from("Taint source meets taint sink.")),
                        ),
                        Seriousness::High,
                    );
                }
            }
            KnownNameType::Intrinsic(intrinsic) => {
                debug!(
                    "Find LLVM intrinsic: {}, arguments: {:?}, dest: {:?}",
                    func_name, arguments, dest
                );
                // Handle it manually
                self.handle_intrinsic(intrinsic, arguments, dest);
            }
            KnownNameType::Normal => {
                self.analyze_normal_function(func_name, arguments, dest);
            }
            KnownNameType::Ignore => {
                debug!(
                    "Skip should-be-ignored function: {}",
                    utils::demangle_name(&func_name)
                );
            }
        }
    }

    fn handle_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: &Vec<(Operand, Vec<ParameterAttribute>)>, // caller's arguments
        dest: &Option<Name>,
    ) {
        match intrinsic {
            Intrinsic::Memcpy => {
                // Syntax of `llvm.memcpy` intrinsic:
                // declare void @llvm.memcpy.p0i8.p0i8.i64(i8* <dest>, i8* <src>, i64 <len>, i1 <isvolatile>)
                assert!(arguments.len() == 4);
                assert!(*dest == None);

                // Extract the 1st and 2nd arguments, and propagate taint
                if let (
                    Operand::LocalOperand { name: dest, .. },
                    Operand::LocalOperand { name: src, .. },
                ) = (&arguments[0].0, &arguments[1].0)
                {
                    self.state.propagate_taint(src, dest);
                }
            }
            Intrinsic::IntoVec => {
                // E.g., alloc::slice::<impl [T]>::into_vec
                // These functions usually pass the second argument to the first, and return void
                assert!(arguments.len() >= 2);
                // FIXME: seems like the return value can be non-void, comment this line for the time being
                // assert!(*dest == None);
                // Extract the 1st and 2nd arguments, and propagate taint
                if let (
                    Operand::LocalOperand { name: des, .. },
                    Operand::LocalOperand { name: src, .. },
                ) = (&arguments[0].0, &arguments[1].0)
                {
                    self.state.propagate_taint(src, des);
                }
            }
            Intrinsic::Deref => {
                // This includes both `deref` and `deref_mut`.
                // It passes the first argument to the return value
                // %1 = invoke { [0 x i8]*, i64 } @"_ZN68_$LT$alloc..string..String$u20$as$u20$core..ops..deref..DerefMut$GT$9deref_mut17ha158855bd2b05e71E"(%"alloc::string::String"* align 8 dereferenceable(24) %s)
                // FIXME: seems like # of arguments can be > 1, comment this line for the time being
                // assert!(arguments.len() == 1);
                assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    self.state.propagate_taint(name, dest.as_ref().unwrap());
                }
            }
            Intrinsic::RcNew => {
                // E.g., %14 = call nonnull i64* @"_ZN5alloc2rc11Rc$LT$T$GT$3new17h18438c0fa384bab7E"(i32* noalias nonnull align 4 %13), !dbg !3037
                // It passes the first argument to the return value
                assert!(arguments.len() == 1);
                assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    self.state.propagate_taint(name, dest.as_ref().unwrap());
                }
            }
            Intrinsic::Unwrap => {
                // E.g., %0 = call { i8*, i64 } @"_ZN4core6result19Result$LT$T$C$E$GT$6unwrap17h1f9a0e613e9c3360E"(%"core::result::Result<std::ffi::c_str::CString, std::ffi::c_str::NulError>"* noalias nocapture dereferenceable(40) %_2, %"core::panic::location::Location"* align 8 dereferenceable(24) bitcast (<{ i8*, [16 x i8] }>* @alloc37 to %"core::panic::location::Location"*)), !dbg !984
                // It passes the first argument to the return value
                // assert!(arguments.len() == 2);

                // Seems like the return value can be None, so we check it here
                // assert!(*dest != None);
                if *dest != None {
                    if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                        self.state.propagate_taint(name, dest.as_ref().unwrap());
                    }
                }
            }
            Intrinsic::CStringIntoRaw => {
                // E.g., %p = call i8* @_ZN3std3ffi5c_str7CString8into_raw17he8bf9ee6170c88bfE(i8* noalias nonnull align 1 %s.0, i64 %s.1), !dbg !986
                // It forgets the first argument and passes the first argument to the return value
                assert!(arguments.len() == 2);
                assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    if self.state.get_memory_state(name) < MemoryState::Forgotten {
                        self.state.set_tainted(name, MemoryState::Forgotten);
                    }
                    self.state.propagate_taint(name, dest.as_ref().unwrap());
                }
            }
            Intrinsic::CStringAsCStr => {
                // Borrow the first argument and pass it to the return value
                assert!(arguments.len() >= 1);
                assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    if self.state.get_memory_state(name) < MemoryState::Borrowed {
                        self.state.set_tainted(name, MemoryState::Borrowed);
                    }
                    self.state.propagate_taint(name, dest.as_ref().unwrap());
                }
            }
            Intrinsic::Forget => {
                // Forget the first argument
                // E.g., invoke void @_ZN4core3mem6forget17h7877bf55d202402bE(%"alloc::vec::Vec<i32>"* noalias nocapture noundef dereferenceable(24) %_21)
                // assert!(arguments.len() == 1);
                // assert!(*dest == None);

                // FIXME: The arguments can be empty, just ignore this case for now...
                if arguments.len() >= 1 {
                    if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                        if self.state.get_memory_state(name) < MemoryState::Forgotten {
                            self.state.set_tainted(name, MemoryState::Forgotten);
                        }
                        // The return value can be None or not None
                        // If it is not None, we propagate the state to the destination
                        if let Some(dest_name) = dest {
                            self.state.propagate_taint(name, dest_name);
                        }
                    }
                }
            }
            Intrinsic::BoxIntoRaw => {
                // Forget the first argument and pass the first argument to the return value
                // E.g., %_4 = call { i32, i32 }* @"_ZN5alloc5boxed16Box$LT$T$C$A$GT$8into_raw17hfcf7bd0c971663edE"({ i32, i32 }* noalias nonnull align 4 %20), !dbg !991
                // assert!(arguments.len() == 1);
                assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    if self.state.get_memory_state(name) < MemoryState::Forgotten {
                        self.state.set_tainted(name, MemoryState::Forgotten);
                    }
                    self.state.propagate_taint(name, dest.as_ref().unwrap());
                }
            }
            Intrinsic::VecIntoRawParts => {
                // Forget the second argument
                // E.g., invoke void @"_ZN5alloc3vec16Vec$LT$T$C$A$GT$14into_raw_parts17h3189686ebda467e7E"({ i32*, i64, i64 }* sret({ i32*, i64, i64 }) %b, %"alloc::vec::Vec<i32>"* %_21)
                assert!(arguments.len() == 2);
                assert!(*dest == None);
                if let Operand::LocalOperand { name, .. } = &arguments[1].0 {
                    if self.state.get_memory_state(name) < MemoryState::Forgotten {
                        self.state.set_tainted(name, MemoryState::Forgotten);
                    }
                }
            }
            Intrinsic::VecAsPtr => {
                // Borrow the first argument, and pass it to the return value
                assert!(arguments.len() >= 1);
                assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    if self.state.get_memory_state(name) < MemoryState::Borrowed {
                        self.state.set_tainted(name, MemoryState::Borrowed);
                    }
                    self.state.propagate_taint(name, dest.as_ref().unwrap());
                }
            }
            Intrinsic::VecFromRawParts => {
                // Pass the first argument to the return value
                // This function converts a forgotten memory back to Rust,
                // so if the first argument is forgotten, change it to `Alloc`
                assert!(arguments.len() >= 1);
                // assert!(*dest != None);
                if let Operand::LocalOperand { name, .. } = &arguments[0].0 {
                    if self.state.get_memory_state(name) == MemoryState::Forgotten {
                        self.state.set_tainted(name, MemoryState::Tainted);
                    }
                    if *dest != None {
                        self.state.propagate_taint(name, dest.as_ref().unwrap());
                    }
                }
            }
            Intrinsic::VecPush => {
                // Pass the second argument to the first argument
                // E.g., invoke void @"_ZN5alloc3vec16Vec$LT$T$C$A$GT$4push17he4ff1507cb00663dE"(%"alloc::vec::Vec<*const f64>"* align 8 dereferenceable(24) %cost, double* %_73)
                assert!(arguments.len() >= 2);
                // assert!(*dest == None);
                if let (
                    Operand::LocalOperand { name: des, .. },
                    Operand::LocalOperand { name: src, .. },
                ) = (&arguments[0].0, &arguments[1].0)
                {
                    self.state.propagate_taint(src, des);
                }
            }
        }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#call-instruction
    fn analyze_call(&mut self, call: &Call) {
        // If the function is called by its name
        if let Some(func_name) = Self::get_func_name_from_call(call) {
            self.analyze_function(&func_name, &call.arguments, &call.dest);
        }
        // Otherwise if the function is called through a function pointer
        // Since it is hard to know where the function pointer points to, we
        // just assume it points to an FFI (i.e., a taint sink)
        else if Self::is_function_pointer_from_call(call) {
            // Check whether any argument is tainted
            for (arg_operand, _) in &call.arguments {
                if let Operand::LocalOperand { name, .. } = arg_operand {
                    match self.state.get_memory_state(&name) {
                        MemoryState::Tainted => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::UseAfterFree],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Tainted`.",
                                    )),
                                ),
                                Seriousness::Low,
                            );
                        }
                        MemoryState::Borrowed => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::UseAfterFree],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Borrowed`.",
                                    )),
                                ),
                                Seriousness::Low,
                            );
                        }
                        MemoryState::Forgotten => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::MemoryLeakage],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Forgotten`.",
                                    )),
                                ),
                                Seriousness::Medium,
                            );
                        }
                        MemoryState::Unknown => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::UseAfterFree, BugType::MemoryLeakage],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Unknown`.",
                                    )),
                                ),
                                Seriousness::Medium,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#invoke-instruction
    fn analyze_invoke(&mut self, invoke: &Invoke) {
        if let Some(func_name) = Self::get_func_name_from_invoke(invoke) {
            self.analyze_function(&func_name, &invoke.arguments, &Some(invoke.result.clone()));
        } else if Self::is_function_pointer_from_invoke(invoke) {
            for (arg_operand, _) in &invoke.arguments {
                if let Operand::LocalOperand { name, .. } = arg_operand {
                    match self.state.get_memory_state(&name) {
                        MemoryState::Tainted => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::UseAfterFree],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Tainted`.",
                                    )),
                                ),
                                Seriousness::Low,
                            );
                        }
                        MemoryState::Borrowed => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::UseAfterFree],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Borrowed`.",
                                    )),
                                ),
                                Seriousness::Low,
                            );
                        }
                        MemoryState::Forgotten => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::MemoryLeakage],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Forgotten`.",
                                    )),
                                ),
                                Seriousness::Medium,
                            );
                        }
                        MemoryState::Unknown => {
                            self.generate_diagnosis(
                                BugInfo::new(
                                    false,
                                    vec![BugType::UseAfterFree, BugType::MemoryLeakage],
                                    Some(String::from(
                                        "Call by function pointer, argument is `Unknown`.",
                                    )),
                                ),
                                Seriousness::Medium,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Determine whether the function in the `Call` instruction is a function pointer
    /// This is just a wrapper function that avoid boilerplate
    fn is_function_pointer_from_call(call: &Call) -> bool {
        if let Either::Right(operand) = &call.function {
            if let Operand::LocalOperand { name: _, ty } = operand {
                if let Type::PointerType { pointee_type, .. } = &**ty {
                    if matches!(**pointee_type, Type::FuncType { .. }) {
                        return true;
                    }
                    return false;
                }
                return false;
            }
            return false;
        }
        return false;
    }

    /// Determine whether the function in the `Invoke` instruction is a function pointer
    /// This is just a wrapper function that avoid boilerplate
    fn is_function_pointer_from_invoke(invoke: &Invoke) -> bool {
        if let Either::Right(operand) = &invoke.function {
            if let Operand::LocalOperand { name: _, ty } = operand {
                if let Type::PointerType { pointee_type, .. } = &**ty {
                    if matches!(**pointee_type, Type::FuncType { .. }) {
                        return true;
                    }
                    return false;
                }
                return false;
            }
            return false;
        }
        return false;
    }

    fn get_function_summary(
        &mut self,
        function: &Function,
        init: Vec<Option<MemoryState>>,
    ) -> Option<Summary> {
        let func_name = function.name.clone();
        let key = &(func_name.clone(), init.clone());
        let summary = self
            .func_analysis
            .context
            .borrow()
            .summary_cache
            .get(key)
            .cloned();
        if let Some(summary) = summary {
            // If the summary is already in cache, return it
            debug!("Summary is already in the cache");
            Some(summary)
        } else {
            // If the summary is not computed yet
            debug!("Summary is not in the cache yet, compute it");

            // Initialize initial state for tainted parameters
            let mut init_state = BlockState::default();
            let callee_params: Vec<_> = function.parameters.iter().cloned().collect();
            // `arg_map` has type `Vec<(&Option<MemoryState>, Parameter)>`
            let arg_map = init.iter().zip(callee_params).collect::<Vec<_>>();

            // Update tainted variables that are passed to the function
            for (caller_arg, callee_arg) in arg_map {
                if let Some(mem_state) = caller_arg {
                    init_state.set_tainted(&callee_arg.name, *mem_state);
                }
            }

            // Compute summary and cache it
            if let Some(mut taint_analysis) = FuncAnalysis::new_with_init(
                self.func_analysis.context.clone(),
                &func_name,
                &init_state,
                self.func_analysis.depth + 1,
            ) {
                taint_analysis.iterate_to_fixpoint();

                let state = taint_analysis.get_state_after_call();
                let ret_state = taint_analysis.ret_state;
                let summary = (state, ret_state);
                self.func_analysis
                    .context
                    .borrow_mut()
                    .summary_cache
                    .insert(key, summary.clone());

                Some(summary)
            } else {
                None
            }
        }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#ret-instruction
    fn analyze_ret(&mut self, ret: &Ret) {
        // self.func_analysis.context.borrow_mut().call_stack.pop();

        if let Some(Operand::LocalOperand { name, .. }) = &ret.return_operand {
            self.func_analysis.ret_state = self.state.get_memory_state(name);
        }
    }

    /// https://releases.llvm.org/12.0.0/docs/LangRef.html#callbr-instruction
    fn analyze_callbr(&mut self, callbr: &CallBr) {
        debug!("visit callbr: {:?}", callbr);
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#insertelement-instruction
    fn analyze_insertelement(&mut self, insertelement: &InsertElement) {
        // dest <- element
        let element = &insertelement.element;
        let dest = &insertelement.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = element
        {
            // Here we don't use `self.state.propagate_taint`, because this instruction only
            // changes one element of an vector. So even if `element` is not tainted, it may
            // not clear the taint state of `dest`
            self.state
                .set_tainted(dest, self.state.get_memory_state(value_name));
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#extractelement-instruction
    fn analyze_extractelement(&mut self, extractelement: &ExtractElement) {
        // dest <- vector
        let vector = &extractelement.vector;
        let dest = &extractelement.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = vector
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#extractvalue-instruction
    fn analyze_extractvalue(&mut self, extractvalue: &ExtractValue) {
        // dest <- aggregate
        let aggregate = &extractvalue.aggregate;
        let dest = &extractvalue.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = aggregate
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#insertvalue-instruction
    fn analyze_insertvalue(&mut self, insertvalue: &InsertValue) {
        // dest <- element
        let element = &insertvalue.element;
        let dest = &insertvalue.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = element
        {
            self.state
                .set_tainted(dest, self.state.get_memory_state(value_name));
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#shufflevector-instruction
    fn analyze_shufflevector(&mut self, shufflevector: &ShuffleVector) {
        // dest <- operand0 or operand1
        let operand0 = &shufflevector.operand0;
        let operand1 = &shufflevector.operand1;
        let dest = &shufflevector.dest;
        if let (
            Operand::LocalOperand { name: op1_name, .. },
            Operand::LocalOperand { name: op2_name, .. },
        ) = (operand0, operand1)
        {
            if self.state.is_tainted(op1_name) || self.state.is_tainted(op2_name) {
                let op1_state = self.state.get_memory_state(op1_name);
                let op2_state = self.state.get_memory_state(op2_name);
                self.state.set_tainted(dest, op1_state.union(op2_state));
            } else {
                // Neither op1 nor op2 are tainted, clear the taint state
                self.state.set_tainted(dest, MemoryState::Untainted);
            }
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#getelementptr-instruction
    fn analyze_getelementptr(&mut self, getelementptr: &GetElementPtr) {
        // dest <- address
        let address = &getelementptr.address;
        let dest = &getelementptr.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = address
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://docs.rs/llvm-ir/latest/llvm_ir/instruction/struct.Trunc.html
    fn analyze_trunc(&mut self, trunc: &Trunc) {
        // dest <- operand
        let operand = &trunc.operand;
        let dest = &trunc.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = operand
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#zext-to-instruction
    fn analyze_zext(&mut self, zext: &ZExt) {
        // dest <- operand
        let operand = &zext.operand;
        let dest = &zext.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = operand
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#sext-to-instruction
    fn analyze_sext(&mut self, sext: &SExt) {
        // dest <- operand
        let operand = &sext.operand;
        let dest = &sext.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = operand
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#ptrtoint-to-instruction
    fn analyze_ptrtoint(&mut self, ptrtoint: &PtrToInt) {
        // dest <- operand
        let operand = &ptrtoint.operand;
        let dest = &ptrtoint.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = operand
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://docs.rs/llvm-ir/latest/llvm_ir/instruction/struct.IntToPtr.html
    fn analyze_inttoptr(&mut self, inttoptr: &IntToPtr) {
        // dest <- operand
        let operand = &inttoptr.operand;
        let dest = &inttoptr.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = operand
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#addrspacecast-to-instruction
    fn analyze_addrspacecast(&mut self, addrspacecast: &AddrSpaceCast) {
        // dest <- operand
        let operand = &addrspacecast.operand;
        let dest = &addrspacecast.dest;
        if let Operand::LocalOperand {
            name: value_name, ..
        } = operand
        {
            self.state.propagate_taint(value_name, dest);
        }
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#phi-instruction
    fn analyze_phi(&mut self, phi: &Phi) {
        // dest <- operand
        let dest = &phi.dest;
        for (operand, _) in &phi.incoming_values {
            if let Operand::LocalOperand {
                name: value_name, ..
            } = operand
            {
                if self.state.is_tainted(value_name) {
                    self.state
                        .set_tainted(dest, self.state.get_memory_state(value_name));
                    break;
                }
            }
        }
        // If we go here, meaning that all the operands in `incoming_values` are not tainted
        self.state.set_tainted(dest, MemoryState::Untainted);
    }

    /// https://releases.llvm.org/13.0.0/docs/LangRef.html#alloca-instruction
    fn analyze_alloca(&mut self, alloca: &Alloca) {
        // It is hard to detect memory allocation for complicated types like `Vec`, `CString`, and `String`
        // As a heuristic, we can use the information in the `Alloca` instruction
        // E.g., initializing a `String` will generate LLVM IR like this:
        // %s = alloca %"alloc::string::String", align 8
        if let llvm_ir::types::Type::NamedStructType { name } = &*alloca.allocated_type {
            if name.starts_with("alloc::vec::Vec")
                || name.starts_with("alloc::string::String")
                || name.contains("std::ffi::c_str::CString")
            {
                self.state.set_tainted(&alloca.dest, MemoryState::Tainted);
            }
        }
    }

    fn generate_diagnosis(&mut self, bug_info: BugInfo, seriousness: Seriousness) {
        let diagnosis = Diagnosis {
            seriousness,
            bug_info,
            function_name: utils::demangle_name(&self.func_analysis.function.name),
            // call_stack: self.func_analysis.context.borrow().call_stack.clone(),
        };
        self.func_analysis
            .context
            .borrow_mut()
            .diagnoses
            .insert(diagnosis);
    }
}
