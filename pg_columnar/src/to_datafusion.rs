
use datafusion::arrow::datatypes::Schema;
use pg_sys::{
    namestrcpy, pgrx_list_nth, BuiltinOid, Const, Datum, FormData_pg_attribute,
    FormData_pg_operator, List, NameData, Node, OpExpr, PlannedStmt, RangeTblEntry, RelationData,
    RelationIdGetRelation, SearchSysCache1, SeqScan, SysCacheIdentifier_OPEROID, Var, GETSTRUCT,
};
use pgrx::pg_sys::{get_attname, rt_fetch, NodeTag, Oid};
use pgrx::prelude::*;
use pgrx::spi::Error;
use pgrx::PgRelation;
use pgrx::pg_sys::ModifyTable;
use std::ffi::CStr;
use pgrx::pg_sys::Plan;
use pgrx::pg_sys::ValuesScan;
use pgrx::pg_sys::RelationClose;
use pgrx::pg_sys::exprType;

use datafusion::sql::TableReference;
use datafusion::logical_expr::{LogicalPlan, TableScan};
use std::sync::Arc;
use datafusion::logical_expr::DmlStatement;
use crate::col_datafusion::CONTEXT;
use datafusion::datasource::DefaultTableSource;
use datafusion::common::DFSchema;
use datafusion::common::arrow::datatypes::Field;
use datafusion::common::arrow::datatypes::DataType;
use datafusion::logical_expr::TableSource;
use datafusion::logical_expr::Expr;
use datafusion::common::ScalarValue;
use datafusion::logical_expr::Values;
use datafusion::datasource::provider_as_source;

unsafe fn get_attr(table: *mut RelationData, index: isize) -> *const FormData_pg_attribute {
    // info!("{:?}", table);
    let tupdesc = (*table).rd_att;
    let natts = (*tupdesc).natts;
    if natts > 0 && (index as i32) <= natts {
        return (*tupdesc).attrs.as_ptr().offset(index - 1);
    } else {
        return std::ptr::null();
    }
}

pub fn postgres_to_datafusion_type(
    p_type: PgBuiltInOids
) -> Result<DataType, Error> {
    // Map each PgBuiltInOids (the Postgres types) to a Datafusion type.
    // TODO: Are we covering all OIDs?
    // You can see the full list of OIDs here https://docs.rs/pgrx/latest/pgrx/pg_sys/type.PgBuiltInOids.html
    Ok(match p_type {
        PgBuiltInOids::BOOLOID => DataType::Boolean,
        PgBuiltInOids::INT4OID => DataType::Int32,
        PgBuiltInOids::INT8OID => DataType::Int64,
        PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::BPCHAROID => DataType::Utf8,
        // TODO: Add missing types
        PgBuiltInOids::INT2OID => DataType::Int16,
        // TODO: user-specified precision
        PgBuiltInOids::NUMERICOID => DataType::Decimal128(5,2),
        PgBuiltInOids::FLOAT4OID => DataType::Float32,
        PgBuiltInOids::FLOAT8OID => DataType::Float64,
        // TODO: Utf8 is variable length
        PgBuiltInOids::CHAROID => DataType::Utf8,
        // TODO: Implement the rest of the types
        _ => DataType::Null,
    })
}

