// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::HashMap;
use crate::gucs;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::pdbscan::get_rel_name;
use crate::postgres::customscan::pdbscan::get_rel_name_from_rti_list;
use crate::postgres::customscan::CustomScan;
use once_cell::sync::Lazy;
use pgrx::{pg_guard, pg_sys, PgList, PgMemoryContexts};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::hash_map::Entry;

/// JOIN coordination private data for PostgreSQL List serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JoinCoordinationPrivateData {
    /// Mapping of table OIDs to their range table indexes
    relation_mapping: HashMap<pg_sys::Oid, pg_sys::Index>,
    /// List of table OIDs participating in the JOIN
    table_oids: Vec<pg_sys::Oid>,
    /// LIMIT value from the query
    limit: Option<usize>,
}

impl From<*mut pg_sys::List> for JoinCoordinationPrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list
                .get_ptr(0)
                .expect("private data list should not be empty");
            let json_str = std::ffi::CStr::from_ptr((*node.cast::<pg_sys::String>()).sval)
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(json_str)
                .expect("JOIN coordination private data should be valid JSON")
        }
    }
}

pub fn register_rel_pathlist<CS: CustomScan + 'static>(_: CS) {
    unsafe {
        static mut PREV_REL_HOOKS: Lazy<
            HashMap<std::any::TypeId, pg_sys::set_rel_pathlist_hook_type>,
        > = Lazy::new(Default::default);
        static mut PREV_JOIN_HOOKS: Lazy<
            HashMap<std::any::TypeId, pg_sys::set_join_pathlist_hook_type>,
        > = Lazy::new(Default::default);

        #[pg_guard]
        extern "C-unwind" fn __priv_rel_callback<CS: CustomScan + 'static>(
            root: *mut pg_sys::PlannerInfo,
            rel: *mut pg_sys::RelOptInfo,
            rti: pg_sys::Index,
            rte: *mut pg_sys::RangeTblEntry,
        ) {
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(Some(prev_hook)) = PREV_REL_HOOKS.get(&std::any::TypeId::of::<CS>()) {
                    (*prev_hook)(root, rel, rti, rte);
                }

                paradedb_rel_pathlist_callback::<CS>(root, rel, rti, rte);
            }
        }

        #[pg_guard]
        extern "C-unwind" fn __priv_join_callback<CS: CustomScan + 'static>(
            root: *mut pg_sys::PlannerInfo,
            joinrel: *mut pg_sys::RelOptInfo,
            outerrel: *mut pg_sys::RelOptInfo,
            innerrel: *mut pg_sys::RelOptInfo,
            jointype: pg_sys::JoinType::Type,
            extra: *mut pg_sys::JoinPathExtraData,
        ) {
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(Some(prev_hook)) = PREV_JOIN_HOOKS.get(&std::any::TypeId::of::<CS>()) {
                    (*prev_hook)(root, joinrel, outerrel, innerrel, jointype, extra);
                }

                paradedb_join_pathlist_callback::<CS>(
                    root, joinrel, outerrel, innerrel, jointype, extra,
                );
            }
        }

        // Register relation pathlist hook
        #[allow(static_mut_refs)]
        match PREV_REL_HOOKS.entry(std::any::TypeId::of::<CS>()) {
            Entry::Occupied(_) => panic!(
                "{} rel hook is already registered",
                std::any::type_name::<CS>()
            ),
            Entry::Vacant(entry) => entry.insert(pg_sys::set_rel_pathlist_hook),
        };

        // Register join pathlist hook
        #[allow(static_mut_refs)]
        match PREV_JOIN_HOOKS.entry(std::any::TypeId::of::<CS>()) {
            Entry::Occupied(_) => panic!(
                "{} join hook is already registered",
                std::any::type_name::<CS>()
            ),
            Entry::Vacant(entry) => entry.insert(pg_sys::set_join_pathlist_hook),
        };

        pg_sys::set_rel_pathlist_hook = Some(__priv_rel_callback::<CS>);
        pg_sys::set_join_pathlist_hook = Some(__priv_join_callback::<CS>);

        pg_sys::RegisterCustomScanMethods(CS::custom_scan_methods())
    }
}

