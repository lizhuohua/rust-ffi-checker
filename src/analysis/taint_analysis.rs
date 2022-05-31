use crate::analysis::abstract_domain::AbstractDomain;
use crate::analysis::abstract_domain::{BlockState, MemoryState};
use crate::analysis::block_visitor::BlockVisitor;
use crate::analysis::context::GlobalContext;
use crate::analysis::diagnosis::Diagnosis;
use crate::utils;
// use llvm_ir_analysis::ControlFlowGraph;
// use llvm_ir::name::Name;
use llvm_ir::terminator::Terminator;
use llvm_ir::BasicBlock;
use llvm_ir_analysis::FunctionAnalysis;
use log::{debug, info, warn};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

const MAX_ITERATION: u32 = 200;
const MAX_DEPTH: u32 = 20;

/// Represents the whole static analysis task, which includes analyzing multiple functions
pub struct StaticAnalysis {
    pub context: Rc<RefCell<GlobalContext>>,
}

impl StaticAnalysis {
    pub fn new(context: Rc<RefCell<GlobalContext>>) -> Self {
        Self {
            context: context.clone(),
        }
    }

    pub fn run(&mut self) {
        // To avoid borrow of `self.context`
        let entry_points = self.context.borrow().analysis_option.entry_points.clone();
        let functions = self.context.borrow().functions.clone();
        // Analyze each entry point
        for entry_func in &entry_points {
            // Because `entry_func` is collected from HIR, so we only know the function name.
            // But in LLVM IR, the function names have prefixes, e.g., `crate_name::StructName::func_name`.
            // So for each `entry_func`, there may be many associated functions in LLVM IR, we find and analyze all of them.
            let mut found = false;
            for func_full_name in functions.keys() {
                if utils::demangle_name(func_full_name).ends_with(entry_func) {
                    found = true;
                    // This unwrap should never panic, this `func_full_name` is gotten from `functions.keys()`
                    let mut func_analysis =
                        FuncAnalysis::new(self.context.clone(), func_full_name).unwrap();
                    info!("Analyzing function: {}", entry_func);
                    func_analysis.iterate_to_fixpoint();
                }
            }
            if found == false {
                warn!("LLVM bitcode for entry point: {} is not found", entry_func);
            }
        }
    }

    pub fn output_diagnoses(&self) {
        // For each function, we only issue one warning
        // If many warnings are generated for the same function name, only output the most severe one
        let mut diagnoses_to_issue: HashMap<String, Diagnosis> = HashMap::new();
        for diagnosis in &self.context.borrow().diagnoses {
            let threshold = self.context.borrow().analysis_option.precision_threshold;
            if diagnosis.seriousness >= threshold {
                // Ignore warnings that from `core` and `clang_sys`, etc., these are not our analysis target
                // if !(diagnosis.function_name.starts_with("core::")
                //     || diagnosis.function_name.starts_with("clang_sys::")
                //     || diagnosis.function_name.starts_with("<core::")
                //     || diagnosis.function_name.starts_with("alloc::")
                //     || diagnosis.function_name.starts_with("<alloc::")
                //     || diagnosis.function_name.starts_with("tokio::"))

                // Only output warnings that are from the current crate, this is distinguished by checking
                // whether the function name starts with the crate name, e.g., "core::...", "alloc::...".
                // But FFI functions are usually public and do not have the suffix, so we also output warnings
                // if function name doesn't contain "::".
                if self
                    .context
                    .borrow()
                    .analysis_option
                    .crate_names
                    .iter()
                    .any(|name| diagnosis.function_name.starts_with(name))
                    || !diagnosis.function_name.contains("::")
                {
                    if let Some(diagn) = diagnoses_to_issue.get(&diagnosis.function_name) {
                        if diagn.seriousness <= diagnosis.seriousness {
                            diagnoses_to_issue
                                .insert(diagnosis.function_name.clone(), diagnosis.clone());
                        }
                    } else {
                        diagnoses_to_issue
                            .insert(diagnosis.function_name.clone(), diagnosis.clone());
                    }
                }
            }
        }
        for diagnosis in diagnoses_to_issue {
            println!("{:?}", diagnosis);
        }
    }
}

/// Represents analyzing a single function
/// It may launch another `FuncAnalysis` because of the interprocedural analysis
pub struct FuncAnalysis {
    pub context: Rc<RefCell<GlobalContext>>,
    pub init_state: BlockState,
    pub function: llvm_ir::Function,
    pub taint_domain: AbstractDomain,
    /// Indicates the state of the return value
    pub ret_state: MemoryState,
    /// The current depth of interprocedural analysis
    pub depth: u32,
    pub iteration: u32,
}

impl FuncAnalysis {
    /// Initialize a function analysis given the entry function `func_name`, if the function name is not found, return `None`
    pub fn new(context: Rc<RefCell<GlobalContext>>, func_name: &String) -> Option<Self> {
        // Clear and initialize the call stack, since we want to launch a new function analysis
        // context.borrow_mut().call_stack = vec![utils::demangle_name(func_name)];

        if let Some(function) = context.borrow().functions.get(func_name) {
            Some(Self {
                context: context.clone(),
                init_state: BlockState::default(),
                function: function.clone(),
                taint_domain: AbstractDomain::default(),
                ret_state: MemoryState::Untainted,
                depth: 1,
                iteration: 0,
            })
        } else {
            // Cannot find the LLVM bitcode for `func_name`, abort
            None
        }
    }