pub async unsafe fn transform_seqscan_to_datafusion(
    plan: *mut Plan,
    rtable: *mut List
) -> Result<LogicalPlan, Error> {
    // Plan variables
    let scan = plan as *mut SeqScan;

    // find the table we're supposed to be scanning by querying the range table
    // RangeTblEntry
    // scanrelid is index into the range table
    let rte = unsafe { rt_fetch((*scan).scan.scanrelid, rtable) };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    // let relname = unsafe { &mut (*(*relation).rd_rel).relname as *mut NameData };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };

    // let's enumerate all the fields in Plan, especially qual and initPlan
    let list = (*plan).qual;
    if !list.is_null() {
        // info!("enumerating through qual");
        let elements = (*list).elements;
        for i in 0..(*list).length {
            let list_cell_node =
                (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
            // info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
            // match (*list_cell_node).type_ {
            //     NodeTag::T_Var => {
            //         let var = list_cell_node as *mut pgrx::pg_sys::Var;
            //         transform_var(var, rtable);
            //     }
            //     NodeTag::T_OpExpr => {
            //         let op_expr = list_cell_node as *mut OpExpr;
            //         transform_opexpr(op_expr, rtable);
            //     }
            //     _ => (),
            // }
        }
    }
    /*
    let list = (*plan).initPlan;
    if !list.is_null() {
        // these are subplan nodes that are supposed to exist in ps.sbplans
        info!("enumerating through initPlan");
        let elements = (*list).elements;
        for i in 0..(*list).length {
            let list_cell_node =
                (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
            info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
        }
    }
    */

    // // TODO: I think we can make this much simpler by exposing NameStr directly in pgrx::pg_sys
    // let tablename = unsafe { CStr::from_ptr(relname as *const _ as *const i8) };
    // unsafe { namestrcpy(relname, tablename.as_ptr()) };
    // let tablename_str = unsafe { CStr::from_ptr(relname as *const _ as *const i8) }
    //     .to_string_lossy() // Convert to a String
    //     .into_owned();
    // info!("table name {:?}", tablename_str);
    // let table_names = vec![tablename_str]; // Create a Vec<String> with the table name
    // // TODO: in Table AM, we create tables using the OID instead, so we should probably pass that instead?
    let tablename = format!("{}", pg_relation.oid());
    let table_reference = TableReference::from(tablename.clone());
    let mut cols: Vec<Field> = vec![];

    // TODO: I only passed in a single table name, but this seems to be for arbitrary many tables that the SeqScan is over, probably
    // we'll need to tweak the logic here to make it work for multiple tables
    // let table = proto::read_rel::ReadType::NamedTable(proto::read_rel::NamedTable {
    //     names: table_names,
    //     advanced_extension: None,
    // });

    // // following duckdb, create a new schema and fill it with column names
    // let mut base_schema = proto::NamedStruct::default();
    // let mut col_names: Vec<String> = vec![];
    // let mut col_types = proto::r#type::Struct::default();
    // col_types.set_nullability(proto::r#type::Nullability::Required);
    /*
    let mut base_schema = proto::NamedStruct {
        names: vec![],
        r#struct: Some(proto::r#type::Struct {
            types: vec![],
            type_variation_reference: 0,
            nullability: Into::into(proto::r#type::Nullability::Required),
        }),
    };
    */

    // Iterate through the targetlist, which kinda looks like the columns the `SELECT` pulls
    // The nodes are T_TargetEntry
    unsafe {
        let list = (*plan).targetlist;
        if !list.is_null() {
            // info!("enumerating through target list");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                // info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
                match (*list_cell_node).type_ {
                    NodeTag::T_TargetEntry => {
                        // the name of the column is resname
                        // the oid of the source table is resorigtbl
                        // the column's number in source table is resorigcol
                        let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
                        let col_name = (*target_entry).resname;
                        if !col_name.is_null() {
                            let col_name_str =
                                CStr::from_ptr(col_name).to_string_lossy().into_owned();
                            // info!("column name {:?}", col_name_str);
                            // type of column
                            // get the tupel descr data
                            // sanity check?
                            // let tupdesc = (*relation).rd_att;
                            // resorigcol: column's original number in source table
                            let col_num = (*target_entry).resorigcol;
                            // TODO: I did these type conversions just to silence the compiler
                            let pg_att = get_attr(relation, col_num as isize);
                            if !pg_att.is_null() {
                                let att_not_null = (*pg_att).attnotnull; // !!!!! nullability
                                let p_type_id = BuiltinOid::try_from((*pg_att).atttypid);
                                if let Ok(built_in_oid) = p_type_id {
                                    if let Ok(datafusion_type) =
                                        postgres_to_datafusion_type(built_in_oid)
                                    {
                                        // info!(
                                        //     "found attribute {:?} with type {:?}",
                                        //     col_name_str, datafusion_type
                                        // );
                                        // col_names.push(col_name_str);
                                        // // TODO: no unwrap, handle error
                                        // col_types.types.push(substrait_type);
                                        cols.push(Field::new(col_name_str, datafusion_type, !att_not_null));
                                    } else {
                                        // info!(
                                        //     "OID {:?} isn't convertable to datafusion type yet",
                                        //     (*pg_att).atttypid
                                        // );
                                    }
                                }
                            }
                        }
                    }
                    /*
                    NodeTag::T_Var => {
                        // the varno and varattno identify the "semantic referent", which is a base-relation column
                        // unless the reference is to a join ...
                        // target list var no = scanrelid
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        // varno is the index of var's relation in the range table
                        let var_rte = rt_fetch((*var).varno as u32, rtable);
                        let var_relid = (*var_rte).relid;
                        // varattno is the attribute number, or 0 for all attributes
                        let att_name = get_attname(var_relid, (*var).varattno, false);
                        let att_name_str = CStr::from_ptr(att_name).to_string_lossy().into_owned();
                        // vartype is the pg_type OID for the type of this var
                        let att_type = PostgresType::from_oid((*var).vartype);
                        if let Some(pg_type) = att_type {
                            info!("found attribute {:?} with type {:?}", att_name_str, pg_type);
                            col_names.push(att_name_str);
                            // TODO: fill out nullability
                            // TODO: no unwrap, handle error
                            col_types
                                .types
                                .push(postgres_to_substrait_type(pg_type, false)?);
                        } else {
                            // TODO: return error?
                            info!("Oid {} is not supported", (*var).vartype);
                        }
                    }
                    */
                    _ => {}
                }
            }
        } else {
            // info!("(*plan).targetlist was null");
        }
    }

    // let arrow_schema = Schema::new(cols);
    // info!("arrow_schema: {:?}", arrow_schema);
    // let df_schema = DFSchema::try_from_qualified_schema(&table_reference, &arrow_schema).unwrap();

    // let arrow_schema = Schema::new(vec![Field::new("a", DataType::Int32, true)]);
    // let df_schema = DFSchema::try_from_qualified_schema(&table_reference, &arrow_schema).unwrap();

    let table_provider = CONTEXT.table_provider(&table_reference).await.expect("failed to get table provider");
    let table_source = provider_as_source(table_provider);
    return Ok(LogicalPlan::TableScan(TableScan::try_new(
        tablename,
        table_source,
        None,
        vec![],
        None).expect("failed to create table scan")
    ));
}