/// Although this hook function can be used to examine, modify, or remove paths generated by the
/// core system, a custom scan provider will typically confine itself to generating CustomPath
/// objects and adding them to rel using add_path. The custom scan provider is responsible for
/// initializing the CustomPath object, which is declared like this:
#[pg_guard]
pub extern "C-unwind" fn paradedb_rel_pathlist_callback<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    rti: pg_sys::Index,
    rte: *mut pg_sys::RangeTblEntry,
) {
    unsafe {
        if !gucs::enable_custom_scan() {
            return;
        }

        if let Some(mut path) =
            CS::rel_pathlist_callback(CustomPathBuilder::new::<CS>(root, rel, rti, rte))
        {
            let forced = path.flags & Flags::Force as u32 != 0;
            path.flags ^= Flags::Force as u32; // make sure to clear this flag because it's special to us

            let mut custom_path = PgMemoryContexts::CurrentMemoryContext
                .copy_ptr_into(&mut path, std::mem::size_of_val(&path));

            if (*custom_path).path.parallel_aware {
                // add the partial path since the user-generated plan is parallel aware
                pg_sys::add_partial_path(rel, custom_path.cast());

                // remove all the existing possible paths
                (*rel).pathlist = std::ptr::null_mut();

                // then make another copy of it, increase its costs really, really high and
                // submit it as a regular path too, immediately after clearing out all the other
                // existing possible paths.
                //
                // We don't want postgres to choose this path, but we have to have at least one
                // non-partial path available for it to consider
                let copy = PgMemoryContexts::CurrentMemoryContext
                    .copy_ptr_into(&mut path, std::mem::size_of_val(&path));
                (*copy).path.parallel_aware = false;
                (*copy).path.total_cost = 1000000000.0;
                (*copy).path.startup_cost = 1000000000.0;

                // will be added down below
                custom_path = copy.cast();
            } else if forced {
                // remove all the existing possible paths
                (*rel).pathlist = std::ptr::null_mut();
            }

            // add this path for consideration
            pg_sys::add_path(rel, custom_path.cast());
        }
    }
}

/// JOIN pathlist callback for detecting multi-table search scenarios
/// This is called during JOIN planning where we have access to:
/// 1. Complete JOIN context (all tables involved)
/// 2. LIMIT information (available at JOIN planning level)
/// 3. JOIN conditions (passed as extra->restrictlist)
#[pg_guard]
pub extern "C-unwind" fn paradedb_join_pathlist_callback<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    joinrel: *mut pg_sys::RelOptInfo,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
    jointype: pg_sys::JoinType::Type,
    extra: *mut pg_sys::JoinPathExtraData,
) {
    unsafe {
        if !gucs::enable_custom_scan() {
            return;
        }

        // Check if JOIN coordination is enabled
        if !gucs::is_join_coordination_enabled() {
            return;
        }

        // Only handle INNER JOINs for now
        if jointype != pg_sys::JoinType::JOIN_INNER {
            return;
        }

        pgrx::warning!("=== JOIN PATHLIST CALLBACK ===");
        pgrx::warning!("  jointype: {:?}", jointype);
        pgrx::warning!("  limit_tuples: {}", (*root).limit_tuples);

        // Check if both outer and inner relations have search predicates
        let outer_has_search = relation_has_search_predicates(outerrel);
        let inner_has_search = relation_has_search_predicates(innerrel);

        pgrx::warning!("  outer_has_search: {}", outer_has_search);
        pgrx::warning!("  inner_has_search: {}", inner_has_search);

        // Only proceed if both relations have search predicates
        if !outer_has_search || !inner_has_search {
            pgrx::warning!("  SKIPPING: Not both relations have search predicates");
            return;
        }

        // Check if we have a LIMIT (now available at JOIN planning level!)
        let has_limit = (*root).limit_tuples > -1.0;
        pgrx::warning!("  has_limit: {}", has_limit);

        if !has_limit {
            pgrx::warning!("  SKIPPING: No LIMIT clause");
            return;
        }

        pgrx::warning!("  CREATING JOIN COORDINATION PATH!");

        // Create a custom JOIN path that uses our coordination logic
        let custom_path =
            create_join_coordination_path::<CS>(root, joinrel, outerrel, innerrel, extra);

        if let Some(path) = custom_path {
            pgrx::warning!("  SUCCESSFULLY CREATED JOIN COORDINATION PATH!");
            pg_sys::add_path(joinrel, path);
        } else {
            pgrx::warning!("  FAILED TO CREATE JOIN COORDINATION PATH");
        }
    }
}

