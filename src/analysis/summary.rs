use crate::analysis::abstract_domain::{BlockState, MemoryState};
use std::collections::HashMap;

/// A key is a pair of function name and context
pub type SummaryKey = (String, Vec<Option<MemoryState>>);
/// A summary is a pair of state and a `MemoryState`. The `MemoryState` indicates whether the return value is tainted
pub type Summary = (BlockState, MemoryState);

/// Store the summaries that have been computed
/// Given a `SummaryKey`, it returns the corresponding `BlockState` if possible
pub struct SummaryCache {
    map: HashMap<SummaryKey, Summary>,
}

impl Default for SummaryCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl SummaryCache {
    pub fn insert(&mut self, key: &SummaryKey, state: Summary) {
        self.map.insert(key.clone(), state);
    }

    pub fn get(&self, key: &SummaryKey) -> Option<&Summary> {
        self.map.get(key)
    }
}