    /// Initialize a function analysis given the initial state `init`
    /// This is used for interprocedural analysis, where we need to follow a function call
    /// and start an analysis under the current state (the call stack is also inherited)
    /// If `func_name` is not found, return `None`
    pub fn new_with_init(
        context: Rc<RefCell<GlobalContext>>,
        func_name: &String,
        init: &BlockState,
        depth: u32,
    ) -> Option<Self> {
        if depth >= MAX_DEPTH {
            return None;
        }
        // Remember the call stack and restore it after `Self::new`
        // let call_stack = context.borrow().call_stack.clone();
        if let Some(mut res) = Self::new(context, func_name) {
            // Rewrite the initial state
            res.init_state = init.clone();
            // Restore the call stack
            // res.context.borrow_mut().call_stack = call_stack;
            // Assign the depth of function call
            res.depth = depth;
            Some(res)
        } else {
            None
        }
    }

    /// Start the fixed point algorithm until a fixed point is reached
    pub fn iterate_to_fixpoint(&mut self) {
        let mut old_state = self.taint_domain.clone();
        let mut worklist = VecDeque::from(self.function.basic_blocks.clone());

        let mut iteration = 0;
        while let Some(bb) = worklist.pop_front() {
            self.analyze_basic_block(&bb);
            let new_state = self.get_state_from_predecessors(&bb);
            if old_state.get(&bb.name) == None || !(new_state <= old_state.get(&bb.name).unwrap()) {
                debug!("old: {:?}", old_state);
                old_state.insert(bb.name.clone(), new_state);
                debug!("new: {:?}", old_state);
                let mut successors = self.get_successors(&bb);
                debug!(
                    "Adding successors of {} to the worklist: {:?}",
                    bb.name, successors
                );
                worklist.append(&mut successors);
                // worklist.append(&mut self.get_successors(&bb));
            }

            // To make stop analysis if it takes too much time
            iteration += 1;
            if iteration > MAX_ITERATION {
                break;
            }
        }

        // loop {
        //     // if self.iteration >= MAX_ITERATION {
        //     //     // If the maximum iteration limit is reached
        //     //     break;
        //     // }
        //     self.iteration += 1;
        //     let basic_blocks = self.function.basic_blocks.clone();
        //     for bb in &basic_blocks {
        //         self.analyze_basic_block(bb);
        //     }
        //     if self.taint_domain <= old_state {
        //         // A fix point is reached
        //         break;
        //     } else {
        //         debug!("old: {:?}", old_state);
        //         debug!("new: {:?}", self.taint_domain);
        //         old_state = self.taint_domain.clone();
        //     }
        // }
    }

    /// Get the state after this function call
    pub fn get_state_after_call(&self) -> BlockState {
        let mut result = BlockState::default();
        for bb in &self.function.basic_blocks {
            if matches!(bb.term, Terminator::Ret(..)) {
                if let Some(state) = self.taint_domain.get(&bb.name) {
                    result = result.union(&state);
                }
            }
        }
        result
    }

    /// Start analyzing a basic block
    fn analyze_basic_block(&mut self, bb: &BasicBlock) {
        debug!(
            "Analyzing basic block: {} in function {}",
            bb.name, self.function.name
        );
        let pre_condition = if self.function.basic_blocks[0] == *bb {
            // If this is the first basic block of the function being analyzed
            // Initialize the pre-condition using the initial state
            self.init_state.clone()
        } else {
            // Else, initialize the pre-condition by gathering the states from all the predecessors
            self.get_state_from_predecessors(bb)
        };
        debug!("Pre condition: {:?}", pre_condition);

        // Analyze through a block visitor
        let mut block_visitor = BlockVisitor::new(self, &pre_condition, bb.clone());
        let post_condition = block_visitor.analyze();
        debug!("After analyzing, post condition: {:?}", post_condition);

        // Update the post-condition
        self.taint_domain.insert(bb.name.clone(), post_condition);
    }

    fn get_state_from_predecessors(&self, bb: &BasicBlock) -> BlockState {
        let function_analysis = Rc::new(FunctionAnalysis::new(&self.function));
        let cfg = function_analysis.control_flow_graph();
        let mut res = BlockState::default();
        for pred in cfg.preds(&bb.name) {
            if let Some(pred_state) = self.taint_domain.get(pred) {
                res = res.union(&pred_state);
            }
        }
        res
    }

    fn get_successors(&self, bb: &BasicBlock) -> VecDeque<BasicBlock> {
        let function_analysis = Rc::new(FunctionAnalysis::new(&self.function));
        let cfg = function_analysis.control_flow_graph();
        cfg.succs(&bb.name)
            .filter_map(|x| {
                if let llvm_ir_analysis::CFGNode::Block(name) = x {
                    let mut basicblock = BasicBlock::new(name.clone());
                    let mut found = false;
                    for bb in &self.function.basic_blocks {
                        if &bb.name == name {
                            found = true;
                            basicblock = bb.clone();
                            break;
                        }
                    }
                    assert!(found);
                    Some(basicblock)
                } else {
                    None
                }
            })
            .collect()
    }
}
