use llvm_ir::Name;
use log::debug;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::hash::Hash;

/// Represent a set of variables associated with the same memory allocation
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Allocation {
    pub set: BTreeSet<Name>,
}

impl Allocation {
    pub fn new(var: Name) -> Self {
        let mut res = BTreeSet::new();
        res.insert(var.clone());
        Self { set: res }
    }

    pub fn insert(&mut self, var: Name) {
        self.set.insert(var);
    }
}

impl fmt::Debug for Allocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.set)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MemoryState {
    Untainted,
    Tainted,
    Borrowed,
    Forgotten,
    Unknown,
}

/// The partial ordering is like the following:
///
///          Unknown
///             |
///     +---------------+
///     |               |
/// Borrowed        Forgotten
///     |               |
///     +---------------+
///             |
///          Tainted
///             |
///         Untainted
///
impl PartialOrd for MemoryState {
    // We only need "<="
    // Note that this is a partial ordering, `Borrowed` and `Forgotten` are not comparable
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if *self == Self::Untainted {
            Some(Ordering::Less)
        } else if *self == Self::Tainted {
            if *other == Self::Untainted {
                Some(Ordering::Greater)
            } else {
                Some(Ordering::Less)
            }
        } else if *self == Self::Borrowed {
            if *other == Self::Untainted || *other == Self::Tainted {
                Some(Ordering::Greater)
            } else if *other == Self::Forgotten {
                None
            } else {
                Some(Ordering::Less)
            }
        } else if *self == Self::Forgotten {
            if *other == Self::Untainted || *other == Self::Tainted {
                Some(Ordering::Greater)
            } else if *other == Self::Borrowed {
                None
            } else {
                Some(Ordering::Less)
            }
        }
        // if *self == Self::Unknown
        else {
            Some(Ordering::Greater)
        }
    }
}

impl MemoryState {
    /// Compute the least upper bound of two `MemoryState`
    pub fn union(&self, other: Self) -> Self {
        if *self <= other {
            other
        } else if *self >= other {
            *self
        } else {
            // If we go here, meaning that `self` and `other` are not comparable,
            // so they can only be `Borrowed` and `Forgotten`
            assert!(*self == Self::Borrowed || *self == Self::Forgotten);
            assert!(other == Self::Borrowed || other == Self::Forgotten);
            Self::Unknown
        }
    }
}

/// The state of a basic block. Mathematically it is a map lattice,
/// which contains all possible mappings that map from `Allocation` to `MemoryState`.
#[derive(Clone, PartialEq, Eq)]
pub struct BlockState {
    state: HashMap<Allocation, MemoryState>,
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            state: HashMap::new(),
        }
    }
}

impl fmt::Debug for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.state)
    }
}

