use std::collections::HashMap;

use pgrx::{
    item_pointer_get_both,
    pg_sys::{BlockNumber, ItemPointerData, OffsetNumber},
};
use tantivy::Snippet;

static mut MANAGER: Manager = Manager::new();

pub fn get_executor_manager() -> &'static mut Manager {
    unsafe { &mut MANAGER }
}

type BlockInfo = (BlockNumber, OffsetNumber);

#[derive(Debug, PartialEq)]
pub struct Manager {
    max_score: f32,
    min_score: f32,
    scores: Option<HashMap<BlockInfo, f32>>,
    highlights: Option<HashMap<(BlockInfo, String), String>>,
}

impl Manager {
    pub const fn new() -> Self {
        Self {
            scores: None,
            highlights: None,
            max_score: 0.0,
            min_score: 0.0,
        }
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

    pub fn set_max_score(&mut self, max_score: f32) {
        self.max_score = max_score;
    }

    pub fn get_max_score(&self) -> f32 {
        self.max_score
    }

    pub fn set_min_score(&mut self, min_score: f32) {
        self.min_score = min_score;
    }

    pub fn get_min_score(&self) -> f32 {
        self.min_score
    }

    pub fn add_highlight(&mut self, ctid: ItemPointerData, field_name: String, snippet: Snippet) {
        if self.highlights.is_none() {
            self.highlights.replace(HashMap::new());
        }

        let highlighted_str = self.parse_snippet(snippet);
        self.highlights.as_mut().unwrap().insert(
            (item_pointer_get_both(ctid), field_name.to_string()),
            highlighted_str,
        );
    }

    pub fn get_highlight(&mut self, ctid: ItemPointerData, field_name: String) -> Option<String> {
        let (block, offset) = item_pointer_get_both(ctid);
        Some(
            self.highlights
                .as_mut()
                .unwrap()
                .get(&((block, offset), field_name))
                .expect("failed to get highlight")
                .clone(),
        )
    }

    fn parse_snippet(&self, snippet: Snippet) -> String {
        snippet.to_html()
    }
}
