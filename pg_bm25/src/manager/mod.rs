use std::collections::HashMap;

use pgrx::{
    item_pointer_get_both,
    pg_sys::{BlockNumber, ItemPointerData, OffsetNumber},
};

static mut MANAGER: Manager = Manager::new();

pub fn get_executor_manager() -> &'static mut Manager {
    unsafe { &mut MANAGER }
}

pub struct Manager {
    pub scores: Option<HashMap<(BlockNumber, OffsetNumber), f32>>,
}

impl Manager {
    pub const fn new() -> Self {
        Self { scores: None }
    }

    pub fn add_score(&mut self, ctid: (BlockNumber, OffsetNumber), score: f32) {
        if self.scores.is_none() {
            self.scores.replace(HashMap::new());
        }

        self.scores.as_mut().unwrap().insert(ctid, score);
    }

    pub fn get_score(&mut self, ctid: ItemPointerData) -> Option<f32> {
        let (block, offset) = item_pointer_get_both(ctid);
        self.scores.as_mut().unwrap().get(&(block, offset)).copied()
    }
}
