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

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::thread::LocalKey;

use pgrx::pg_sys;

use crate::api::HashMap;
use crate::postgres::customscan::CustomScan;
use crate::postgres::customscan::aggregatescan::AggregateScan;
use crate::postgres::customscan::joinscan::JoinScan;

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
    /// If a context is in this set, we suppress any future warnings for it across all categories.
    successful_contexts: BTreeSet<String>,
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

impl ToWarningContexts for String {
    fn to_warning_contexts(self) -> Vec<String> {
        vec![self]
    }
}

impl ToWarningContexts for Vec<String> {
    fn to_warning_contexts(self) -> Vec<String> {
        self
    }
}

/// Add a warning to be emitted at the end of the planning phase.
///
/// # Arguments
/// * `category` - The category of the warning (e.g., Join, Top K)
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
/// * `category` - The category of the warning (e.g., Join, Top K)
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

        // Filter out contexts that have already been successfully planned
        let filtered_ctxs: Vec<String> = ctxs
            .into_iter()
            .filter(|c| !state.successful_contexts.contains(c))
            .collect();

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

/// Mark contexts as successful and clear any previous warnings for them across all categories.
/// This is used when a successful plan is found for a set of tables, invalidating previous
/// warnings about why a plan couldn't be generated.
pub fn mark_contexts_successful<C: ToWarningContexts>(contexts: C) {
    let ctxs_to_remove = contexts.to_warning_contexts();
    if ctxs_to_remove.is_empty() {
        return;
    }

    PLANNER_STATE.with(|state| {
        let mut state = state.borrow_mut();

        for category_warnings in state.warnings.values_mut() {
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

        state.successful_contexts.extend(ctxs_to_remove);
    });
}

pub fn clear_planner_warnings() {
    PLANNER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.warnings.clear();
        state.successful_contexts.clear();
    });
}

/// Render the collected warnings into the lines [`emit_planner_warnings`] will emit.
///
/// Kept separate from the emission so that no borrow of [`PLANNER_STATE`] is alive while Postgres
/// is reporting: `pgrx::warning!` reaches `errfinish()`, which ends in `CHECK_FOR_INTERRUPTS()`.
/// An interrupt there — statement timeout, cancel, recovery conflict — raises an `ERROR` and
/// `siglongjmp`s past these Rust frames without running a single destructor, so a `Ref` guard held
/// across the call would leave the thread-local borrowed for the life of the backend and every
/// later planning cycle would fail in [`clear_planner_warnings`] with "RefCell already borrowed".
///
/// This reads the warnings rather than draining them: [`superseded_by_scan_warning`] still needs
/// them while the plan executes, and they are cleared at the start of the next planning cycle.
fn render_planner_warnings() -> Vec<String> {
    PLANNER_STATE.with(|state| {
        let state = state.borrow();
        // Flatten the map to iterate over all messages regardless of category
        state
            .warnings
            .values()
            .flat_map(|category_warnings| category_warnings.iter())
            .map(|(message, warning_data)| {
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
                    output
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
                    format!("{output} ({label}: {context_str})")
                }
            })
            .collect()
    })
}

pub fn emit_planner_warnings() {
    for message in render_planner_warnings() {
        pgrx::warning!("{message}");
    }
}

//
// Per-statement execution-time warnings.
//
// Unlike the planner warnings above, which are batched and flushed at the end of planning, these
// fire during execution and dedup against the currently-executing statement so a warning raised
// from many search-operator predicate nodes surfaces just once.
//

type StatementId = pg_sys::TimestampTz;

const NEVER: StatementId = pg_sys::TimestampTz::MIN;

thread_local! {
    static WARNED_SEQ_SCAN_AT: Cell<StatementId> = const { Cell::new(NEVER) };
    static WARNED_SPILLED_AT: Cell<StatementId> = const { Cell::new(NEVER) };
}

fn current_statement_id() -> StatementId {
    unsafe { pg_sys::GetCurrentStatementStartTimestamp() }
}

fn warn_once_per_statement(warned_at: &'static LocalKey<Cell<StatementId>>, message: &str) {
    let stmt_id = current_statement_id();
    if warned_at.with(|last| last.replace(stmt_id)) != stmt_id {
        pgrx::warning!("{message}");
    }
}

/// Whether the planner already warned (this statement) that a join or aggregate scan wasn't used.
/// Those warnings name a concrete alternative, so the sequential-scan/spill warnings below would
/// just be noise next to them. The planner state is populated during planning and only cleared at
/// the start of the next planning cycle, so it is still readable while the plan executes.
fn superseded_by_scan_warning() -> bool {
    let categories = [
        JoinScan::NAME.to_str().expect("scan NAME is valid UTF-8"),
        AggregateScan::NAME
            .to_str()
            .expect("scan NAME is valid UTF-8"),
    ];
    PLANNER_STATE.with(|state| {
        let state = state.borrow();
        categories.iter().any(|cat| {
            state
                .warnings
                .get(*cat)
                .is_some_and(|msgs| !msgs.is_empty())
        })
    })
}

/// Warn (once per statement) that a search-operator predicate is being applied as a per-row filter
/// over a sequential scan rather than via the index.
pub fn warn_sequential_scan() {
    if superseded_by_scan_warning() {
        return;
    }
    warn_once_per_statement(
        &WARNED_SEQ_SCAN_AT,
        "the table is being sequentially scanned for this query, so performance may be slow\n\
         if you are not sure why, please file an issue: https://github.com/paradedb/paradedb/issues/new/choose",
    );
}

/// Warn (once per statement) that the materialized filter match set exceeded `work_mem` and
/// spilled to a temporary file.
pub fn warn_filter_spilled() {
    if superseded_by_scan_warning() {
        return;
    }
    warn_once_per_statement(
        &WARNED_SPILLED_AT,
        "the query's filter match set exceeded work_mem and spilled to a temporary file; query performance may be degraded",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_warnings_holds_no_borrow_and_leaves_the_state_readable() {
        clear_planner_warnings();

        add_detailed_planner_warning(
            "Join",
            "JoinScan not used",
            "orders",
            vec!["text".to_string()],
        );
        add_planner_warning(
            "Top K",
            "Top K not used",
            vec!["a".to_string(), "b".to_string()],
        );

        let mut messages = render_planner_warnings();
        messages.sort();

        assert_eq!(
            messages,
            vec![
                "JoinScan not used (type: text) (table: orders)".to_string(),
                "Top K not used (tables: a, b)".to_string(),
            ]
        );

        // Nothing may still be borrowing the state once rendering returns. `pgrx::warning!` can
        // longjmp out of `emit_planner_warnings`, and a guard alive at that moment would poison
        // `PLANNER_STATE` for the rest of the backend.
        assert!(
            PLANNER_STATE.with(|state| state.try_borrow_mut().is_ok()),
            "render_planner_warnings left PLANNER_STATE borrowed"
        );

        // Rendering reads the warnings, it does not consume them: `superseded_by_scan_warning`
        // reads the same state while the plan executes.
        assert_eq!(render_planner_warnings().len(), 2);

        clear_planner_warnings();
        assert!(render_planner_warnings().is_empty());
    }
}
