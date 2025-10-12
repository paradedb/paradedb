use pgrx::pg_sys;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
}

pub(crate) struct GroupByClause {
    grouping_columns: Vec<GroupingColumn>,
}
