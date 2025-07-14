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

use crate::api::operator::{anyelement_query_input_opoid, anyelement_text_opoid};
use crate::api::Cardinality;
use crate::api::FieldName;
use crate::api::HashSet;
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::CustomScan;
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
#[repr(i32)]
pub enum SortDirection {
    #[default]
    Asc = pg_sys::BTLessStrategyNumber as i32,
    Desc = pg_sys::BTGreaterStrategyNumber as i32,
    None = pg_sys::BTEqualStrategyNumber as i32,
}

impl AsRef<str> for SortDirection {
    fn as_ref(&self) -> &str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
            SortDirection::None => "<none>",
        }
    }
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl From<i32> for SortDirection {
    fn from(value: i32) -> Self {
        SortDirection::from(value as u32)
    }
}

impl From<u32> for SortDirection {
    fn from(value: u32) -> Self {
        match value {
            pg_sys::BTLessStrategyNumber => SortDirection::Asc,
            pg_sys::BTGreaterStrategyNumber => SortDirection::Desc,
            _ => panic!("unrecognized sort strategy number: {value}"),
        }
    }
}

impl From<SortDirection> for crate::index::reader::index::SortDirection {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Asc => crate::index::reader::index::SortDirection::Asc,
            SortDirection::Desc => crate::index::reader::index::SortDirection::Desc,
            SortDirection::None => crate::index::reader::index::SortDirection::None,
        }
    }
}

impl From<SortDirection> for u32 {
    fn from(value: SortDirection) -> Self {
        value as _
    }
}

#[derive(Debug)]
pub enum OrderByStyle {
    Score(*mut pg_sys::PathKey),
    Field(*mut pg_sys::PathKey, FieldName),
}

impl OrderByStyle {
    pub fn pathkey(&self) -> *mut pg_sys::PathKey {
        match self {
            OrderByStyle::Score(pathkey) => *pathkey,
            OrderByStyle::Field(pathkey, _) => *pathkey,
        }
    }

    pub fn direction(&self) -> SortDirection {
        unsafe {
            let pathkey = self.pathkey();
            assert!(!pathkey.is_null());

            (*self.pathkey()).pk_strategy.into()
        }
    }
}

///
/// The type of ExecMethod that was chosen at planning time. We fully select an ExecMethodType at
/// planning time in order to be able to make claims about the sortedness and estimates for our
/// execution.
///
/// `which_fast_fields` lists in this enum are _all_ of the fast fields which were identified at
/// planning time: based on the join order that the planner ends up choosing, only a subset of
/// these might be used at execution time (in an order specified by the execution time target
/// list), but never a superset.
///
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum ExecMethodType {
    #[default]
    Normal,
    TopN {
        heaprelid: pg_sys::Oid,
        limit: usize,
        sort_direction: SortDirection,
    },
    FastFieldString {
        field: String,
        which_fast_fields: HashSet<WhichFastField>,
    },
    FastFieldNumeric {
        which_fast_fields: HashSet<WhichFastField>,
    },
    FastFieldMixed {
        which_fast_fields: HashSet<WhichFastField>,
    },
}

