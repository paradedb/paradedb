use datafusion::arrow::datatypes::Schema;
use pg_sys::{
    exprType, get_attname, namestrcpy, pgrx_list_nth, Aggref, BuiltinOid, Const, Datum,
    FormData_pg_attribute, FormData_pg_operator, List, ModifyTable, NameData, Node, NodeTag, Oid,
    OpExpr, Plan, PlannedStmt, RangeTblEntry, RelationClose, RelationData, RelationIdGetRelation,
    SearchSysCache1, SeqScan, SysCacheIdentifier_OPEROID, ValuesScan, Var, GETSTRUCT,
};
use pgrx::nodes::is_a;
use pgrx::pg_sys::rt_fetch;
use pgrx::prelude::*;
use pgrx::PgRelation;

use std::ffi::CStr;
use std::sync::Arc;

use async_std::task;

use crate::col_datafusion::CONTEXT;

use datafusion::common::arrow::datatypes::{DataType, Field};
use datafusion::common::{DFSchema, DataFusionError, ScalarValue};
use datafusion::datasource::{provider_as_source, DefaultTableSource};
use datafusion::logical_expr::expr::AggregateFunction;
use datafusion::logical_expr::{
    Aggregate, AggregateFunction as BuiltInAgg, DmlStatement, Expr, Limit, LogicalPlan, TableScan,
    TableSource, Values,
};
use datafusion::sql::TableReference;

fn datafusion_err_to_string(msg: &'static str) -> impl Fn(DataFusionError) -> String {
    return move |dfe: DataFusionError| -> String { format!("{}: {}", msg, dfe.to_string()) };
}

unsafe fn get_attr(table: *mut RelationData, index: isize) -> *const FormData_pg_attribute {
    let tupdesc = (*table).rd_att;
    let natts = (*tupdesc).natts;
    if natts > 0 && (index as i32) <= natts {
        return (*tupdesc).attrs.as_ptr().offset(index - 1);
    } else {
        return std::ptr::null();
    }
}

pub fn postgres_to_datafusion_type(p_type: PgBuiltInOids) -> Result<DataType, String> {
    // Map each PgBuiltInOids (the Postgres types) to a Datafusion type.
    // TODO: Are we covering all OIDs?
    // You can see the full list of OIDs here https://docs.rs/pgrx/latest/pgrx/pg_sys/type.PgBuiltInOids.html
    return match p_type {
        PgBuiltInOids::BOOLOID => Ok(DataType::Boolean),
        PgBuiltInOids::INT4OID => Ok(DataType::Int32),
        PgBuiltInOids::INT8OID => Ok(DataType::Int64),
        PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::BPCHAROID => {
            Ok(DataType::Utf8)
        }
        // TODO: Add missing types
        PgBuiltInOids::INT2OID => Ok(DataType::Int16),
        // TODO: user-specified precision
        PgBuiltInOids::NUMERICOID => Ok(DataType::Decimal128(5, 2)),
        PgBuiltInOids::FLOAT4OID => Ok(DataType::Float32),
        PgBuiltInOids::FLOAT8OID => Ok(DataType::Float64),
        // TODO: Utf8 is variable length
        PgBuiltInOids::CHAROID => Ok(DataType::Utf8),
        // TODO: Implement the rest of the types
        _ => Err(format!(
            "OID {:?} isn't convertable to datafusion type yet",
            p_type
        )),
    };
}

