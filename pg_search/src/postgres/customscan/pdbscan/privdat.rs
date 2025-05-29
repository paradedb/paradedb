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

use crate::api::{AsCStr, Cardinality, Varno};
use crate::api::{HashMap, HashSet};
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::pdbscan::join_qual_inspect::JoinSearchPredicates;
use crate::postgres::customscan::pdbscan::ExecMethodType;
use crate::query::SearchQueryInput;
use pgrx::pg_sys::AsPgCStr;
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};

/// Information about composite relations in a join
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JoinCompositeInfo {
    /// Which side has the composite relation
    pub composite_side: CompositeSide,
    /// Whether the base relation side has search predicates
    pub base_has_search: bool,
    /// Whether the composite side has any search predicates
    pub composite_has_search: bool,
}

/// Which side of the join has composite relations
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompositeSide {
    /// Outer side is composite, inner is base
    Outer,
    /// Inner side is composite, outer is base
    Inner,
    /// Neither side is composite (both are base relations)
    None,
}

/// Which side of a unilateral join has a child plan for table scanning
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UnilateralChildSide {
    /// Outer side has child plan (inner side has search)
    Outer,
    /// Inner side has child plan (outer side has search)
    Inner,
}

/// Private data for the custom scan
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateData {
    heaprelid: Option<pg_sys::Oid>,
    indexrelid: Option<pg_sys::Oid>,
    range_table_index: Option<pg_sys::Index>,
    query: Option<SearchQueryInput>,
    limit: Option<usize>,
    segment_count: usize,
    sort_field: Option<String>,
    sort_direction: Option<SortDirection>,
    exec_method_type: ExecMethodType,
    #[serde(with = "var_attname_lookup_serializer")]
    var_attname_lookup: Option<HashMap<(Varno, pg_sys::AttrNumber), String>>,
    target_list_len: Option<usize>,
    planned_which_fast_fields: Option<HashSet<WhichFastField>>,
    referenced_columns_count: usize,

    /// Join search predicates for custom join execution
    join_search_predicates: Option<JoinSearchPredicates>,

    /// Outer relation OIDs for join execution (can be multiple for composite relations)
    join_outer_relids: Vec<pg_sys::Oid>,

    /// Inner relation OIDs for join execution (can be multiple for composite relations)
    join_inner_relids: Vec<pg_sys::Oid>,

    /// Information about composite relations in the join
    join_composite_info: Option<JoinCompositeInfo>,

    /// Which side has a child plan for unilateral joins
    unilateral_child_plan_side: Option<UnilateralChildSide>,

    /// Expected join target list from PostgreSQL's joinrel->reltarget
    #[serde(skip)]
    expected_join_targetlist: Option<*mut pg_sys::List>,

    /// Serializable target list information for joins
    expected_join_target_count: Option<usize>,

    /// Serializable target expressions as strings for reconstruction
    expected_join_target_expressions: Option<Vec<String>>,
}

impl Default for PrivateData {
    fn default() -> Self {
        Self {
            heaprelid: None,
            indexrelid: None,
            range_table_index: None,
            query: None,
            limit: None,
            sort_field: None,
            sort_direction: None,
            segment_count: 0,
            exec_method_type: ExecMethodType::Normal,
            planned_which_fast_fields: None,
            referenced_columns_count: 0,
            target_list_len: None,
            var_attname_lookup: None,
            join_search_predicates: None,
            join_outer_relids: Vec::new(),
            join_inner_relids: Vec::new(),
            unilateral_child_plan_side: None,
            expected_join_targetlist: None,
            expected_join_target_count: None,
            expected_join_target_expressions: None,
            join_composite_info: None,
        }
    }
}

mod var_attname_lookup_serializer {
    use super::*;

    use serde::{de::Error, Deserializer, Serializer};

    fn key_to_string(key: &(Varno, pg_sys::AttrNumber)) -> String {
        format!("{},{}", key.0, key.1)
    }