impl ExecMethodType {
    ///
    /// Returns true if this execution method will emit results in sorted order with the given
    /// number of workers.
    ///
    pub fn is_sorted(&self) -> bool {
        match self {
            ExecMethodType::TopN { .. } => true,
            // See https://github.com/paradedb/paradedb/issues/2623 about enabling sorted orders for
            // String and Mixed.
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct Args {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub rti: pg_sys::Index,
    pub rte: *mut pg_sys::RangeTblEntry,
}

impl Args {
    #[allow(dead_code)]
    pub fn root(&self) -> &pg_sys::PlannerInfo {
        unsafe { self.root.as_ref().expect("Args::root should not be null") }
    }

    pub fn rel(&self) -> &pg_sys::RelOptInfo {
        unsafe { self.rel.as_ref().expect("Args::rel should not be null") }
    }

    pub fn rte(&self) -> &pg_sys::RangeTblEntry {
        unsafe { self.rte.as_ref().expect("Args::rte should not be null") }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
#[repr(u32)]
#[allow(dead_code)]
pub enum Flags {
    /// #define CUSTOMPATH_SUPPORT_BACKWARD_SCAN	0x0001
    BackwardScan = 0x0001,

    /// #define CUSTOMPATH_SUPPORT_MARK_RESTORE		0x0002
    MarkRestore = 0x0002,

    /// #define CUSTOMPATH_SUPPORT_PROJECTION		0x0004
    Projection = 0x0004,

    /// ParadeDB custom flag for indicating we want to force the plan to be used
    Force = 0x0008,
}

pub struct CustomPathBuilder<P: Into<*mut pg_sys::List> + Default> {
    args: Args,
    flags: HashSet<Flags>,

    custom_path_node: pg_sys::CustomPath,

    custom_paths: PgList<pg_sys::Path>,

    /// `custom_private` can be used to store the custom path's private data. Private data should be
    /// stored in a form that can be handled by nodeToString, so that debugging routines that attempt
    /// to print the custom path will work as designed.
    custom_private: P,
}

#[derive(Copy, Clone, Debug)]
pub enum RestrictInfoType {
    BaseRelation,
    Join,
    None,
}

impl<P: Into<*mut pg_sys::List> + Default> CustomPathBuilder<P> {
    pub fn new<CS: CustomScan>(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        rti: pg_sys::Index,
        rte: *mut pg_sys::RangeTblEntry,
    ) -> CustomPathBuilder<P> {
        unsafe {
            Self {
                args: Args {
                    root,
                    rel,
                    rti,
                    rte,
                },
                flags: Default::default(),

                custom_path_node: pg_sys::CustomPath {
                    path: pg_sys::Path {
                        type_: pg_sys::NodeTag::T_CustomPath,
                        pathtype: pg_sys::NodeTag::T_CustomScan,
                        parent: rel,
                        pathtarget: (*rel).reltarget,
                        param_info: pg_sys::get_baserel_parampathinfo(
                            root,
                            rel,
                            pg_sys::bms_copy((*rel).lateral_relids),
                        ),
                        ..Default::default()
                    },
                    methods: CS::custom_path_methods(),
                    ..Default::default()
                },
                custom_paths: PgList::default(),
                custom_private: P::default(),
            }
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    //
    // convenience getters for type safety
    //

    pub fn restrict_info(&self) -> (PgList<pg_sys::RestrictInfo>, RestrictInfoType) {
        unsafe {
            let baseri = PgList::from_pg(self.args.rel().baserestrictinfo);
            let joinri = PgList::from_pg(self.args.rel().joininfo);

            if baseri.is_empty() && joinri.is_empty() {
                // both lists are empty, so return an empty list
                (PgList::new(), RestrictInfoType::None)
            } else if joinri.is_empty() {
                // joininfo is empty, so return baserestrictinfo
                (baseri, RestrictInfoType::BaseRelation)
            } else {
                // joininfo is not empty, so we need to filter it
                let (pushed_down_joinri, unpushed_joinri) =
                    Self::partition_restrict_info_by_pushdown(&joinri);

                // Check that none of the unpushed entries contain our @@@ operator
                if Self::contains_our_operator(&unpushed_joinri) {
                    pgrx::warning!(
                        ">>> 111 joininfo contains unpushed entries with @@@ operator: {:?}",
                        Self::restrict_info_to_string(&unpushed_joinri, self.args.root)
                    );
                    (PgList::new(), RestrictInfoType::None)
                } else {
                    // Use the pushed down entries from joininfo
                    if pushed_down_joinri.is_empty() {
                        pgrx::warning!(
                            ">>> 222 pushed_down_joinri is empty and unpushed_joinri does not contain @@@ operator: {:?}",
                            Self::restrict_info_to_string(&unpushed_joinri, self.args.root)
                        );
                        (PgList::new(), RestrictInfoType::None)
                    } else {
                        if baseri.is_empty() {
                            pgrx::warning!(
                                ">>> 333 baseri is empty and pushed_down_joinri is not empty: {:?}",
                                Self::restrict_info_to_string(&pushed_down_joinri, self.args.root)
                            );
                            (pushed_down_joinri, RestrictInfoType::Join)
                        } else {
                            pgrx::warning!(
                                ">>> 444 baseri is not empty: {:?} and pushed_down_joinri is not empty: {:?}",
                                Self::restrict_info_to_string(&baseri, self.args.root),
                                Self::restrict_info_to_string(&pushed_down_joinri, self.args.root)
                            );
                            (pushed_down_joinri, RestrictInfoType::Join)
                        }
                    }
                }
            }
        }
    }

    /// Partition a restrict info list into pushed down and unpushed entries
    fn partition_restrict_info_by_pushdown(
        restrict_info: &PgList<pg_sys::RestrictInfo>,
    ) -> (PgList<pg_sys::RestrictInfo>, PgList<pg_sys::RestrictInfo>) {
        unsafe {
            let mut pushed_down = PgList::new();
            let mut unpushed = PgList::new();

            for ri in restrict_info.iter_ptr() {
                if (*ri).is_pushed_down {
                    pushed_down.push(ri);
                } else {
                    unpushed.push(ri);
                }
            }

            (pushed_down, unpushed)
        }
    }

    /// Check if any RestrictInfo in the list contains our @@@ operator
    fn contains_our_operator(restrict_info: &PgList<pg_sys::RestrictInfo>) -> bool {
        unsafe {
            let text_opoid = anyelement_text_opoid();
            let query_input_opoid = anyelement_query_input_opoid();

            restrict_info.iter_ptr().any(|ri| {
                Self::clause_contains_operator((*ri).clause, text_opoid)
                    || Self::clause_contains_operator((*ri).clause, query_input_opoid)
            })
        }
    }

    /// Check if a clause (expression node) contains the specified operator
    fn clause_contains_operator(clause: *mut pg_sys::Expr, target_opoid: pg_sys::Oid) -> bool {
        unsafe {
            if clause.is_null() {
                return false;
            }

            let node = clause.cast::<pg_sys::Node>();
            match (*node).type_ {
                pg_sys::NodeTag::T_OpExpr => {
                    let opexpr = node.cast::<pg_sys::OpExpr>();
                    (*opexpr).opno == target_opoid
                }
                pg_sys::NodeTag::T_BoolExpr => {
                    let boolexpr = node.cast::<pg_sys::BoolExpr>();
                    let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
                    for arg in args.iter_ptr() {
                        if Self::clause_contains_operator(arg.cast(), target_opoid) {
                            return true;
                        }
                    }
                    false
                }
                pg_sys::NodeTag::T_FuncExpr => {
                    let funcexpr = node.cast::<pg_sys::FuncExpr>();
                    let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                    for arg in args.iter_ptr() {
                        if Self::clause_contains_operator(arg.cast(), target_opoid) {
                            return true;
                        }
                    }
                    false
                }
                _ => false,
            }
        }
    }

    fn restrict_info_to_string(
        restrict_info: &PgList<pg_sys::RestrictInfo>,
        root: *mut pg_sys::PlannerInfo,
    ) -> String {
        restrict_info
            .iter_ptr()
            .map(|ri| Self::ri_tostring(ri, root))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn ri_tostring(ri: *mut pg_sys::RestrictInfo, root: *mut pg_sys::PlannerInfo) -> String {
        unsafe {
            pgrx::warning!("ri_tostring: Starting");

            let ri = if let Some(ri) = ri.as_ref() {
                ri
            } else {
                pgrx::warning!("ri_tostring: RestrictInfo is null");
                return "<null restrictinfo>".to_string();
            };

            let clause_str = if !ri.clause.is_null() {
                pgrx::warning!("ri_tostring: Processing clause");

                // Use our custom expression deparsing instead of PostgreSQL's deparse_expression
                let expr_str = Self::custom_deparse_expression(ri.clause.cast(), root);
                format!("RestrictInfo {{ clause: {} }}", expr_str)
            } else {
                pgrx::warning!("ri_tostring: clause is null");
                "RestrictInfo { clause: <null clause> }".to_string()
            };

            pgrx::warning!("ri_tostring: Returning result");
            clause_str
        }
    }

    /// Custom expression deparsing that handles common PostgreSQL node types
    /// without requiring complex deparse context setup
    fn custom_deparse_expression(
        node: *mut pg_sys::Node,
        root: *mut pg_sys::PlannerInfo,
    ) -> String {
        unsafe {
            if node.is_null() {
                return "<null>".to_string();
            }

            match (*node).type_ {
                pg_sys::NodeTag::T_Var => {
                    let var = node.cast::<pg_sys::Var>();
                    Self::deparse_var(var, root)
                }
                pg_sys::NodeTag::T_Const => {
                    let const_node = node.cast::<pg_sys::Const>();
                    Self::deparse_const(const_node)
                }
                pg_sys::NodeTag::T_OpExpr => {
                    let opexpr = node.cast::<pg_sys::OpExpr>();
                    Self::deparse_opexpr(opexpr, root)
                }
                pg_sys::NodeTag::T_FuncExpr => {
                    let funcexpr = node.cast::<pg_sys::FuncExpr>();
                    Self::deparse_funcexpr(funcexpr, root)
                }
                pg_sys::NodeTag::T_BoolExpr => {
                    let boolexpr = node.cast::<pg_sys::BoolExpr>();
                    Self::deparse_boolexpr(boolexpr, root)
                }
                pg_sys::NodeTag::T_RelabelType => {
                    let relabel = node.cast::<pg_sys::RelabelType>();
                    // For RelabelType, just deparse the underlying argument
                    Self::custom_deparse_expression((*relabel).arg.cast(), root)
                }
                _ => {
                    // For unknown node types, fall back to a simple representation
                    format!("<node_type_{}>", (*node).type_ as i32)
                }
            }
        }
    }

    /// Deparse a Var node to a string representation - simplified version
    fn deparse_var(var: *mut pg_sys::Var, _root: *mut pg_sys::PlannerInfo) -> String {
        unsafe {
            let varno = (*var).varno;
            let varattno = (*var).varattno;

            // Simple representation without accessing system catalogs
            format!("table_{}.col_{}", varno, varattno)
        }
    }

    /// Deparse a Const node to a string representation
    fn deparse_const(const_node: *mut pg_sys::Const) -> String {
        unsafe {
            if (*const_node).constisnull {
                return "NULL".to_string();
            }

            match (*const_node).consttype {
                pg_sys::BOOLOID => {
                    let value = pg_sys::DatumGetBool((*const_node).constvalue);
                    if value { "true" } else { "false" }.to_string()
                }
                pg_sys::INT4OID => {
                    let value = pg_sys::DatumGetInt32((*const_node).constvalue);
                    value.to_string()
                }
                pg_sys::INT8OID => {
                    let value = pg_sys::DatumGetInt64((*const_node).constvalue);
                    value.to_string()
                }
                pg_sys::FLOAT4OID => {
                    let value = pg_sys::DatumGetFloat4((*const_node).constvalue);
                    value.to_string()
                }
                pg_sys::FLOAT8OID => {
                    let value = pg_sys::DatumGetFloat8((*const_node).constvalue);
                    value.to_string()
                }
                pg_sys::TEXTOID | pg_sys::VARCHAROID => {
                    // For text types, we need to extract the string value
                    let varlena =
                        pg_sys::DatumGetPointer((*const_node).constvalue) as *mut pg_sys::varlena;
                    if !varlena.is_null() {
                        let text_slice = pgrx::varlena_to_byte_slice(varlena);
                        match std::str::from_utf8(text_slice) {
                            Ok(s) => format!("'{}'", s.replace("'", "''")), // Escape single quotes
                            Err(_) => "<invalid utf8>".to_string(),
                        }
                    } else {
                        "<null text>".to_string()
                    }
                }
                _ => {
                    // For other types, just show the type OID
                    format!("<const_type_{}>", (*const_node).consttype)
                }
            }
        }
    }

    /// Deparse an OpExpr node to a string representation
    fn deparse_opexpr(opexpr: *mut pg_sys::OpExpr, root: *mut pg_sys::PlannerInfo) -> String {
        unsafe {
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
            let op_oid = (*opexpr).opno;

            match args.len() {
                1 => {
                    // Unary operator
                    let arg = Self::custom_deparse_expression(args.get_ptr(0).unwrap(), root);
                    format!("(op_{} {})", op_oid, arg)
                }
                2 => {
                    // Binary operator
                    let left = Self::custom_deparse_expression(args.get_ptr(0).unwrap(), root);
                    let right = Self::custom_deparse_expression(args.get_ptr(1).unwrap(), root);
                    format!("({} op_{} {})", left, op_oid, right)
                }
                _ => {
                    // Multiple arguments - this is unusual for OpExpr
                    let arg_strs: Vec<String> = args
                        .iter_ptr()
                        .map(|arg| Self::custom_deparse_expression(arg, root))
                        .collect();
                    format!("op_{}({})", op_oid, arg_strs.join(", "))
                }
            }
        }
    }

    /// Deparse a FuncExpr node to a string representation
    fn deparse_funcexpr(funcexpr: *mut pg_sys::FuncExpr, root: *mut pg_sys::PlannerInfo) -> String {
        unsafe {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            let func_oid = (*funcexpr).funcid;

            let arg_strs: Vec<String> = args
                .iter_ptr()
                .map(|arg| Self::custom_deparse_expression(arg, root))
                .collect();

            format!("func_{}({})", func_oid, arg_strs.join(", "))
        }
    }

    /// Deparse a BoolExpr node to a string representation
    fn deparse_boolexpr(boolexpr: *mut pg_sys::BoolExpr, root: *mut pg_sys::PlannerInfo) -> String {
        unsafe {
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let arg_strs: Vec<String> = args
                .iter_ptr()
                .map(|arg| Self::custom_deparse_expression(arg, root))
                .collect();

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => {
                    format!("({})", arg_strs.join(" AND "))
                }
                pg_sys::BoolExprType::OR_EXPR => {
                    format!("({})", arg_strs.join(" OR "))
                }
                pg_sys::BoolExprType::NOT_EXPR => {
                    if arg_strs.len() == 1 {
                        format!("(NOT {})", arg_strs[0])
                    } else {
                        format!("(NOT ({}))", arg_strs.join(", "))
                    }
                }
                _ => {
                    format!("UNKNOWN_BOOL_OP({})", arg_strs.join(", "))
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn path_target(&self) -> *mut pg_sys::PathTarget {
        self.args.rel().reltarget
    }

    #[allow(dead_code)]
    pub fn limit(&self) -> i32 {
        unsafe { (*self.args().root).limit_tuples.round() as i32 }
    }

    //
    // public settings
    //

    #[allow(dead_code)]
    pub fn clear_flags(mut self) -> Self {
        self.flags.clear();
        self
    }

    pub fn set_flag(mut self, flag: Flags) -> Self {
        self.flags.insert(flag);
        self
    }

    #[allow(dead_code)]
    pub fn add_custom_path(mut self, path: *mut pg_sys::Path) -> Self {
        self.custom_paths.push(path);
        self
    }

    pub fn custom_private(&mut self) -> &mut P {
        &mut self.custom_private
    }

    pub fn set_rows(mut self, rows: Cardinality) -> Self {
        self.custom_path_node.path.rows = rows;
        self
    }

    pub fn set_startup_cost(mut self, cost: pg_sys::Cost) -> Self {
        self.custom_path_node.path.startup_cost = cost;
        self
    }

    pub fn set_total_cost(mut self, cost: pg_sys::Cost) -> Self {
        self.custom_path_node.path.total_cost = cost;
        self
    }

    pub fn add_path_key(mut self, style: &OrderByStyle) -> Self {
        unsafe {
            let mut pklist =
                PgList::<pg_sys::PathKey>::from_pg(self.custom_path_node.path.pathkeys);
            pklist.push(style.pathkey());

            self.custom_path_node.path.pathkeys = pklist.into_pg();
        }
        self
    }

    pub fn set_force_path(mut self, force: bool) -> Self {
        if force {
            self.flags.insert(Flags::Force);
        } else {
            self.flags.remove(&Flags::Force);
        }
        self
    }

    pub fn set_parallel(mut self, nworkers: usize) -> Self {
        self.custom_path_node.path.parallel_aware = true;
        self.custom_path_node.path.parallel_safe = true;
        self.custom_path_node.path.parallel_workers =
            nworkers.try_into().expect("nworkers should be a valid i32");

        self
    }

    pub fn is_parallel(&self) -> bool {
        self.custom_path_node.path.parallel_workers > 0
    }

    pub fn build(mut self) -> pg_sys::CustomPath {
        self.custom_path_node.custom_paths = self.custom_paths.into_pg();
        self.custom_path_node.custom_private = self.custom_private.into();
        self.custom_path_node.flags = self
            .flags
            .into_iter()
            .fold(0, |acc, flag| acc | flag as u32);

        self.custom_path_node
    }
}