/// Create a custom JOIN coordination path
unsafe fn create_join_coordination_path<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    joinrel: *mut pg_sys::RelOptInfo,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
    extra: *mut pg_sys::JoinPathExtraData,
) -> Option<*mut pg_sys::Path> {
    use pgrx::PgMemoryContexts;

    // Extract relation mapping information from both sides
    let mut relation_mapping = HashMap::default();
    let mut table_oids = Vec::new();

    // Extract all table OIDs and their RTIs from outer relation
    let outer_relations = extract_all_table_info_from_relids(root, (*outerrel).relids);
    for (oid, rti) in outer_relations {
        table_oids.push(oid);
        relation_mapping.insert(oid, rti);
        pgrx::warning!(
            "  Extracted outer table OID: {} (RTI: {})",
            get_rel_name(oid),
            rti
        );
    }

    // Extract all table OIDs and their RTIs from inner relation
    let inner_relations = extract_all_table_info_from_relids(root, (*innerrel).relids);
    for (oid, rti) in inner_relations {
        table_oids.push(oid);
        relation_mapping.insert(oid, rti);
        pgrx::warning!(
            "  Extracted inner table OID: {} (RTI: {})",
            get_rel_name(oid),
            rti
        );
    }

    // Only proceed if we have tables with search predicates
    if table_oids.is_empty() {
        pgrx::warning!("  FAILED: No tables found in JOIN relations");
        return None;
    }

    // Get LIMIT from root
    let limit = if (*root).limit_tuples > -1.0 {
        Some((*root).limit_tuples as usize)
    } else {
        None
    };

    // Create simplified JOIN coordination private data (no variable mapping)
    let join_private_data = JoinCoordinationPrivateData {
        relation_mapping,
        table_oids: table_oids.clone(),
        limit,
    };

    pgrx::warning!(
        "  JOIN private data: {} tables, limit: {:?}",
        table_oids.len(),
        limit
    );

    // Serialize the private data as a PostgreSQL List
    let private_data_json =
        serde_json::to_string(&join_private_data).expect("Failed to serialize JOIN private data");
    pgrx::warning!("  Serialized JSON: {}", private_data_json);

    let mut private_list = PgList::new();

    // Use PostgreSQL's memory allocation to ensure the string persists
    let json_len = private_data_json.len() + 1; // +1 for null terminator
    let json_ptr = unsafe {
        let ptr = PgMemoryContexts::CurrentMemoryContext.palloc(json_len) as *mut u8;
        std::ptr::copy_nonoverlapping(private_data_json.as_ptr(), ptr, private_data_json.len());
        ptr.add(private_data_json.len()).write(0); // null terminator
        ptr as *mut i8
    };
    pgrx::warning!("  Created persistent string at: {:p}", json_ptr);

    let string_node = pg_sys::makeString(json_ptr);
    pgrx::warning!("  Created string node: {:p}", string_node);

    private_list.push(string_node.cast::<pg_sys::Node>());
    pgrx::warning!("  Added node to list, list length: {}", private_list.len());

    // Calculate costs for our custom JOIN path
    // For now, use aggressive costing to ensure our path is selected
    let startup_cost = 1.0; // Very low startup cost
    let total_cost = 10.0; // Very low total cost to beat hash join

    // Estimate rows - for now, use a conservative estimate
    let rows = ((*outerrel).rows * (*innerrel).rows * 0.01).max(1.0); // 1% selectivity estimate

    pgrx::warning!(
        "  Creating custom path: startup_cost={}, total_cost={}, rows={}",
        startup_cost,
        total_cost,
        rows
    );

    // Create a CustomPath structure
    let custom_path_size = std::mem::size_of::<pg_sys::CustomPath>();
    let custom_path =
        PgMemoryContexts::CurrentMemoryContext.palloc0(custom_path_size) as *mut pg_sys::CustomPath;

    if custom_path.is_null() {
        pgrx::warning!("  FAILED: Could not allocate CustomPath");
        return None;
    }

    // Initialize the Path portion
    (*custom_path).path.type_ = pg_sys::NodeTag::T_CustomPath;
    (*custom_path).path.pathtype = pg_sys::NodeTag::T_CustomScan;
    (*custom_path).path.parent = joinrel;
    // CRITICAL: For join nodes, use the joinrel's reltarget which already has the correct columns
    (*custom_path).path.pathtarget = (*joinrel).reltarget;
    (*custom_path).path.param_info = std::ptr::null_mut();
    (*custom_path).path.parallel_aware = false;
    (*custom_path).path.parallel_safe = true;
    (*custom_path).path.parallel_workers = 0;
    (*custom_path).path.rows = rows;
    (*custom_path).path.startup_cost = startup_cost;
    (*custom_path).path.total_cost = total_cost;
    (*custom_path).path.pathkeys = std::ptr::null_mut(); // No ordering guaranteed

    // Initialize CustomPath-specific fields
    (*custom_path).flags = 0; // No special flags for now
    (*custom_path).custom_paths = std::ptr::null_mut(); // No child paths
    (*custom_path).custom_restrictinfo = (*extra).restrictlist; // Store JOIN conditions
    (*custom_path).custom_private = private_list.into_pg(); // Store our JOIN private data
    (*custom_path).methods = join_coordination_custom_path_methods::<CS>(); // Use JOIN-specific methods

    pgrx::warning!("  CustomPath created successfully with joinrel.reltarget");
    Some(custom_path as *mut pg_sys::Path)
}