// pub fn transform_result_to_df_logicalplan(
//     plan: *mut Plan,
//     rtable: *mut List
// ) -> Result<LogicalPlan, Error> {
//     let result = plan as *mut Result;

//     let result_state = 
// }

pub unsafe fn transform_valuesscan_to_datafusion(
    plan: *mut Plan,
    rtable: *mut List
) -> Result<(LogicalPlan, Arc<DFSchema>), Error> {
    let valuesscan = plan as *mut ValuesScan;

    // let rtable = unsafe { (*ps).rtable };

    // // find the table we're supposed to be scanning by querying the range table
    // // RangeTblEntry
    // // scanrelid is index into the range table
    // let rte = unsafe { rt_fetch((*valuesscan).scan.scanrelid, rtable) };
    // unsafe { info!("length {} relid {}", (*rtable).length, (*rte).relid) };
    // let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    // // let relname = unsafe { &mut (*(*relation).rd_rel).relname as *mut NameData };
    // let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };

    let mut cols: Vec<Field> = vec![];
    unsafe {
        // let result_node = (*valuesscan).scan.plan.lefttree as *mut pgrx::pg_sys::Result;
        // let target_list = (*(*result_node).plan).targetlist as *mut pgrx::pg_sys::List;
        let target_list = (*plan).targetlist;

        if !target_list.is_null() {
            // info!("enumerating through target list");
            let elements = (*target_list).elements;
            for i in 0..(*target_list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                // info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
                match (*list_cell_node).type_ {
                    NodeTag::T_TargetEntry => {
                        // the name of the column is resname
                        // the oid of the source table is resorigtbl
                        // the column's number in source table is resorigcol
                        let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
                        let col_name = (*target_entry).resname;
                        if !col_name.is_null() {
                            let col_name_str =
                                CStr::from_ptr(col_name).to_string_lossy().into_owned();
                            // info!("column name {:?}", col_name_str);
                            // type of column
                            // get the tupel descr data
                            // sanity check?
                            // let tupdesc = (*relation).rd_att;
                            // resorigcol: column's original number in source table
                            // info!("resorigtbl {} resorigcol {} resno {}", (*target_entry).resorigtbl, (*target_entry).resorigcol, (*target_entry).resno);
                            // let col_num = (*target_entry).resorigcol;
                            // TODO: I did these type conversions just to silence the compiler
                            // let relation = unsafe { RelationIdGetRelation((*target_entry).resorigtbl) };
                            // info!("get_attr");
                            // let pg_att = get_attr(relation, col_num as isize);
                            // info!("got attr");
                            // if !pg_att.is_null() {
                            //     let att_not_null = (*pg_att).attnotnull; // !!!!! nullability
                            //     let p_type_id = BuiltinOid::try_from((*pg_att).atttypid);
                            //     if let Ok(built_in_oid) = p_type_id {
                            //         if let Ok(datafusion_type) =
                            //             postgres_to_datafusion_type(built_in_oid)
                            //             // postgres_to_substrait_type(built_in_oid, att_not_null)
                            //         {
                            //             info!(
                            //                 "found attribute {:?} with type {:?}",
                            //                 col_name_str, datafusion_type
                            //             );
                            //             // col_names.push(col_name_str);
                            //             // // TODO: no unwrap, handle error
                            //             // col_types.types.push(substrait_type);
                            //             cols.push(Field::new(col_name_str, datafusion_type, !att_not_null));
                            //         } else {
                            //             info!(
                            //                 "OID {:?} isn't convertable to substrait type yet",
                            //                 (*pg_att).atttypid
                            //             );
                            //         }
                            //     }
                            // }
                            // RelationClose(relation);
                            let col_type = exprType((*target_entry).expr as *mut pgrx::pg_sys::Node);
                            if let Ok(res_p_type) = BuiltinOid::try_from(col_type) {
                                if let Ok(datafusion_type) = postgres_to_datafusion_type(res_p_type) {
                                    cols.push(Field::new(col_name_str, datafusion_type, true)); // TODO: actually get nullability
                                } else {
                                    info!("OID {:?} isn't convertable to datafusion type yet", col_type);
                                }
                            } else {
                                info!("not a builtin OID");
                            }
                        }
                    }
                    /*
                    NodeTag::T_Var => {
                        // the varno and varattno identify the "semantic referent", which is a base-relation column
                        // unless the reference is to a join ...
                        // target list var no = scanrelid
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        // varno is the index of var's relation in the range table
                        let var_rte = rt_fetch((*var).varno as u32, rtable);
                        let var_relid = (*var_rte).relid;
                        // varattno is the attribute number, or 0 for all attributes
                        let att_name = get_attname(var_relid, (*var).varattno, false);
                        let att_name_str = CStr::from_ptr(att_name).to_string_lossy().into_owned();
                        // vartype is the pg_type OID for the type of this var
                        let att_type = PostgresType::from_oid((*var).vartype);
                        if let Some(pg_type) = att_type {
                            info!("found attribute {:?} with type {:?}", att_name_str, pg_type);
                            col_names.push(att_name_str);
                            // TODO: fill out nullability
                            // TODO: no unwrap, handle error
                            col_types
                                .types
                                .push(postgres_to_substrait_type(pg_type, false)?);
                        } else {
                            // TODO: return error?
                            info!("Oid {} is not supported", (*var).vartype);
                        }
                    }
                    */
                    _ => {}
                }
            }
        }
    }

    let mut values: Vec<Vec<Expr>> = vec![];
    unsafe {
        // TODO: macro out the list iteration
        let values_lists = (*valuesscan).values_lists;
        if !values_lists.is_null() {
            // info!("enumerating through values lists");
            let values_lists_elements = (*values_lists).elements;
            for i in 0..(*values_lists).length {
                // info!("i {}", i);
                let values_list =
                    (*values_lists_elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::List;

                let mut values_row: Vec<Expr> = vec![];
                
                let values_list_elements = (*values_list).elements;
                for j in 0..(*values_list).length {
                    // info!("j {}", j);
                    let value_expr = (*values_list_elements.offset(j as isize)).ptr_value as *mut pgrx::pg_sys::Node;

                    // info!("value_expr type {:?}", (*value_expr).type_);

                    match (*value_expr).type_ {
                        NodeTag::T_Const => {
                            let const_expr = value_expr as *mut pgrx::pg_sys::Const;

                            let value_type = (*const_expr).consttype; // Oid
                            let value_datum = (*const_expr).constvalue; // Datum
                            let value_isnull = (*const_expr).constisnull; // bool

                            // info!("value {}", value_datum.value());

                            // TODO: actually get the type here - for now just defaulting to Int32
                            values_row.push(Expr::Literal(ScalarValue::Int32(Some(value_datum.value() as i32))));
                        }
                        // TODO: deal with all other types
                        _ => (),
                    }
                }
                values.push(values_row);
            }
        } else {
            info!("values_lists is null");
        }
    }

    // info!("values: {:?}", values);

    // let arrow_schema = Schema::new(vec![Field::new("a", DataType::Int32, true)]);
    let arrow_schema = Schema::new(cols);
    let df_schema = DFSchema::try_from(arrow_schema).unwrap();

    Ok((LogicalPlan::Values(Values {
        schema: df_schema.clone().into(),
        values: values
    }), df_schema.clone().into()))
}

pub fn transform_modify_to_datafusion(
    plan: *mut Plan,
    rtable: *mut List
) -> Result<LogicalPlan, Error> {
    // Plan variables
    let modify = plan as *mut ModifyTable;

    // find the table we're supposed to be modifying by querying the range table
    // RangeTblEntry
    // scanrelid is index into the range table
    let rte = unsafe { rt_fetch((*modify).nominalRelation, rtable) };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };

    // // let's enumerate all the fields in Plan, especially qual and initPlan
    // unsafe {
    //     let list = (*plan).qual;
    //     if !list.is_null() {
    //         info!("enumerating through qual");
    //         let elements = (*list).elements;
    //         for i in 0..(*list).length {
    //             let list_cell_node =
    //                 (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
    //             info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
    //             // match (*list_cell_node).type_ {
    //             //     NodeTag::T_Var => {
    //             //         let var = list_cell_node as *mut pgrx::pg_sys::Var;
    //             //         transform_var(var, rtable);
    //             //     }
    //             //     NodeTag::T_OpExpr => {
    //             //         let op_expr = list_cell_node as *mut OpExpr;
    //             //         transform_opexpr(op_expr, rtable);
    //             //     }
    //             //     _ => (),
    //             // }
    //         }
    //     }
    // }

    let (input, vs_schema) = unsafe { transform_valuesscan_to_datafusion((*plan).lefttree, rtable).expect("valuesscan transformation failed") };
    let tablename = format!("{}", pg_relation.oid());
    let table_reference = TableReference::from(tablename);
    let mut cols: Vec<Field> = vec![];

    let arc_schema: Arc<DFSchema>;

    // Iterate through the targetlist, which kinda looks like the columns the `SELECT` pulls
    // The nodes are T_TargetEntry
    unsafe {
        let list = (*plan).targetlist;
        if !list.is_null() {
            // info!("enumerating through target list");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                // info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
                match (*list_cell_node).type_ {
                    NodeTag::T_TargetEntry => {
                        // the name of the column is resname
                        // the oid of the source table is resorigtbl
                        // the column's number in source table is resorigcol
                        let target_entry = list_cell_node as *mut pgrx::pg_sys::TargetEntry;
                        let col_name = (*target_entry).resname;
                        if !col_name.is_null() {
                            let col_name_str =
                                CStr::from_ptr(col_name).to_string_lossy().into_owned();
                            // info!("column name {:?}", col_name_str);
                            // type of column
                            // get the tupel descr data
                            // sanity check?
                            // let tupdesc = (*relation).rd_att;
                            // resorigcol: column's original number in source table
                            let col_num = (*target_entry).resorigcol;
                            // TODO: I did these type conversions just to silence the compiler
                            let pg_att = get_attr(relation, col_num as isize);
                            if !pg_att.is_null() {
                                let att_not_null = (*pg_att).attnotnull; // !!!!! nullability
                                let p_type_id = BuiltinOid::try_from((*pg_att).atttypid);
                                if let Ok(built_in_oid) = p_type_id {
                                    if let Ok(datafusion_type) =
                                        postgres_to_datafusion_type(built_in_oid)
                                        // postgres_to_substrait_type(built_in_oid, att_not_null)
                                    {
                                        // info!(
                                        //     "found attribute {:?} with type {:?}",
                                        //     col_name_str, datafusion_type
                                        // );
                                        // col_names.push(col_name_str);
                                        // // TODO: no unwrap, handle error
                                        // col_types.types.push(substrait_type);
                                        cols.push(Field::new(col_name_str, datafusion_type, !att_not_null));
                                    } else {
                                        // info!(
                                        //     "OID {:?} isn't convertable to substrait type yet",
                                        //     (*pg_att).atttypid
                                        // );
                                    }
                                }
                            }
                        }
                    }
                    /*
                    NodeTag::T_Var => {
                        // the varno and varattno identify the "semantic referent", which is a base-relation column
                        // unless the reference is to a join ...
                        // target list var no = scanrelid
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        // varno is the index of var's relation in the range table
                        let var_rte = rt_fetch((*var).varno as u32, rtable);
                        let var_relid = (*var_rte).relid;
                        // varattno is the attribute number, or 0 for all attributes
                        let att_name = get_attname(var_relid, (*var).varattno, false);
                        let att_name_str = CStr::from_ptr(att_name).to_string_lossy().into_owned();
                        // vartype is the pg_type OID for the type of this var
                        let att_type = PostgresType::from_oid((*var).vartype);
                        if let Some(pg_type) = att_type {
                            info!("found attribute {:?} with type {:?}", att_name_str, pg_type);
                            col_names.push(att_name_str);
                            // TODO: fill out nullability
                            // TODO: no unwrap, handle error
                            col_types
                                .types
                                .push(postgres_to_substrait_type(pg_type, false)?);
                        } else {
                            // TODO: return error?
                            info!("Oid {} is not supported", (*var).vartype);
                        }
                    }
                    */
                    _ => {}
                }
            }

            let arrow_schema = Schema::new(cols);
            let df_schema = DFSchema::try_from(arrow_schema).unwrap();
            arc_schema = df_schema.into();
        } else {
            // info!("modify (*plan).targetlist was null");
            arc_schema = vs_schema;
        }
    }
    // let arrow_schema = Schema::new(cols);
    // let df_schema = DFSchema::try_from(arrow_schema).unwrap();



    // info!("finished creating plan");
    return Ok(LogicalPlan::Dml(DmlStatement {
        table_name: table_reference,
        table_schema: arc_schema,
        op: unsafe { match (*modify).operation {
            // TODO: WriteOp::InsertOverwrite also exists - handle that properly
            CmdType_CMD_INSERT => datafusion::logical_expr::WriteOp::InsertInto,
            CmdType_CMD_UPDATE => datafusion::logical_expr::WriteOp::Update,
            CmdType_CMD_DELETE => datafusion::logical_expr::WriteOp::Delete,
            _ => panic!("unsupported modify operation"),
        } },
        input: input.into()
    }));
}