// Call this function on the root plan node
pub unsafe fn transform_pg_plan_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
) -> Result<LogicalPlan, String> {
    let node = plan as *mut Node;
    let node_tag = (*node).type_;
    info!("parent: {:?}", node_tag);

    // lefttree is the outer plan - this is what is fed INTO the current plan level
    // TODO: righttree is the inner plan - this is only ever set for JOIN operations, so we'll ignore it for now
    // more info: https://www.pgmustard.com/blog/2019/9/17/postgres-execution-plans-field-glossary
    let mut outer_plan = None;
    let lefttree = (*plan).lefttree;
    if !lefttree.is_null() {
        outer_plan = Some(transform_pg_plan_to_df_plan(lefttree, rtable)?);
    }

    info!("{:?}", node_tag);
    match node_tag {
        NodeTag::T_SeqScan => transform_seqscan_to_df_plan(plan, rtable, outer_plan),
        NodeTag::T_ModifyTable => transform_modify_to_df_plan(plan, rtable, outer_plan),
        NodeTag::T_ValuesScan => transform_valuesscan_to_df_plan(plan, rtable, outer_plan),
        NodeTag::T_Result => transform_result_to_df_plan(plan, rtable, outer_plan),
        NodeTag::T_Sort => todo!(),
        NodeTag::T_Group => todo!(),
        NodeTag::T_Agg => transform_agg_to_df_plan(plan, rtable, outer_plan),
        NodeTag::T_Limit => transform_limit_to_df_plan(plan, rtable, outer_plan),
        NodeTag::T_Invalid => todo!(),
        _ => Err(format!("node type {:?} not supported yet", node_tag)),
    }
}

// Transform helpers

pub unsafe fn transform_const_to_df_expr(node: *mut Node) -> Result<Expr, String> {
    let constant = node as *mut pgrx::pg_sys::Const;

    let const_type = (*constant).consttype; // Oid
    let const_datum = (*constant).constvalue; // Datum
    let const_isnull = (*constant).constisnull; // bool

    // TODO: actually get the type here - for now just defaulting to Int32
    Ok(Expr::Literal(ScalarValue::Int32(Some(
        const_datum.value() as i32,
    ))))
}

pub unsafe fn transform_targetentry_to_df_field(node: *mut Node) -> Result<Field, String> {
    let target_entry = node as *mut pgrx::pg_sys::TargetEntry;

    let col_name = (*target_entry).resname;
    // If the column is an aggregate function it won't have a name, so we
    // give it a hard-coded random name
    // TODO: Is this the right thing to do?
    let col_name_str = if col_name.is_null() {
        CStr::from_bytes_with_nul_unchecked(b"temp\0")
            .to_string_lossy()
            .into_owned()
    } else {
        CStr::from_ptr(col_name).to_string_lossy().into_owned()
    };

    let col_type = exprType((*target_entry).expr as *mut pgrx::pg_sys::Node);
    // TODO: it's possible that col_type could be things other than column types? perhaps T_Var or T_Const?

    let pg_relation = PgRelation::from_pg_owned(RelationIdGetRelation((*target_entry).resorigtbl));
    // TODO: how to get nullability of pg_relation is null?

    let mut nullable = true;
    if !pg_relation.is_null() {
        let col_num = (*target_entry).resorigcol;
        let pg_att = get_attr(pg_relation.as_ptr(), col_num as isize);
        if !pg_att.is_null() {
            nullable = !(*pg_att).attnotnull; // !!!!! nullability
        }
    }

    if let Ok(built_in_oid) = BuiltinOid::try_from(col_type) {
        let datafusion_type = postgres_to_datafusion_type(built_in_oid).unwrap();
        return Ok(Field::new(col_name_str, datafusion_type, nullable));
    } else {
        return Err(format!("Invalid BuiltinOid"));
    }
}

pub unsafe fn transform_targetentry_to_expr(node: *mut Node) -> Result<Expr, String> {
    let target_entry = node as *mut pgrx::pg_sys::TargetEntry;
    let te_expr_node = (*target_entry).expr as *mut pgrx::pg_sys::Node;
    let node_tag = (*te_expr_node).type_;

    match node_tag {
        NodeTag::T_Const => transform_const_to_df_expr(te_expr_node),
        // TODO: handle other types (T_Var, etc.)
        _ => Err(format!("transform_targetentry_to_expr does not handle node_tag {:?}", node_tag))
    }
}

// ---- Every specific node transformation function should have the same signature (*mut Plan, *mut List, Option<LogicalPlan>) -> Result<LogicalPlan, String>