/// Extract all table OIDs and their RTIs from a relids Bitmapset
/// This handles composite relations that may contain multiple tables from previous JOINs
unsafe fn extract_all_table_info_from_relids(
    root: *mut pg_sys::PlannerInfo,
    relids: *mut pg_sys::Bitmapset,
) -> Vec<(pg_sys::Oid, pg_sys::Index)> {
    let mut table_info = Vec::new();

    if relids.is_null() {
        return table_info;
    }

    // Iterate through all members of the bitmapset
    let mut rti = -1;
    loop {
        rti = pg_sys::bms_next_member(relids, rti);
        if rti < 0 {
            break;
        }

        let rti_index = rti as pg_sys::Index;

        // Get the table OID for this RTI
        if let Some(table_oid) = get_table_oid_for_rti(root, rti_index) {
            table_info.push((table_oid, rti_index));
            pgrx::warning!(
                "    Found table OID {} for RTI {}",
                get_rel_name(table_oid),
                rti_index
            );
        } else {
            pgrx::warning!("    Could not find table OID for RTI {}", rti_index);
        }
    }

    table_info
}

/// Get the table OID for a given range table index
/// This properly maps RTI to the actual table OID by looking up the range table entry
unsafe fn get_table_oid_for_rti(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> Option<pg_sys::Oid> {
    // Validate RTI bounds
    if rti == 0 {
        return None;
    }

    // Get the range table
    let rtable = (*(*root).parse).rtable;
    let rtable_list = pgrx::PgList::<pg_sys::RangeTblEntry>::from_pg(rtable);

    // RTI is 1-based, so convert to 0-based index
    let rte_index = (rti - 1) as usize;

    if rte_index >= rtable_list.len() {
        pgrx::warning!(
            "    RTI {} is out of bounds (rtable length: {})",
            rti,
            rtable_list.len()
        );
        return None;
    }

    // Get the RTE
    if let Some(rte) = rtable_list.get_ptr(rte_index) {
        // Only handle relation RTEs
        if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
            let table_oid = (*rte).relid;
            pgrx::warning!(
                "    RTI {} maps to table OID {}",
                rti,
                get_rel_name(table_oid)
            );
            return Some(table_oid);
        } else {
            pgrx::warning!(
                "    RTI {} is not a relation (rtekind: {:?})",
                rti,
                (*rte).rtekind
            );
        }
    } else {
        pgrx::warning!("    Could not get RTE for RTI {}", rti);
    }

    None
}

/// Get the table OID for a given varno (by looking up the corresponding RTI)
unsafe fn get_table_oid_for_varno(
    root: *mut pg_sys::PlannerInfo,
    varno: pg_sys::Index,
) -> Option<pg_sys::Oid> {
    // varno corresponds to RTI in the range table
    get_table_oid_for_rti(root, varno)
}

/// JOIN-specific custom path methods
/// These handle the planning of custom JOIN paths differently from regular table scans
unsafe fn join_coordination_custom_path_methods<CS: CustomScan>() -> *const pg_sys::CustomPathMethods
{
    static mut METHODS: *mut pg_sys::CustomPathMethods = std::ptr::null_mut();

    if METHODS.is_null() {
        METHODS =
            PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(pg_sys::CustomPathMethods {
                CustomName: CS::NAME.as_ptr(),
                PlanCustomPath: Some(plan_join_coordination_custom_path::<CS>),
                ReparameterizeCustomPathByChild: None, // Not needed for JOIN coordination
            });
    }
    METHODS
}

