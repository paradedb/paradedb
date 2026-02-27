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
use std::collections::BTreeSet;

use crate::api::HashMap;

thread_local! {
    /// Global collector for planner warnings to deduplicate them across the query planning phase.
    /// Maps Category -> Generic warning message -> Set of contexts (e.g., table names/aliases)
    static PLANNER_WARNINGS: RefCell<HashMap<String, HashMap<String, BTreeSet<String>>>> =
      RefCell::new(HashMap::default());
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
    let ctxs = contexts.to_warning_contexts();
    PLANNER_WARNINGS.with(|warnings| {
        let mut warnings = warnings.borrow_mut();
        let category_entry = warnings.entry(category.into()).or_default();
        let entry = category_entry.entry(message.into()).or_default();
        entry.extend(ctxs);
    });
}

/// Clear warnings for specific contexts within a category.
/// This is used when a successful plan is found for a set of tables, invalidating previous
/// warnings about why a plan couldn't be generated.
pub fn clear_planner_warnings_for_contexts<S: AsRef<str>, C: ToWarningContexts>(
    category: S,
    contexts: C,
) {
    let ctxs_to_remove = contexts.to_warning_contexts();
    if ctxs_to_remove.is_empty() {
        return;
    }

    PLANNER_WARNINGS.with(|warnings| {
        let mut warnings = warnings.borrow_mut();
        if let Some(category_warnings) = warnings.get_mut(category.as_ref()) {
            for contexts in category_warnings.values_mut() {
                for ctx in &ctxs_to_remove {
                    contexts.remove(ctx);
                }
            }
            // Optional: Clean up empty entries
            category_warnings.retain(|_, contexts| !contexts.is_empty());
        }
    });
}

pub fn clear_planner_warnings() {
    PLANNER_WARNINGS.with(|warnings| {
        warnings.borrow_mut().clear();
    });
}

pub fn emit_planner_warnings() {
    PLANNER_WARNINGS.with(|warnings| {
        let warnings = warnings.borrow();
        // Flatten the map to iterate over all messages regardless of category
        for category_warnings in warnings.values() {
            for (message, contexts) in category_warnings.iter() {
                if contexts.is_empty() {
                    pgrx::warning!("{}", message);
                } else {
                    let label = if contexts.len() > 1 {
                        "tables"
                    } else {
                        "table"
                    };

                    let context_str = contexts.iter().cloned().collect::<Vec<_>>().join(", ");
                    pgrx::warning!("{message} ({label}: {context_str})");
                }
            }
        }
    });
}