impl PartialOrd for BlockState {
    // We only need "<="
    // `self` <= `other` iff for all `(alloc, self_state)` pair in `self`,
    // `alloc` is defined in `other` and the corresponding state `other_state` satisfies
    // `self_state` <= `other_state`.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.state == other.state {
            Some(Ordering::Equal)
        } else {
            // First flatten the two hash maps so we can compare them
            let mut self_state_flatten = HashMap::new();
            let mut other_state_flatten = HashMap::new();
            for (alloc, self_state) in &self.state {
                for var in &alloc.set {
                    self_state_flatten.insert(var, self_state);
                }
            }
            for (alloc, other_state) in &other.state {
                for var in &alloc.set {
                    other_state_flatten.insert(var, other_state);
                }
            }
            for (var, self_state) in &self_state_flatten {
                if let Some(other_state) = other_state_flatten.get(var) {
                    if !(self_state <= other_state) {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            return Some(Ordering::Less);
        }
    }
}

impl BlockState {
    // Compute the least upper bound of two `BlockState`
    // pub fn union(&self, other: &Self) -> Self {
    //     let mut res_state = self.state.clone();
    //     for (alloc, other_state) in &other.state {
    //         if let Some(&self_state) = res_state.get(alloc) {
    //             res_state.insert(alloc.clone(), self_state.union(*other_state));
    //         } else {
    //             res_state.insert(alloc.clone(), *other_state);
    //         }
    //     }
    //     Self { state: res_state }
    // }

    /// Compute the least upper bound of two `BlockState`, the logic is a bit complex, see the comments.
    /// The performance may be slow.
    pub fn union(&self, other: &Self) -> Self {
        // The result we want to compute
        let mut res_state = BlockState::default();

        // Get all variables from both `self` and `other`
        let mut all_vars = vec![];
        for alloc in self.state.keys().chain(other.state.keys()) {
            for var in &alloc.set {
                all_vars.push(var);
            }
        }

        for var in all_vars {
            let mut var_state = res_state.get_memory_state(var);
            // If `var` is already in the resulting state, compare its state in `self` and `other`, take the maximum one
            if var_state > MemoryState::Untainted {
                let alloc = res_state.get_allocation(var).unwrap();
                let var_state_self = self.get_memory_state(var);
                let var_state_other = other.get_memory_state(var);
                var_state = var_state.union(var_state_self).union(var_state_other);
                res_state.state.insert(alloc, var_state);
            } else {
                // If `var` is not in the resulting state, try to find the corresponding `Allocation` in `self` and `other`
                if let Some(alloc1) = self.get_allocation(var) {
                    if let Some(alloc2) = other.get_allocation(var) {
                        // If `var` is in both `self` and `other`, take the union of the `Allocation`s
                        let alloc: BTreeSet<_> = alloc1.set.union(&alloc2.set).cloned().collect();
                        // The state is also computed via taking the union
                        let var_state = self.state[&alloc1].union(other.state[&alloc2]);
                        res_state.state.insert(Allocation { set: alloc }, var_state);
                    } else {
                        // If `var` is only in `self`
                        res_state.state.insert(alloc1.clone(), self.state[&alloc1]);
                    }
                } else {
                    if let Some(alloc2) = other.get_allocation(var) {
                        // If `var` is only in `other`
                        res_state.state.insert(alloc2.clone(), other.state[&alloc2]);
                    } else {
                        // If `var` is neither in `self` nor `other`, should be impossible to happen
                        unreachable!(
                            "`var` is neither in `self` nor `other`, should be impossible to happen"
                        );
                    }
                }
            }
        }

        res_state
    }

    pub fn set_tainted(&mut self, var: &Name, state: MemoryState) {
        match self.get_allocation(var) {
            Some(alloc) => {
                if state == MemoryState::Untainted {
                    // If `state` is "Untainted", simply delete it because "Untainted" is default
                    self.state.remove(&alloc);
                } else {
                    let mut alloc = alloc.clone();
                    self.state.remove(&alloc);
                    if let Name::Name(box name) = var {
                        // If `var` has a string name
                        let v: Vec<_> = name.split(".").collect();
                        let mut s = String::new();
                        for elem in v {
                            s.push_str(elem);
                            alloc.insert(Name::Name(Box::new(s.clone())));
                            s.push('.');
                        }
                    }
                    self.state.insert(alloc, state);
                }
            }
            None => {
                if state != MemoryState::Untainted {
                    let mut alloc = Allocation::new(var.clone());
                    // Besides adding `var` to taint state, we also need to add all its
                    // prefixes, e.g., for "%a.really.long.identifier", we will add
                    // "%a.really.long.identifier", "%a.really.long", "%a.really", and "%a"
                    if let Name::Name(box name) = var {
                        // If `var` has a string name
                        let v: Vec<_> = name.split(".").collect();
                        let mut s = String::new();
                        for elem in v {
                            s.push_str(elem);
                            alloc.insert(Name::Name(Box::new(s.clone())));
                            s.push('.');
                        }
                    }
                    self.state.insert(alloc, state);
                }
            }
        }
    }

    /// Return the memory state of variable `var` in the current basic block
    /// If its state is not stored in `self.state`, return "Untainted" by default
    pub fn get_memory_state(&self, var: &Name) -> MemoryState {
        for (alloc, mem_state) in &self.state {
            if alloc.set.contains(var) {
                return *mem_state;
            }
        }
        return MemoryState::Untainted;
    }

    pub fn get_allocation(&self, var: &Name) -> Option<Allocation> {
        for alloc in self.state.keys() {
            if alloc.set.contains(var) {
                return Some(alloc.clone());
            }
        }
        return None;
    }

    pub fn is_tainted(&self, var: &Name) -> bool {
        MemoryState::Tainted <= self.get_memory_state(var)
    }

    pub fn propagate_taint(&mut self, from: &Name, to: &Name) {
        debug!("Propagate taint from {} to {}", from, to);
        debug!("Current state: {:?}", self.state);
        // First get the memory state of `from`
        let from_state = self.get_memory_state(from);
        // If the state of `from` is "untainted", clear the state of `to` if `to` is tainted
        // Note that to be conservative, we only clear the state of variable `to`, instead of
        // all the variables that are alias to `to`.
        if from_state == MemoryState::Untainted {
            if let Some(to_alloc) = self.get_allocation(to) {
                // In general it is hard to change the key of a hash map
                // We simply delete the old element and insert the new one
                let mem_state = self.state[&to_alloc];
                let mut new_to_alloc = to_alloc.clone();
                new_to_alloc.set.remove(to);
                self.state.remove(&to_alloc);
                // If `new_to_alloc` is empty, don't insert it
                if !new_to_alloc.set.is_empty() {
                    self.state.insert(new_to_alloc, mem_state);
                }
            }
        }
        // It the state of `from` is not "untainted", change the state of `to` accordingly
        // Again to be conservative, we change all the variables that are alias to `to`.
        // We merge the two allocation sets of "from" and "to"
        else {
            let mut from_alloc = self.get_allocation(from).unwrap();
            if let Some(mut to_alloc) = self.get_allocation(to) {
                self.state.remove(&from_alloc);
                self.state.remove(&to_alloc);
                from_alloc.set.append(&mut to_alloc.set);
            } else {
                // If "to" is not in the state, add it in "from_alloc"
                // Note that we have to remove the old state first
                self.state.remove(&from_alloc);
                from_alloc.insert(to.clone());
            }
            self.state.insert(from_alloc, from_state);
        }

        debug!("After propagation, state: {:?}", self.state);
    }
}

/// The structure of the static analysis lattice.
/// Mathematically, it is a map lattice that maps each basic block to its state.
/// In the implementation, for each basic block, we store its `BlockState`.
/// Basic blocks are identified by their names.
#[derive(Clone, PartialEq)]
pub struct AbstractDomain {
    map: HashMap<Name, BlockState>,
}

impl Default for AbstractDomain {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl fmt::Debug for AbstractDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.map)
    }
}