    fn key_from_string(s: &str) -> Result<(Varno, pg_sys::AttrNumber), String> {
        let mut parts = s.splitn(2, ',');
        let p1_str = parts
            .next()
            .ok_or_else(|| "Missing first part of key".to_string())?;
        let p2_str = parts
            .next()
            .ok_or_else(|| "Missing second part of key".to_string())?;

        let p1 = p1_str
            .parse::<Varno>()
            .map_err(|e| format!("Failed to parse first key part '{}': {}", p1_str, e))?;
        let p2 = p2_str
            .parse::<pg_sys::AttrNumber>()
            .map_err(|e| format!("Failed to parse second key part '{}': {}", p2_str, e))?;

        Ok((p1, p2))
    }

    pub fn serialize<S>(
        map_option: &Option<HashMap<(Varno, pg_sys::AttrNumber), String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let Some(map) = map_option else {
            return serializer.serialize_none();
        };

        // Serialize as Vec<(String, String)>.
        map.iter()
            .map(|(k, v)| (key_to_string(k), v))
            .collect::<Vec<(String, &String)>>()
            .serialize(serializer)
    }

    #[allow(clippy::type_complexity)]
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<HashMap<(Varno, pg_sys::AttrNumber), String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize as Vec<(String, String)>.
        let Some(string_map) = Option::<Vec<(&'de str, String)>>::deserialize(deserializer)? else {
            return Ok(None);
        };

        let mut map = HashMap::default();
        map.reserve(string_map.len());

        for (k_str, v) in string_map {
            let key_tuple = key_from_string(k_str)
                .map_err(|e| D::Error::custom(format!("Invalid key format '{}': {}", k_str, e)))?;
            map.insert(key_tuple, v);
        }
        Ok(Some(map))
    }
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list.get_ptr(0).unwrap();
            let content = node
                .as_c_str()
                .unwrap()
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(content).unwrap()
        }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        let content = serde_json::to_string(&value).unwrap();
        unsafe {
            let mut ser = PgList::new();
            ser.push(pg_sys::makeString(content.as_pg_cstr()).cast::<pg_sys::Node>());
            ser.into_pg()
        }
    }
}

//
// setter functions
//

impl PrivateData {
    pub fn set_heaprelid(&mut self, oid: pg_sys::Oid) {
        self.heaprelid = Some(oid);
    }

    pub fn set_indexrelid(&mut self, oid: pg_sys::Oid) {
        self.indexrelid = Some(oid);
    }

    pub fn set_range_table_index(&mut self, rti: pg_sys::Index) {
        self.range_table_index = Some(rti);
    }

    pub fn set_query(&mut self, query: SearchQueryInput) {
        self.query = Some(query);
    }

    pub fn set_limit(&mut self, limit: Option<Cardinality>) {
        self.limit = limit.map(|l| l.round() as usize);
    }

    pub fn set_sort_direction(&mut self, sort_direction: Option<SortDirection>) {
        self.sort_direction = sort_direction;
    }

    pub fn set_sort_field(&mut self, sort_field: String) {
        self.sort_field = Some(sort_field);
    }

    pub fn set_var_attname_lookup(
        &mut self,
        var_attname_lookup: HashMap<(Varno, pg_sys::AttrNumber), String>,
    ) {
        self.var_attname_lookup = Some(var_attname_lookup);
    }

    pub fn set_segment_count(&mut self, segment_count: usize) {
        self.segment_count = segment_count;
    }

    pub fn set_planned_which_fast_fields(
        &mut self,
        planned_which_fast_fields: HashSet<WhichFastField>,
    ) {
        self.planned_which_fast_fields = Some(planned_which_fast_fields);
    }

    pub fn set_exec_method_type(&mut self, exec_method_type: ExecMethodType) {
        self.exec_method_type = exec_method_type;
    }

    pub fn set_target_list_len(&mut self, len: Option<usize>) {
        self.target_list_len = len;
    }

    pub fn set_referenced_columns_count(&mut self, count: usize) {
        self.referenced_columns_count = count;
    }

    pub fn set_join_search_predicates(
        &mut self,
        join_search_predicates: Option<JoinSearchPredicates>,
    ) {
        self.join_search_predicates = join_search_predicates;
    }

    pub fn set_join_outer_relids(&mut self, oids: Vec<pg_sys::Oid>) {
        self.join_outer_relids = oids;
    }

