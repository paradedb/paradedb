use async_std::task;
use datafusion::arrow::datatypes::Schema;
use datafusion::common::arrow::datatypes::Field;
use datafusion::common::{DFSchema, DataFusionError};
use datafusion::datasource::provider_as_source;
use datafusion::logical_expr::{Expr, LogicalPlan, TableSource};
use datafusion::prelude::ParquetReadOptions;
use datafusion::sql::TableReference;
use pgrx::*;
use std::ffi::CString;
use std::sync::Arc;

use crate::tableam::utils::{get_parquet_directory, name_from_pg, CONTEXT};

pub trait DatafusionPlanTranslator {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String>;
}

pub trait DatafusionExprTranslator {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String>;
}

pub fn datafusion_err_to_string(msg: &'static str) -> impl Fn(DataFusionError) -> String {
    move |dfe: DataFusionError| -> String { format!("{}: {}", msg, dfe) }
}

pub fn datafusion_schema_from_table(
    table_source: Arc<dyn TableSource>,
) -> Result<DFSchema, String> {
    let datafusion_schema = DFSchema::try_from(Schema::new(
        table_source
            .schema()
            .fields()
            .iter()
            .map(|f| Field::new(f.name(), f.data_type().clone(), f.is_nullable()))
            .collect::<Vec<_>>(),
    ))
    .map_err(datafusion_err_to_string("Result DFSchema failed"))?;

    Ok(datafusion_schema)
}

pub unsafe fn datafusion_table_from_name(table_name: &str) -> Result<Arc<dyn TableSource>, String> {
    let table_reference = TableReference::from(table_name);

    if !CONTEXT.table_exist(table_reference.clone()).unwrap() {
        task::block_on(CONTEXT.register_parquet(
            table_name,
            get_parquet_directory(table_name).as_str(),
            ParquetReadOptions::default(),
        ))
        .expect("Could not register Parquet");
    }

    let table_provider =
        task::block_on(CONTEXT.table_provider(table_reference)).expect("Could not get table");

    Ok(provider_as_source(table_provider))
}

pub unsafe fn table_name_from_rte(rte: *mut pg_sys::RangeTblEntry) -> Result<String, String> {
    let relation = pg_sys::RelationIdGetRelation((*rte).relid);
    let pg_relation = PgRelation::from_pg_owned(relation);
    let name = name_from_pg(&pg_relation);
    Ok(name)
}

#[pg_guard]
pub unsafe fn using_columnar(ps: *mut pg_sys::PlannedStmt) -> bool {
    let rtable = (*ps).rtable;
    if rtable.is_null() {
        return false;
    }

    // Get mem table AM handler OID
    let handlername_cstr = CString::new("mem").unwrap();
    let handlername_ptr = handlername_cstr.as_ptr() as *mut i8;
    let memam_oid = pg_sys::get_am_oid(handlername_ptr, true);
    if memam_oid == pg_sys::InvalidOid {
        return false;
    }

    let amTup = pg_sys::SearchSysCache1(
        pg_sys::SysCacheIdentifier_AMOID.try_into().unwrap(),
        pg_sys::Datum::from(memam_oid),
    );
    let amForm = pg_sys::heap_tuple_get_struct::<pg_sys::FormData_pg_am>(amTup);
    let memhandler_oid = (*amForm).amhandler;
    pg_sys::ReleaseSysCache(amTup);

    let elements = (*rtable).elements;
    let mut using_noncol: bool = false;
    let mut using_col: bool = false;

    for i in 0..(*rtable).length {
        let rte = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::RangeTblEntry;
        if (*rte).rtekind != pgrx::pg_sys::RTEKind_RTE_RELATION {
            continue;
        }
        let relation = pg_sys::RelationIdGetRelation((*rte).relid);
        let pg_relation = PgRelation::from_pg_owned(relation);
        if !pg_relation.is_table() {
            continue;
        }

        let am_handler = (*relation).rd_amhandler;

        // If any table uses the Table AM handler, then return true.
        // TODO: if we support more operations, this will be more complex.
        //       for example, if to support joins, some of the nodes will use
        //       table AM for the nodes while others won't. In this case,
        //       we'll have to process in postgres plan for part of it and
        //       datafusion for the other part. For now, we'll simply
        //       fail if we encounter an unsupported node, so this won't happen.
        if am_handler == memhandler_oid {
            using_col = true;
        } else {
            using_noncol = true;
        }
    }

    if using_col && using_noncol {
        panic!("Mixing table types in a single query is not supported yet");
    }

    using_col
}