impl PartialOrd for AbstractDomain {
    // We only need "<="
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            for (bb, state1) in &self.map {
                if let Some(state2) = other.map.get(&bb) {
                    if !(state1 <= state2) {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            Some(Ordering::Less)
        }
    }
}

impl AbstractDomain {
    pub fn get(&self, name: &Name) -> Option<BlockState> {
        self.map.get(name).map(|state| state.to_owned())
    }

    pub fn insert(&mut self, name: Name, state: BlockState) {
        self.map.insert(name, state);
    }
}

// ----------------------------------------------------------------------------------------

// /// The state of a basic block. Mathematically it is a map lattice,
// /// which contains all possible mappings that map from `Name` to "Tainted"/"Untainted".
// /// The implementation only stores a set of `Name` that are tainted.
// #[derive(Clone, PartialEq, Eq)]
// pub struct TaintState {
//     state: HashSet<Name>,
// }

// impl Default for TaintState {
//     fn default() -> Self {
//         Self {
//             state: HashSet::new(),
//         }
//     }
// }

// impl fmt::Debug for TaintState {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self.state)
//     }
// }

// impl PartialOrd for TaintState {
//     // We only need "<="
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         if self.state == other.state {
//             Some(Ordering::Equal)
//         } else if self.state.is_subset(&other.state) {
//             Some(Ordering::Less)
//         } else {
//             None
//         }
//     }
// }

// impl TaintState {
//     /// Compute the least upper bound of two `TaintState`
//     pub fn union(&self, other: &TaintState) -> Self {
//         let union: HashSet<Name> = self
//             .state
//             .union(&other.state)
//             .map(|var| var.to_owned())
//             .collect();
//         Self { state: union }
//     }

//     pub fn is_tainted(&self, var: &Name) -> bool {
//         self.state.contains(var)
//     }

//     pub fn set_tainted(&mut self, var: &Name, taint: bool) {
//         if taint {
//             // Besides adding `var` to taint state, we also need to add all its
//             // prefixes, e.g., for "%a.really.long.identifier", we will add
//             // "%a.really.long.identifier", "%a.really.long", "%a.really", and "%a"
//             if let Name::Name(box name) = var {
//                 // If `var` has a string name
//                 let v: Vec<_> = name.split(".").collect();
//                 let mut s = String::new();
//                 for elem in v {
//                     s.push_str(elem);
//                     self.state.insert(Name::Name(Box::new(s.clone())));
//                     s.push('.');
//                 }
//             } else {
//                 // Otherwise, `var` doesn't have a string name and is a sequential number, just add it
//                 self.state.insert(var.clone());
//             }
//         } else {
//             self.state.remove(var);
//         }
//     }

//     pub fn propagate_taint(&mut self, from: &Name, to: &Name) {
//         self.set_tainted(to, self.is_tainted(from));
//     }
// }

// /// The structure of the analysis lattice.
// /// Mathematically, it is a map lattice that maps each basic block to its state.
// /// In the implementation, for each basic block, we store its `TaintState`.
// /// Basic blocks are identified by their names.
// #[derive(Clone, PartialEq)]
// pub struct TaintDomain {
//     map: HashMap<Name, TaintState>,
// }

// impl Default for TaintDomain {
//     fn default() -> Self {
//         TaintDomain {
//             map: HashMap::new(),
//         }
//     }
// }

// impl fmt::Debug for TaintDomain {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self.map)
//     }
// }

// impl PartialOrd for TaintDomain {
//     // We only need "<="
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         if self == other {
//             Some(Ordering::Equal)
//         } else {
//             for (bb, state1) in &self.map {
//                 if let Some(state2) = other.map.get(&bb) {
//                     if !(state1 <= state2) {
//                         return None;
//                     }
//                 } else {
//                     return None;
//                 }
//             }
//             Some(Ordering::Less)
//         }
//     }
// }

// impl TaintDomain {
//     pub fn get(&self, name: &Name) -> Option<TaintState> {
//         self.map.get(name).map(|state| state.to_owned())
//     }

//     pub fn insert(&mut self, name: Name, state: TaintState) {
//         self.map.insert(name, state);
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_state_ordering() {
        let untainted = MemoryState::Untainted;
        let tainted = MemoryState::Tainted;
        let borrowed = MemoryState::Borrowed;
        let forgotten = MemoryState::Forgotten;
        let unknown = MemoryState::Unknown;
        assert!(untainted < tainted);
        assert!(untainted < borrowed);
        assert!(untainted < forgotten);
        assert!(untainted < unknown);

        assert!(tainted > untainted);
        assert!(tainted < borrowed);
        assert!(tainted < forgotten);
        assert!(tainted < unknown);

        assert!(borrowed > untainted);
        assert!(borrowed > tainted);
        assert!(!(borrowed < forgotten));
        assert!(!(borrowed == forgotten));
        assert!(!(borrowed > forgotten));
        assert!(borrowed < unknown);

        assert!(forgotten > untainted);
        assert!(forgotten > tainted);
        assert!(forgotten < unknown);

        assert!(unknown > untainted);
        assert!(unknown > tainted);
        assert!(unknown > borrowed);
        assert!(unknown > forgotten);
    }

    #[test]
    fn test_mem_state_union() {
        let untainted = MemoryState::Untainted;
        let tainted = MemoryState::Tainted;
        let borrowed = MemoryState::Borrowed;
        let forgotten = MemoryState::Forgotten;
        let unknown = MemoryState::Unknown;

        assert_eq!(untainted.union(tainted), tainted);
        assert_eq!(untainted.union(borrowed), borrowed);
        assert_eq!(tainted.union(forgotten), forgotten);
        assert_eq!(borrowed.union(forgotten), unknown);
        assert_eq!(borrowed.union(unknown), unknown);
    }

    #[test]
    fn test_block_state_order() {
        let untainted = MemoryState::Untainted;
        let tainted = MemoryState::Tainted;
        let borrowed = MemoryState::Borrowed;
        let forgotten = MemoryState::Forgotten;
        let unknown = MemoryState::Unknown;

        let name1 = Name::Number(1);
        let name2 = Name::Number(2);
        let name3 = Name::Number(3);
        let name4 = Name::Number(4);
        let name5 = Name::Number(5);

        let alloc1 = Allocation {
            set: BTreeSet::from([name1, name2]),
        };
        let alloc2 = Allocation {
            set: BTreeSet::from([name3, name4]),
        };
        let alloc3 = Allocation {
            set: BTreeSet::from([name5]),
        };

        let state1 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), tainted)]),
        };

        let state2 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), borrowed)]),
        };

        let state3 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), forgotten)]),
        };

        let state4 = BlockState {
            state: HashMap::from([
                (alloc1.clone(), untainted),
                (alloc2.clone(), forgotten),
                (alloc3.clone(), unknown),
            ]),
        };

        assert!(state1 < state2);
        assert!(!(state2 < state3));
        assert!(!(state2 > state3));
        assert!(!(state2 == state3));
        assert!(!(state2 < state4));
        assert!(!(state2 > state4));
        assert!(!(state2 == state4));
        assert!(state3 < state4);
    }

    #[test]
    fn test_block_state_union() {
        let untainted = MemoryState::Untainted;
        let tainted = MemoryState::Tainted;
        let borrowed = MemoryState::Borrowed;
        let forgotten = MemoryState::Forgotten;
        let unknown = MemoryState::Unknown;

        let name1 = Name::Number(1);
        let name2 = Name::Number(2);
        let name3 = Name::Number(3);
        let name4 = Name::Number(4);
        let name5 = Name::Number(5);
        let name6 = Name::Number(6);

        let alloc1 = Allocation {
            set: BTreeSet::from([name1.clone(), name2.clone()]),
        };
        let alloc2 = Allocation {
            set: BTreeSet::from([name3.clone(), name4.clone()]),
        };
        let alloc3 = Allocation {
            set: BTreeSet::from([name5.clone()]),
        };
        let alloc4 = Allocation {
            set: BTreeSet::from([name1.clone(), name2.clone(), name3.clone()]),
        };
        let alloc5 = Allocation {
            set: BTreeSet::from([name3.clone()]),
        };
        let alloc6 = Allocation {
            set: BTreeSet::from([name4.clone(), name5.clone(), name6.clone()]),
        };

        // {{1,2}: U, {3,4}: T}
        let state1 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), tainted)]),
        };

        // {{1,2}: U, {3,4}: B}
        let state2 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), borrowed)]),
        };

        // {{1,2}: U, {3,4}: F}
        let state3 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), forgotten)]),
        };

        // {{1,2}: U, {3,4}: Unknown}
        let state4 = BlockState {
            state: HashMap::from([(alloc1.clone(), untainted), (alloc2.clone(), unknown)]),
        };

        // {{1,2}: U, {3,4}: F, {5}: Unknown}
        let state5 = BlockState {
            state: HashMap::from([
                (alloc1.clone(), untainted),
                (alloc2.clone(), forgotten),
                (alloc3.clone(), unknown),
            ]),
        };

        // {{1,2}: U, {3,4}: Unknown, {5}: Unknown}
        let state6 = BlockState {
            state: HashMap::from([
                (alloc1.clone(), untainted),
                (alloc2.clone(), unknown),
                (alloc3.clone(), unknown),
            ]),
        };

        // {{1,2,3}: T, {4,5,6}: Unknown}
        let state7 = BlockState {
            state: HashMap::from([(alloc4.clone(), tainted), (alloc6.clone(), unknown)]),
        };

        // {{1,2}: T, {3}: F}
        let state8 = BlockState {
            state: HashMap::from([(alloc1.clone(), tainted), (alloc5.clone(), forgotten)]),
        };

        // {{1,2,3}: F, {4,5,6}: Unknown}
        let state9 = BlockState {
            state: HashMap::from([(alloc4.clone(), forgotten), (alloc6.clone(), unknown)]),
        };

        assert!(state1.union(&state2) == state2);
        assert!(state2.union(&state3) == state4);
        assert!(state1.union(&state5) == state5);
        assert!(state2.union(&state5) == state6);
        assert!(state7.union(&state8) == state9);

        assert_eq!(state5.is_tainted(&name3), true);
        assert_eq!(state5.is_tainted(&name1), false);
    }
}
