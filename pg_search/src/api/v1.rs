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

#![allow(unsafe_op_in_unsafe_fn)]

use crate::api::operator::is_search_with_query_input_exec_procoid;
use crate::gucs::{PlannerWarnings, v1api_warnings};
use crate::nodecast;
use crate::postgres::catalog::lookup_namespace;
use crate::postgres::planner_warnings::{ExplainMode, explain_mode};
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::{PgList, PgLogLevel, PgSqlErrorCode, pg_guard, pg_sys};
use std::collections::{BTreeMap, HashSet};
use std::ffi::{CStr, c_void};

#[derive(Clone, Copy)]
enum MigrationHint {
    Replacement(&'static str),
    Gap(&'static str),
}

use MigrationHint::{Gap, Replacement};

const SUPPORT_ISSUE_URL: &str =
    "https://github.com/paradedb/paradedb/issues/new?template=feature_request.yml";
const LEGACY_QUERY_TYPE: &str = "paradedb.searchqueryinput";
const LEGACY_QUERY_HINT: MigrationHint = Gap(
    "Use pdb query builders or the V2 search operators. V2 has no equivalent for arbitrary searchqueryinput JSON.",
);

fn replacement(name: &str) -> Option<MigrationHint> {
    Some(match name {
        "all" => Replacement(
            "Use table_alias @@@ pdb.all()::pdb.const(0) to preserve the V1 score of zero.",
        ),
        "empty" => Replacement("Use table_alias @@@ pdb.empty()."),
        "parse" => {
            Replacement("Use table_alias @@@ pdb.parse(query_string, lenient, conjunction_mode).")
        }
        "parse_with_field" => Replacement(
            "Use field @@@ pdb.parse_with_field(query_string, lenient, conjunction_mode).",
        ),
        "boolean" => Gap(
            "Combine required and excluded predicates with SQL AND, OR, and NOT. For minimum_should_match, count true predicates with CASE expressions. V2 has no direct equivalent for optional clauses that contribute only to scoring.",
        ),
        "boost" => Gap(
            "Cast an individual search operand to pdb.boost(factor). Boosting an arbitrary compound search predicate has no direct V2 equivalent.",
        ),
        "const_score" => Gap(
            "Cast an individual search operand to pdb.const(score). Assigning one constant score to an arbitrary compound search predicate has no direct V2 equivalent.",
        ),
        "disjunction_max" => Gap(
            "V2 has no equivalent for DisMax scoring or its tie_breaker. SQL OR can match the same rows with different scores.",
        ),
        "exists" => Replacement("Use field @@@ pdb.exists()."),
        "fuzzy_term" => Replacement(
            "Use field @@@ pdb.fuzzy_term(value, distance, transposition_cost_one, prefix).",
        ),
        "match" => Replacement(
            "Use field @@@ pdb.match(value, ...) with the same options, or use field ||| value for the default match.",
        ),
        "match_conjunction" => Replacement("Use field &&& value."),
        "match_disjunction" => Replacement("Use field ||| value."),
        "phrase" => Replacement(
            "Use field @@@ pdb.phrase(phrases, slop), or use field ### phrase for the default phrase match.",
        ),
        "phrase_array" => {
            Replacement("Use field ### tokens or field @@@ pdb.phrase_array(tokens).")
        }
        "phrase_prefix" => Replacement("Use field @@@ pdb.phrase_prefix(phrases, max_expansion)."),
        "proximity" => Replacement(
            "Use field @@@ pdb.proximity(...), or use field @@@ (left_term ## distance ## right_term).",
        ),
        "proximity_in_order" => Replacement(
            "Use field @@@ pdb.proximity_in_order(...), or use field @@@ (left_term ##> distance ##> right_term).",
        ),
        "range" => Replacement("Use field @@@ pdb.range(range)."),
        "range_term" => Replacement(
            "Use field @@@ pdb.range_term(value) or field @@@ pdb.range_term(range, relation).",
        ),
        "regex" => Replacement("Use field @@@ pdb.regex(pattern)."),
        "regex_phrase" => {
            Replacement("Use field @@@ pdb.regex_phrase(regexes, slop, max_expansions).")
        }
        "term" => {
            Replacement("Use field @@@ pdb.term(value), or use field === value for text tokens.")
        }
        "term_set" => Gap(
            "Use field @@@ pdb.term_set(values) for a single field. For terms spanning multiple fields, combine per-field predicates with SQL OR. This can change scores.",
        ),
        "term_with_operator" => Replacement(
            "Use SQL operators for column-value comparisons. To match an indexed text token, use field @@@ pdb.term(value).",
        ),
        "terms_with_operator" => Replacement(
            "Use SQL comparisons with ANY(array) or ALL(array) for column values. To match any indexed token in an array, use field @@@ pdb.term_set(values).",
        ),
        "to_search_query_input" => Replacement(
            "Use field @@@ query for a pdb.query value. Use the table alias for an unfielded query.",
        ),
        "with_index" => Gap(
            "Apply the V2 predicate to the indexed table or column so ParadeDB can resolve the index. V2 has no explicit index override.",
        ),
        "score" => Replacement("Use pdb.score(...) with the same arguments."),
        "snippet" => Replacement(
            "Use pdb.snippet(...) with the same arguments. For multiple snippets, use pdb.snippets(...).",
        ),
        "snippets" => Replacement("Use pdb.snippets(...) with the same arguments."),
        "snippet_positions" => {
            Replacement("Use pdb.snippet_positions(...) with the same arguments.")
        }
        "aggregate" => Gap(
            "Use SELECT pdb.agg(aggregation::jsonb) FROM table_name WHERE search_predicate. V2 has no standalone index aggregate function with per-call resource limits.",
        ),
        "field" => Gap(
            "Configure fields in the CREATE INDEX column list. Use tokenizer casts for text and JSON, alias and columnar for field aliases and columnar storage, and omit stored. Custom record modes, indexed=false, and expand_dots=false have no direct V2 equivalent.",
        ),
        "tokenizer" => Gap(
            "Use a pdb tokenizer cast, such as field::pdb.simple('stemmer=english'). For a query-time override, cast the search text or pass tokenizer JSON directly to pdb.match. Tokenizer casts do not support custom stopword lists.",
        ),
        "tokenize" => Gap(
            "Use input_text::pdb.simple::text[] with the corresponding tokenizer and options. Tokenizer casts return tokens without their original positions.",
        ),
        "tokenizers" => Gap(
            "V2 has no dedicated SQL function to list tokenizer names. See https://www.paradedb.com/docs/reference/tokenizers/overview for available tokenizer casts.",
        ),
        "jsonb_to_searchqueryinput" | "searchqueryinput_in" => LEGACY_QUERY_HINT,
        _ => return None,
    })
}

fn report(mode: PlannerWarnings, feature: &str, hint: MigrationHint) {
    let level = match mode {
        PlannerWarnings::Off => return,
        PlannerWarnings::Error if explain_mode() != ExplainMode::PlanOnly => PgLogLevel::ERROR,
        _ => PgLogLevel::WARNING,
    };
    let hint = match hint {
        Replacement(message) => message.to_owned(),
        Gap(message) => format!(
            "{message} To request V2 support, open a GitHub issue with your SQL and index definition at {SUPPORT_ISSUE_URL}"
        ),
    };
    ErrorReport::new(
        PgSqlErrorCode::ERRCODE_WARNING_DEPRECATED_FEATURE,
        format!("{feature} uses the deprecated ParadeDB V1 API"),
        pgrx::function_name!(),
    )
    .set_hint(hint)
    .report(level);
}

struct QueryCalls {
    extension: pg_sys::Oid,
    namespace: pg_sys::Oid,
    query_types: Option<[pg_sys::Oid; 2]>,
    seen_functions: HashSet<pg_sys::Oid>,
    found: BTreeMap<String, MigrationHint>,
}

impl QueryCalls {
    unsafe fn function(&mut self, oid: pg_sys::Oid) {
        if oid.to_u32() < pg_sys::FirstNormalObjectId
            || !self.seen_functions.insert(oid)
            || pg_sys::get_func_namespace(oid) != self.namespace
        {
            return;
        }

        let name = pg_sys::get_func_name(oid);
        if name.is_null() {
            return;
        }
        let name_str = CStr::from_ptr(name).to_str().ok();
        if let Some((name, hint)) =
            name_str.and_then(|name| replacement(name).map(|hint| (name, hint)))
            && pg_sys::getExtensionOfObject(pg_sys::ProcedureRelationId, oid) == self.extension
        {
            let feature = match name {
                "jsonb_to_searchqueryinput" | "searchqueryinput_in" => LEGACY_QUERY_TYPE.to_owned(),
                _ => format!("paradedb.{name}()"),
            };
            self.found.insert(feature, hint);
        }
        pg_sys::pfree(name.cast());
    }

    unsafe fn is_legacy_query_type(&mut self, oid: pg_sys::Oid) -> bool {
        if oid.to_u32() < pg_sys::FirstNormalObjectId {
            return false;
        }
        let query_types = self.query_types.get_or_insert_with(|| {
            let tuple = pg_sys::SearchSysCache2(
                pg_sys::SysCacheIdentifier::TYPENAMENSP as _,
                pg_sys::Datum::from(c"searchqueryinput".as_ptr()),
                pg_sys::Datum::from(self.namespace.to_u32()),
            );
            if tuple.is_null() {
                return [pg_sys::InvalidOid; 2];
            }
            let form = pgrx::GETSTRUCT!(tuple)
                .get()
                .cast::<pg_sys::FormData_pg_type>();
            let types = [(*form).oid, (*form).typarray];
            pg_sys::ReleaseSysCache(tuple);
            types
        });
        query_types.contains(&oid)
    }

    unsafe fn input_type(&mut self, oid: pg_sys::Oid) {
        if self.is_legacy_query_type(oid) {
            self.found
                .insert(LEGACY_QUERY_TYPE.into(), LEGACY_QUERY_HINT);
        }
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn walk_query(node: *mut pg_sys::Node, context: *mut c_void) -> bool {
    if node.is_null() {
        return false;
    }
    let calls = &mut *context.cast::<QueryCalls>();
    match (*node).type_ {
        pg_sys::NodeTag::T_Query => {
            return pg_sys::query_tree_walker(node.cast(), Some(walk_query), context, 0);
        }
        pg_sys::NodeTag::T_FuncExpr => {
            let function = &*node.cast::<pg_sys::FuncExpr>();
            calls.function(function.funcid);
            let args = PgList::<pg_sys::Node>::from_pg(function.args);
            if let Some(query_arg) = args.get_ptr(1)
                && let Some(query) = nodecast!(Const, query_arg)
                && calls.is_legacy_query_type((*query).consttype)
                && is_search_with_query_input_exec_procoid(function.funcid)
            {
                // Skip the planner's query literal when replaying a scan filter.
                pg_sys::check_stack_depth();
                return args
                    .iter_ptr()
                    .enumerate()
                    .any(|(position, arg)| position != 1 && walk_query(arg, context));
            }
        }
        // Parsed V1 literals can be constants. V2 builders can return the same type.
        pg_sys::NodeTag::T_Const => calls.input_type((*node.cast::<pg_sys::Const>()).consttype),
        pg_sys::NodeTag::T_Param => {
            let param = &*node.cast::<pg_sys::Param>();
            if param.paramkind == pg_sys::ParamKind::PARAM_EXTERN {
                calls.input_type(param.paramtype);
            }
        }
        pg_sys::NodeTag::T_CoerceViaIO => {
            calls.input_type((*node.cast::<pg_sys::CoerceViaIO>()).resulttype)
        }
        _ => {}
    }
    pg_sys::expression_tree_walker(node, Some(walk_query), context)
}

/// Check V1 usage before the planner adds internal calls.
pub(crate) unsafe fn warn_query(query: *mut pg_sys::Query, extension: pg_sys::Oid) {
    let mode = v1api_warnings();
    if mode == PlannerWarnings::Off {
        return;
    }
    if pg_sys::creating_extension && pg_sys::CurrentExtensionObject == extension {
        return;
    }
    let Some(namespace) = lookup_namespace(c"paradedb") else {
        return;
    };
    let mut calls = QueryCalls {
        extension,
        namespace,
        query_types: None,
        seen_functions: HashSet::new(),
        found: BTreeMap::new(),
    };
    walk_query(query.cast(), std::ptr::from_mut(&mut calls).cast());
    for (name, hint) in calls.found {
        report(mode, &name, hint);
    }
}

unsafe fn legacy_options(options: *mut pg_sys::List) -> Vec<(&'static str, MigrationHint)> {
    PgList::<pg_sys::DefElem>::from_pg(options)
        .iter_ptr()
        .filter_map(|option| {
            if !(*option).defnamespace.is_null() {
                return None;
            }
            Some(match CStr::from_ptr((*option).defname).to_bytes() {
                b"key_field" => (
                    "key_field",
                    Replacement("Remove key_field and put the indexed key column first in the CREATE INDEX column list."),
                ),
                b"text_fields" => (
                    "text_fields",
                    Gap("Configure text columns using tokenizer casts, such as field::pdb.simple. Use alias and columnar for aliases and columnar storage. Custom stopword lists and arbitrary record modes have no direct V2 equivalent."),
                ),
                b"json_fields" => (
                    "json_fields",
                    Gap("Index the JSON column directly or use a tokenizer cast. V2 has no setting for expand_dots=false."),
                ),
                b"numeric_fields" => ("numeric_fields", SCALAR_OPTIONS_HINT),
                b"boolean_fields" => ("boolean_fields", SCALAR_OPTIONS_HINT),
                b"range_fields" => ("range_fields", SCALAR_OPTIONS_HINT),
                b"datetime_fields" => (
                    "datetime_fields",
                    Replacement("Remove datetime_fields and list timestamp and date columns directly in CREATE INDEX."),
                ),
                _ => return None,
            })
        })
        .collect()
}

const SCALAR_OPTIONS_HINT: MigrationHint = Gap(
    "List the columns directly in CREATE INDEX. ParadeDB infers their types and applies the default index settings. V2 does not provide all legacy per-field storage settings.",
);

unsafe fn is_paradedb_am(am: pg_sys::Oid) -> bool {
    if am == pg_sys::InvalidOid {
        return false;
    }
    let extension = pg_sys::getExtensionOfObject(pg_sys::AccessMethodRelationId, am);
    extension != pg_sys::InvalidOid
        && extension == pg_sys::get_extension_oid(c"pg_search".as_ptr(), true)
}

unsafe fn alter_index_am(relation: *mut pg_sys::RangeVar) -> pg_sys::Oid {
    let relid = pg_sys::RangeVarGetRelidExtended(
        relation,
        pg_sys::NoLock as _,
        pg_sys::RVROption::RVR_MISSING_OK,
        None,
        std::ptr::null_mut(),
    );
    let tuple = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier::RELOID as _,
        pg_sys::Datum::from(relid.to_u32()),
    );
    if tuple.is_null() {
        return pg_sys::InvalidOid;
    }
    let form = pgrx::GETSTRUCT!(tuple)
        .get()
        .cast::<pg_sys::FormData_pg_class>();
    let am = (*form).relam;
    pg_sys::ReleaseSysCache(tuple);
    am
}

unsafe fn warn_index_utility(node: *mut pg_sys::Node) {
    let mode = v1api_warnings();
    if mode == PlannerWarnings::Off || node.is_null() || pg_sys::creating_extension {
        return;
    }

    if let Some(index) = nodecast!(IndexStmt, node) {
        let index = &*index;
        // Copying an index can generate a transformed statement.
        if index.transformed || index.accessMethod.is_null() {
            return;
        }
        let am = CStr::from_ptr(index.accessMethod);
        if !matches!(am.to_bytes(), b"bm25" | b"paradedb") {
            return;
        }
        if !is_paradedb_am(pg_sys::get_index_am_oid(am.as_ptr(), true)) {
            return;
        }
        if am == c"bm25" {
            report(mode, "USING bm25", Replacement("Use USING paradedb."));
        }
        for (name, hint) in legacy_options(index.options) {
            report(mode, name, hint);
        }
    } else if let Some(alter) = nodecast!(AlterTableStmt, node)
        && (*alter).objtype == pg_sys::ObjectType::OBJECT_INDEX
    {
        let options = PgList::<pg_sys::AlterTableCmd>::from_pg((*alter).cmds)
            .iter_ptr()
            .filter(|cmd| (**cmd).subtype == pg_sys::AlterTableType::AT_SetRelOptions)
            .flat_map(|cmd| legacy_options((*cmd).def.cast()))
            .collect::<Vec<_>>();
        if !options.is_empty() && is_paradedb_am(alter_index_am((*alter).relation)) {
            for (name, hint) in options {
                report(mode, name, hint);
            }
        }
    }
}

static mut PREV_PROCESS_UTILITY_HOOK: pg_sys::ProcessUtility_hook_type = None;

#[pg_guard]
#[allow(clippy::too_many_arguments)]
unsafe extern "C-unwind" fn process_utility(
    plan: *mut pg_sys::PlannedStmt,
    query_string: *const std::ffi::c_char,
    read_only_tree: bool,
    context: pg_sys::ProcessUtilityContext::Type,
    params: pg_sys::ParamListInfo,
    query_env: *mut pg_sys::QueryEnvironment,
    dest: *mut pg_sys::DestReceiver,
    qc: *mut pg_sys::QueryCompletion,
) {
    warn_index_utility((*plan).utilityStmt);
    if let Some(previous) = PREV_PROCESS_UTILITY_HOOK {
        previous(
            plan,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            qc,
        );
    } else {
        pg_sys::standard_ProcessUtility(
            plan,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            qc,
        );
    }
}

pub(crate) unsafe fn register_utility_hook() {
    PREV_PROCESS_UTILITY_HOOK = pg_sys::ProcessUtility_hook;
    pg_sys::ProcessUtility_hook = Some(process_utility);
}