/// Plan a custom JOIN coordination path
/// This converts our custom JOIN path into an executable CustomScan plan
#[pg_guard]
extern "C-unwind" fn plan_join_coordination_custom_path<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    best_path: *mut pg_sys::CustomPath,
    tlist: *mut pg_sys::List,
    clauses: *mut pg_sys::List,
    custom_plans: *mut pg_sys::List,
) -> *mut pg_sys::Plan {
    unsafe {
        pgrx::warning!("=== PLANNING JOIN COORDINATION CUSTOM PATH ===");
        pgrx::warning!(
            "  rel relids: {:?}",
            get_rel_name_from_rti_list((*rel).relids, root)
        );
        pgrx::warning!(
            "  tlist length: {}",
            pgrx::PgList::<pg_sys::TargetEntry>::from_pg(tlist).len()
        );

        // Parse the existing private data to get our table mapping
        let existing_private_data = JoinCoordinationPrivateData::from((*best_path).custom_private);
        pgrx::warning!(
            "  Private data: {} tables, limit: {:?}",
            existing_private_data.table_oids.len(),
            existing_private_data.limit
        );

        // For now, we won't create child plans - let PostgreSQL handle variable resolution
        // through its normal mechanisms. The key is that our CustomScan execution must
        // properly produce all the required columns.

        // CRITICAL: Use joinrel's target instead of creating our own
        // The joinrel already has the correct reltarget that includes columns from both relations
        let join_target_list = (*(*rel).reltarget).exprs;

        pgrx::warning!(
            "  Using joinrel target list with {} expressions",
            pgrx::PgList::<pg_sys::Expr>::from_pg(join_target_list).len()
        );

        // Create a CustomScan plan for JOIN coordination
        let mut planner_cxt = PgMemoryContexts::CurrentMemoryContext;

        let custom_scan = pg_sys::CustomScan {
            flags: (*best_path).flags,
            custom_private: (*best_path).custom_private, // Use existing private data as-is
            custom_plans: custom_plans, // Use the custom_plans passed in (may be null)
            methods: CS::custom_scan_methods(),
            scan: pg_sys::Scan {
                plan: pg_sys::Plan {
                    type_: pg_sys::NodeTag::T_CustomScan,
                    targetlist: tlist, // Use the target list provided by PostgreSQL
                    startup_cost: (*best_path).path.startup_cost,
                    total_cost: (*best_path).path.total_cost,
                    plan_rows: (*best_path).path.rows,
                    parallel_aware: (*best_path).path.parallel_aware,
                    parallel_safe: (*best_path).path.parallel_safe,
                    ..Default::default()
                },
                scanrelid: 0, // JOIN scans don't have a single scanrelid
            },
            ..Default::default()
        };

        pgrx::warning!("  Created CustomScan plan for JOIN coordination");
        pgrx::warning!(
            "  startup_cost: {}, total_cost: {}, rows: {}",
            custom_scan.scan.plan.startup_cost,
            custom_scan.scan.plan.total_cost,
            custom_scan.scan.plan.plan_rows
        );

        planner_cxt.leak_and_drop_on_delete(custom_scan).cast()
    }
}

/// Check if a relation has search predicates (@@@ operators)
unsafe fn relation_has_search_predicates(rel: *mut pg_sys::RelOptInfo) -> bool {
    use pgrx::PgList;

    // Check baserestrictinfo for search predicates
    let restrict_info_list = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);

    for restrict_info in restrict_info_list.iter_ptr() {
        if contains_search_operator_in_clause((*restrict_info).clause.cast()) {
            return true;
        }
    }

    false
}

/// Check if a clause contains our search operator (@@@)
unsafe fn contains_search_operator_in_clause(clause: *mut pg_sys::Node) -> bool {
    use crate::api::operator::anyelement_query_input_opoid;

    if clause.is_null() {
        return false;
    }

    match (*clause).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let opexpr = clause as *mut pg_sys::OpExpr;
            (*opexpr).opno == anyelement_query_input_opoid()
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = clause as *mut pg_sys::BoolExpr;
            let args = pgrx::PgList::<pg_sys::Node>::from_pg((*boolexpr).args);

            for arg in args.iter_ptr() {
                if contains_search_operator_in_clause(arg) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}
