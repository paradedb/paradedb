// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

#[derive(Default)]
pub struct WarningData {
    pub contexts: BTreeSet<String>,
    pub details: BTreeSet<String>,
    pub has_contexts: bool,
}

#[derive(Default)]
struct PlannerWarningState {
    /// Maps Category -> Generic warning message -> WarningData
    warnings: HashMap<String, HashMap<String, WarningData>>,
    /// Contexts (e.g., table aliases) that have successfully been planned in the current query planning phase.
    /// Maps Category -> Set of successful contexts.
    /// If a context is in this set, we suppress any future warnings for it in the same category.
    successful_contexts: HashMap<String, BTreeSet<String>>,
}

thread_local! {
    /// Global state for planner warnings and successes to deduplicate them across the query planning phase.
    static PLANNER_STATE: RefCell<PlannerWarningState> = RefCell::new(PlannerWarningState::default());
}

/// Trait to convert various types into a list of warning contexts (strings).
pub trait ToWarningContexts {
    fn to_warning_contexts(self) -> Vec<String>;
}

impl ToWarningContexts for () {
    fn to_warning_contexts(self) -> Vec<String> {
        Vec::new()
    }
}

impl ToWarningContexts for &Vec<String> {
    fn to_warning_contexts(self) -> Vec<String> {
        self.to_vec()
    }
}

impl ToWarningContexts for &[String] {
    fn to_warning_contexts(self) -> Vec<String> {
        self.to_vec()
    }
}

impl ToWarningContexts for &str {
    fn to_warning_contexts(self) -> Vec<String> {
        vec![self.to_string()]
    }
}

/// Add a warning to be emitted at the end of the planning phase.
///
/// # Arguments
/// * `category` - The category of the warning (e.g., Join, TopN)
/// * `message` - The generic warning message (e.g., "JoinScan not used: query must have a LIMIT clause")
/// * `contexts` - Contexts (e.g., table aliases) to associate with this warning.
///   Supported types:
///   - `()`: No context (global warning)
///   - `&str`: Single context
///   - `&Vec<String>`: List of contexts
pub fn add_planner_warning<S1: Into<String>, S2: Into<String>, C: ToWarningContexts>(
    category: S1,
    message: S2,
    contexts: C,
) {
    add_detailed_planner_warning(category, message, contexts, Vec::<String>::new())
}

/// Add a detailed warning to be emitted at the end of the planning phase.
///
/// # Arguments
/// * `category` - The category of the warning (e.g., Join, TopN)
/// * `message` - The generic warning message (e.g., "JoinScan not used: query must have a LIMIT clause")
/// * `contexts` - Contexts (e.g., table aliases) to associate with this warning.
/// * `details` - Additional details (e.g. types) to union for this warning.
pub fn add_detailed_planner_warning<
    S1: Into<String>,
    S2: Into<String>,
    C: ToWarningContexts,
    D: IntoIterator<Item = String>,
>(
    category: S1,
    message: S2,
    contexts: C,
    details: D,
) {
    let ctxs = contexts.to_warning_contexts();
    let category_str = category.into();
    let message_str = message.into();
    let had_contexts = !ctxs.is_empty();

    let details_vec: Vec<String> = details.into_iter().collect();

    PLANNER_STATE.with(|state| {
        let mut state = state.borrow_mut();

        // Filter out contexts that have already been successfully planned for this category
        let filtered_ctxs: Vec<String> =
            if let Some(category_successes) = state.successful_contexts.get(&category_str) {
                ctxs.into_iter()
                    .filter(|c| !category_successes.contains(c))
                    .collect()
            } else {
                ctxs
            };

        // If we originally had contexts, but all of them were filtered out because they were successful,
        // we should completely suppress this warning.
        if had_contexts && filtered_ctxs.is_empty() {
            return;
        }

        let category_entry = state.warnings.entry(category_str).or_default();
        let entry = category_entry.entry(message_str).or_default();
        if !filtered_ctxs.is_empty() {
            entry.has_contexts = true;
        }
        entry.contexts.extend(filtered_ctxs);
        entry.details.extend(details_vec);
    });
}

/// Mark contexts within a category as successful and clear any previous warnings for them.
/// This is used when a successful plan is found for a set of tables, invalidating previous
/// warnings about why a plan couldn't be generated.
pub fn mark_contexts_successful<S: AsRef<str>, C: ToWarningContexts>(category: S, contexts: C) {
    let ctxs_to_remove = contexts.to_warning_contexts();
    if ctxs_to_remove.is_empty() {
        return;
    }

    let category_str = category.as_ref().to_string();

    PLANNER_STATE.with(|state| {
        let mut state = state.borrow_mut();

        if let Some(category_warnings) = state.warnings.get_mut(&category_str) {
            for warning_data in category_warnings.values_mut() {
                for ctx in &ctxs_to_remove {
                    warning_data.contexts.remove(ctx);
                }
            }
            // Optional: Clean up empty entries
            category_warnings.retain(|_, warning_data| {
                !warning_data.has_contexts || !warning_data.contexts.is_empty()
            });
        }

        let category_successes = state.successful_contexts.entry(category_str).or_default();
        category_successes.extend(ctxs_to_remove);
    });
}

pub fn clear_planner_warnings() {
    PLANNER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.warnings.clear();
        state.successful_contexts.clear();
    });
}

pub fn emit_planner_warnings() {
    PLANNER_STATE.with(|state| {
        let state = state.borrow();
        // Flatten the map to iterate over all messages regardless of category
        for category_warnings in state.warnings.values() {
            for (message, warning_data) in category_warnings.iter() {
                let mut output = message.clone();

                if !warning_data.details.is_empty() {
                    let details_label = if warning_data.details.len() > 1 {
                        "types"
                    } else {
                        "type"
                    };
                    let details_str = warning_data
                        .details
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    output = format!("{output} ({details_label}: {details_str})");
                }

                if warning_data.contexts.is_empty() {
                    pgrx::warning!("{}", output);
                } else {
                    let label = if warning_data.contexts.len() > 1 {
                        "tables"
                    } else {
                        "table"
                    };

                    let context_str = warning_data
                        .contexts
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    pgrx::warning!("{output} ({label}: {context_str})");
                }
            }
        }
    });
}