pub unsafe fn transform_seqscan_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
    outer_plan: Option<LogicalPlan>,
) -> Result<LogicalPlan, String> {
    // Plan variables
    let scan = plan as *mut SeqScan;

    // Find the table we're supposed to be scanning by querying the range table
    let rte = unsafe { rt_fetch((*scan).scan.scanrelid, rtable) };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };

    let tablename = format!("{}", pg_relation.oid());
    let table_reference = TableReference::from(tablename.clone());
    let mut projections: Vec<usize> = vec![];

    let list = (*plan).targetlist;

    if !list.is_null() {
        let elements = (*list).elements;
        for i in 0..(*list).length {
            let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;
            let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
            let var = (*target_entry).expr as *mut pgrx::pg_sys::Var;

            let col_idx = (*var).varattno as usize;
            projections.push(col_idx - 1);
        }
    }

    let table_provider =
        task::block_on(CONTEXT.table_provider(table_reference)).expect("Could not get table");
    let table_source = provider_as_source(table_provider);

    return Ok(LogicalPlan::TableScan(
        TableScan::try_new(tablename, table_source, Some(projections), vec![], None)
            .map_err(datafusion_err_to_string("failed to create table scan"))?,
    ));
}

pub unsafe fn transform_result_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
    outer_plan: Option<LogicalPlan>
) -> Result<LogicalPlan, String> {
    /*
     * Taken from postgres source:
     *   Result node -
     *      If no outer plan, evaluate a variable-free targetlist.
     *      If outer plan, return tuples from outer plan (after a level of
     *      projection as shown by targetlist).
     * See nodeResult.c for more details about a child plan (outer plan).
     *     If no outer plan, then equivalent to a Values plan.
    */

    let result = plan as *mut pgrx::pg_sys::Result;

    let mut cols: Vec<Field> = vec![];
    let mut values: Vec<Vec<Expr>> = vec![vec![]];

    let target_list = (*plan).targetlist;
    if !target_list.is_null() {
        let elements = (*target_list).elements;
        for i in 0..(*target_list).length {
            let list_cell_node =
                (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
            match (*list_cell_node).type_ {
                NodeTag::T_TargetEntry => {
                    cols.push(transform_targetentry_to_df_field(list_cell_node)?);
                    values[0].push(transform_targetentry_to_expr(list_cell_node)?);
                },
                _ => return Err(format!("target type {:?} not handled yet for valuesscan", (*list_cell_node).type_)),
            }
        }
    }
    
    let arrow_schema = Schema::new(cols);
    let df_schema = DFSchema::try_from(arrow_schema).map_err(datafusion_err_to_string("result DFSchema failed"))?;

    Ok(LogicalPlan::Values(Values {
        schema: df_schema.clone().into(),
        values: values,
    }))
}

pub unsafe fn transform_valuesscan_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
    outer_plan: Option<LogicalPlan>,
) -> Result<LogicalPlan, String> {
    let valuesscan = plan as *mut ValuesScan;

    let mut cols: Vec<Field> = vec![];
    let target_list = (*plan).targetlist;
    if !target_list.is_null() {
        let elements = (*target_list).elements;
        for i in 0..(*target_list).length {
            let list_cell_node =
                (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
            match (*list_cell_node).type_ {
                NodeTag::T_TargetEntry => {
                    cols.push(transform_targetentry_to_df_field(list_cell_node)?)
                }
                _ => {
                    return Err(format!(
                        "target type {:?} not handled yet for valuesscan",
                        (*list_cell_node).type_
                    ))
                }
            }
        }
    }

    let mut values: Vec<Vec<Expr>> = vec![];
    // TODO: macro out the list iteration
    let values_lists = (*valuesscan).values_lists;
    if !values_lists.is_null() {
        let values_lists_elements = (*values_lists).elements;
        for i in 0..(*values_lists).length {
            let values_list =
                (*values_lists_elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::List;

            let mut values_row: Vec<Expr> = vec![];

            let values_list_elements = (*values_list).elements;
            for j in 0..(*values_list).length {
                let value_expr =
                    (*values_list_elements.offset(j as isize)).ptr_value as *mut pgrx::pg_sys::Node;

                match (*value_expr).type_ {
                    NodeTag::T_Const => {
                        // TODO: actually get the type here - for now just defaulting to Int32
                        values_row.push(transform_const_to_df_expr(value_expr)?);
                    }
                    // TODO: deal with all other types
                    _ => {
                        return Err(format!(
                            "value_expr type {:?} not handled",
                            (*value_expr).type_
                        ))
                    }
                }
            }
            values.push(values_row);
        }
    }

    let arrow_schema = Schema::new(cols);
    let df_schema = DFSchema::try_from(arrow_schema)
        .map_err(datafusion_err_to_string("valuesscan DFSchema failed"))?;

    Ok(LogicalPlan::Values(Values {
        schema: df_schema.clone().into(),
        values: values,
    }))
}

pub unsafe fn transform_limit_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
    outer_plan: Option<LogicalPlan>,
) -> Result<LogicalPlan, String> {
    let outer_plan = outer_plan.ok_or("Limit does not have an outer plan")?;

    let limit_node = plan as *mut pg_sys::Limit;
    let skip_node = (*limit_node).limitOffset;
    let fetch_node = (*limit_node).limitCount;

    let fetch = const_node_value(fetch_node)?.unwrap_or(0);
    let skip = const_node_value(skip_node)?.unwrap_or(0);

    Ok(LogicalPlan::Limit(Limit {
        skip,
        fetch: Some(fetch),
        input: Box::new(outer_plan).into(),
    }))
}

#[inline]
unsafe fn const_node_value(node: *mut pg_sys::Node) -> Result<Option<usize>, String> {
    if node.is_null() {
        return Ok(None);
    }

    if (*node).type_ != NodeTag::T_Const {
        return Err(format!("Expected a Const Node, got {:?}", (*node).type_));
    }

    let const_node = &*(node as *const pg_sys::Const);

    if const_node.constisnull {
        Ok(None)
    } else {
        Ok(Some(const_node.constvalue.value() as usize))
    }
}

pub unsafe fn transform_agg_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
    outer_plan: Option<LogicalPlan>,
) -> Result<LogicalPlan, String> {
    let outer_plan = outer_plan
        .ok_or_else(|| panic!("Limit does not have an outer plan"))
        .unwrap();

    let list = (*plan).targetlist;

    if list.is_null() {
        panic!("Agg targetlist is null");
    }

    let elements = (*list).elements;
    let mut agg_expr: Vec<Expr> = vec![];

    // Iterate through the list of aggregates
    for i in 0..(*list).length {
        let list_cell_node = (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;

        assert!(is_a(list_cell_node, NodeTag::T_TargetEntry));

        let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
        let expr = (*target_entry).expr;

        assert!(is_a(expr as *mut Node, NodeTag::T_Aggref));

        // Map the Postgres aggregate function to a DataFusion aggregate function
        let agg_ref = expr as *mut Aggref;
        let df_agg = transform_pg_agg_to_df_agg((*agg_ref).aggfnoid);

        // Read function arguments
        let args = (*agg_ref).args;
        let mut args_expr: Vec<Expr> = vec![];

        if !args.is_null() {
            let elements = (*args).elements;
            for i in 0..(*args).length {
                let arg_node = (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;

                assert!(is_a(arg_node, NodeTag::T_TargetEntry));

                let target_entry = arg_node as *mut pgrx::pg_sys::TargetEntry;
                let var = (*target_entry).expr as *mut pgrx::pg_sys::Var;

                // For now we'll assume we're using the first entry in the range table
                // TODO: Figure out how to get the correct range table entry
                let var_rte = rt_fetch(1, rtable);
                let var_relid = (*var_rte).relid;
                let att_name = get_attname(var_relid, (*var).varattno, false);
                let att_name_str = CStr::from_ptr(att_name).to_string_lossy().into_owned();

                args_expr.push(Expr::Column(att_name_str.into()));
            }
        }

        // Check if the aggregate is distinct
        let distinct = !(*agg_ref).aggdistinct.is_null();

        // Check if * is used, ie COUNT(*)
        if (*agg_ref).aggstar {
            args_expr = vec![Expr::Wildcard { qualifier: None }];
        }

        // TODO: For now we're ignoring filters and order bys
        // These are only relevant for more complex aggregates which we don't support
        // Don't get this confused with the outer plan's filters and order bys
        agg_expr.push(Expr::AggregateFunction(AggregateFunction::new(
            df_agg, args_expr, distinct, None, None,
        )));
    }

    Ok(LogicalPlan::Aggregate(
        Aggregate::try_new(Box::new(outer_plan).into(), vec![], agg_expr)
            .expect("failed to create aggregate"),
    ))
}

#[inline]
unsafe fn transform_pg_agg_to_df_agg(func_oid: pg_sys::Oid) -> BuiltInAgg {
    let func_name = pg_sys::get_func_name(func_oid);
    let func_name_str = CStr::from_ptr(func_name).to_string_lossy().into_owned();

    match func_name_str.as_str() {
        "sum" => BuiltInAgg::Sum,
        "avg" => BuiltInAgg::Avg,
        "count" => BuiltInAgg::Count,
        "max" => BuiltInAgg::Max,
        "min" => BuiltInAgg::Min,
        _ => todo!(),
    }
}

pub unsafe fn transform_modify_to_df_plan(
    plan: *mut Plan,
    rtable: *mut List,
    outer_plan: Option<LogicalPlan>,
) -> Result<LogicalPlan, String> {
    // Outer plan needs to exist
    if outer_plan.is_none() {
        return Err(format!("ModifyTable does not have outer plan"));
    }

    // Plan variables
    let modify = plan as *mut ModifyTable;

    let rte = rt_fetch((*modify).nominalRelation, rtable);
    let relation = RelationIdGetRelation((*rte).relid);
    let pg_relation = PgRelation::from_pg_owned(relation);

    // let (input, vs_schema) = unsafe { transform_valuesscan_to_datafusion((*plan).lefttree, rtable).expect("valuesscan transformation failed") };
    let tablename = format!("{}", pg_relation.oid());
    let table_reference = TableReference::from(tablename);
    let mut cols: Vec<Field> = vec![];

    let mut table_schema: Option<Arc<DFSchema>> = None;

    // Iterate through the targetlist, which kinda looks like the columns the `SELECT` pulls
    // The nodes are T_TargetEntry
    let list = (*plan).targetlist;
    if !list.is_null() {
        let elements = (*list).elements;
        for i in 0..(*list).length {
            let list_cell_node =
                (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
            match (*list_cell_node).type_ {
                NodeTag::T_TargetEntry => {
                    let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
                    let col_name = (*target_entry).resname;
                    if !col_name.is_null() {
                        let col_name_str = CStr::from_ptr(col_name).to_string_lossy().into_owned();
                        let col_num = (*target_entry).resorigcol;
                        let pg_att = get_attr(relation, col_num as isize);
                        if !pg_att.is_null() {
                            let att_not_null = (*pg_att).attnotnull; // !!!!! nullability
                            if let Ok(built_in_oid) = BuiltinOid::try_from((*pg_att).atttypid) {
                                let datafusion_type = postgres_to_datafusion_type(built_in_oid)?;
                                cols.push(Field::new(col_name_str, datafusion_type, !att_not_null));
                            } else {
                                return Err(format!("Invalid BuiltinOid"));
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "targetlist type {:?} not handled yet for modifytable",
                        (*list_cell_node).type_
                    ))
                }
            }
        }

        let arrow_schema = Schema::new(cols);
        table_schema = Some(
            DFSchema::try_from(arrow_schema)
                .map_err(datafusion_err_to_string("modify DFSchema failed"))?
                .into(),
        );
    } else {
        // If the schema isn't part of the ModifyTable plan, we have to pull it from outer_plan
        match outer_plan
            .as_ref()
            .ok_or("ModifyTable has no schema or outer_plan")?
        {
            datafusion::logical_expr::LogicalPlan::Values(values) => {
                table_schema = Some(values.schema.clone())
            }
            _ => info!("Outer plan type not implemented for ModifyTable"),
        }
    }

    let table_schema = table_schema.ok_or(format!("ModifyTable table_schema is not set"))?;

    return Ok(LogicalPlan::Dml(DmlStatement {
        table_name: table_reference,
        table_schema: table_schema.into(),
        op: match (*modify).operation {
            // TODO: WriteOp::InsertOverwrite also exists - handle that properly
            CmdType_CMD_INSERT => datafusion::logical_expr::WriteOp::InsertInto,
            CmdType_CMD_UPDATE => datafusion::logical_expr::WriteOp::Update,
            CmdType_CMD_DELETE => datafusion::logical_expr::WriteOp::Delete,
            _ => return Err(format!("unsupported modify operation")),
        },
        input: outer_plan.ok_or("ModifyTable has no outer_plan")?.into(),
    }));
}