    pub fn set_join_inner_relids(&mut self, oids: Vec<pg_sys::Oid>) {
        self.join_inner_relids = oids;
    }

    pub fn set_join_composite_info(&mut self, info: Option<JoinCompositeInfo>) {
        self.join_composite_info = info;
    }

    pub fn set_unilateral_child_plan_side(&mut self, side: Option<UnilateralChildSide>) {
        self.unilateral_child_plan_side = side;
    }

    pub fn set_expected_join_targetlist(&mut self, targetlist: Option<*mut pg_sys::List>) {
        self.expected_join_targetlist = targetlist;
    }

    pub fn set_expected_join_target_count(&mut self, count: Option<usize>) {
        self.expected_join_target_count = count;
    }

    pub fn set_expected_join_target_expressions(&mut self, expressions: Option<Vec<String>>) {
        self.expected_join_target_expressions = expressions;
    }
}

//
// getter functions
//

impl PrivateData {
    pub fn heaprelid(&self) -> Option<pg_sys::Oid> {
        self.heaprelid
    }

    pub fn indexrelid(&self) -> Option<pg_sys::Oid> {
        self.indexrelid
    }

    pub fn range_table_index(&self) -> Option<pg_sys::Index> {
        self.range_table_index
    }

    pub fn query(&self) -> &Option<SearchQueryInput> {
        &self.query
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    pub fn sort_field(&self) -> Option<String> {
        self.sort_field.clone()
    }

    pub fn sort_direction(&self) -> Option<SortDirection> {
        self.sort_direction
    }

    pub fn is_sorted(&self) -> bool {
        matches!(
            self.sort_direction,
            Some(SortDirection::Asc | SortDirection::Desc)
        )
    }

    pub fn var_attname_lookup(&self) -> &Option<HashMap<(Varno, pg_sys::AttrNumber), String>> {
        &self.var_attname_lookup
    }

    pub fn maybe_ff(&self) -> bool {
        // If we have planned fast fields, then maybe we can use them!
        !self.planned_which_fast_fields.as_ref().unwrap().is_empty()
    }

    pub fn segment_count(&self) -> usize {
        self.segment_count
    }

    pub fn planned_which_fast_fields(&self) -> &Option<HashSet<WhichFastField>> {
        &self.planned_which_fast_fields
    }

    pub fn exec_method_type(&self) -> &ExecMethodType {
        &self.exec_method_type
    }

    pub fn referenced_columns_count(&self) -> usize {
        debug_assert!(self.referenced_columns_count >= self.target_list_len.unwrap_or(0));
        self.referenced_columns_count
    }

    pub fn join_search_predicates(&self) -> &Option<JoinSearchPredicates> {
        &self.join_search_predicates
    }

    pub fn need_scores(&self) -> bool {
        // For now, return false as a default
        // This can be enhanced later to check if scores are needed based on the query
        false
    }

    pub fn join_outer_relids(&self) -> &Vec<pg_sys::Oid> {
        &self.join_outer_relids
    }

    pub fn join_inner_relids(&self) -> &Vec<pg_sys::Oid> {
        &self.join_inner_relids
    }

    /// Get the primary outer relation OID (first one for backward compatibility)
    pub fn join_outer_relid(&self) -> Option<pg_sys::Oid> {
        self.join_outer_relids.first().copied()
    }

    /// Get the primary inner relation OID (first one for backward compatibility)
    pub fn join_inner_relid(&self) -> Option<pg_sys::Oid> {
        self.join_inner_relids.first().copied()
    }

    pub fn join_composite_info(&self) -> &Option<JoinCompositeInfo> {
        &self.join_composite_info
    }

    pub fn unilateral_child_plan_side(&self) -> &Option<UnilateralChildSide> {
        &self.unilateral_child_plan_side
    }

    pub fn expected_join_targetlist(&self) -> Option<*mut pg_sys::List> {
        self.expected_join_targetlist
    }

    pub fn expected_join_target_count(&self) -> Option<usize> {
        self.expected_join_target_count
    }

    pub fn expected_join_target_expressions(&self) -> &Option<Vec<String>> {
        &self.expected_join_target_expressions
    }
}
