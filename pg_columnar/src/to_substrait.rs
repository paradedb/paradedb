/*
 * This file contains utility functions for converting a Postgres query plan
 * into a Substrait query plan.
 * */

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
use datafusion_substrait::substrait::proto;
use datafusion_substrait::substrait::proto::expression::literal::LiteralType;
use datafusion_substrait::substrait::proto::r#type;

// TODO: return type: option or just a pointer?
unsafe fn get_attr(table: *mut RelationData, index: isize) -> *const FormData_pg_attribute {
    let tupdesc = (*table).rd_att;
    let natts = (*tupdesc).natts;
    if natts > 0 && (index as i32) <= natts {
        return (*tupdesc).attrs.as_ptr().offset(index - 1);
    } else {
        return std::ptr::null();
    }
}

// This function converts a PgBuiltInOids to a SubstraitType
pub fn postgres_to_substrait_type(
    p_type: PgBuiltInOids,
    not_null: bool,
) -> Result<proto::Type, Error> {
    let mut s_type = proto::Type::default(); // Create a new Type instance.

    // Set the nullability.
    let type_nullability = if not_null {
        proto::r#type::Nullability::Required
    } else {
        proto::r#type::Nullability::Nullable
    };

    // Map each PgBuiltInOids (the Postgres types) to a Substrait type.
    // TODO: Are we covering all OIDs?
    // You can see the full list of OIDs here https://docs.rs/pgrx/latest/pgrx/pg_sys/type.PgBuiltInOids.html
    match p_type {
        PgBuiltInOids::BOOLOID => {
            let mut bool_type = proto::r#type::Boolean::default();
            bool_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Bool(bool_type));
        }
        PgBuiltInOids::INT4OID => {
            let mut int_type = proto::r#type::I32::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I32(int_type));
        }
        PgBuiltInOids::INT8OID => {
            let mut bigint_type = proto::r#type::I64::default();
            bigint_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I64(bigint_type));
        }
        PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::BPCHAROID => {
            let mut text_type = proto::r#type::VarChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Varchar(text_type));
        }
        // TODO: Add missing types
        PgBuiltInOids::INT2OID => {
            let mut int_type = proto::r#type::I16::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I16(int_type));
        }
        PgBuiltInOids::NUMERICOID => {
            // TODO: user-specified precision
            let mut decimal_type = proto::r#type::Decimal::default();
            decimal_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Decimal(decimal_type));
        }
        PgBuiltInOids::FLOAT4OID => {
            let mut float_type = proto::r#type::Fp32::default();
            float_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Fp32(float_type));
        }
        PgBuiltInOids::FLOAT8OID => {
            let mut float_type = proto::r#type::Fp64::default();
            float_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Fp64(float_type));
        }
        PgBuiltInOids::CHAROID => {
            let mut text_type = proto::r#type::FixedChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::FixedChar(text_type));
        }
        _ => {
            // TODO: Implement the rest of the types
        }
    }
    Ok(s_type) // Return the Substrait type
}

unsafe fn transform_const(constant: *mut Const) -> proto::Expression {
    // need the type and value, I think
    let type_oid = (*constant).consttype;
    let val = (*constant).constvalue;
    info!("constant {:?}", val.value());
    proto::Expression {
        rex_type: Some(proto::expression::RexType::Literal(
            proto::expression::Literal {
                nullable: false, // TODO: correct?
                type_variation_reference: 0,
                literal_type: Some(LiteralType::I32(val.value() as i32)), // TODO: convert type_oid to the correct type
            },
        )),
    }
}

// TODO
unsafe fn transform_var(var: *mut Var, rtable: *mut List) {
    if (*var).xpr.type_ != NodeTag::T_Var {
        return;
    }
    // get the attribute
    let var_rte = rt_fetch((*var).varno as u32, rtable);
    let var_relid = (*var_rte).relid;
    // varattno is the attribute number, or 0 for all attributes
    let att_name = get_attname(var_relid, (*var).varattno, false);
    let att_name_str = CStr::from_ptr(att_name).to_string_lossy().into_owned();
    // for now, return the attribute name and type
    // let att_type = PostgresType::from_oid((*var).vartype);
    info!("{:?} type {:?}", att_name_str, (*var).vartype);
}

// TODO
unsafe fn transform_opexpr(op_expr: *mut OpExpr, rtable: *mut List) {
    // TODO: cast this as datum
    let oper_tup = SearchSysCache1(
        SysCacheIdentifier_OPEROID as i32,
        Datum::from((*op_expr).opno),
    );
    // figure out what kind of operator this is
    if !oper_tup.is_null() {
        let pg_op = GETSTRUCT(oper_tup) as *const FormData_pg_operator;
        let oprname = CStr::from_ptr((*pg_op).oprname.data.as_ptr().cast())
            .to_string_lossy()
            .to_owned();
        info!("operator name {:?}, id {:?}", oprname, (*pg_op).oid);
        // iterate through args
        let list = (*op_expr).args;
        if !list.is_null() {
            info!("enumerating through args");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
                match (*list_cell_node).type_ {
                    NodeTag::T_Var => {
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        transform_var(var, rtable);
                    }
                    NodeTag::T_OpExpr => {
                        let child_op_expr = list_cell_node as *mut OpExpr;
                        transform_opexpr(child_op_expr, rtable);
                    }
                    NodeTag::T_Const => {
                        let constant = list_cell_node as *mut Const;
                        transform_const(constant);
                    }
                    _ => (),
                }
            }
        }
    }
}

// This function converts a Postgres SeqScan to a Substrait ReadRel
pub fn transform_seqscan_to_substrait(
    ps: *mut PlannedStmt,
    sget: *mut proto::ReadRel,
// ) -> Result<proto::ReadRel, Error> {
) -> Result<(), Error> {
    // Plan variables
    let plan = unsafe { (*ps).planTree };
    let scan = plan as *mut SeqScan;
    // range table
    let rtable = unsafe { (*ps).rtable };

    // find the table we're supposed to be scanning by querying the range table
    // RangeTblEntry
    // scanrelid is index into the range table
    let rte = unsafe { rt_fetch((*scan).scan.scanrelid, rtable) };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    // let relname = unsafe { &mut (*(*relation).rd_rel).relname as *mut NameData };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };

    // let's enumerate all the fields in Plan, especially qual and initPlan
    unsafe {
        let list = (*plan).qual;
        if !list.is_null() {
            info!("enumerating through qual");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
                match (*list_cell_node).type_ {
                    NodeTag::T_Var => {
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        transform_var(var, rtable);
                    }
                    NodeTag::T_OpExpr => {
                        let op_expr = list_cell_node as *mut OpExpr;
                        transform_opexpr(op_expr, rtable);
                    }
                    _ => (),
                }
            }
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
    let table_names = vec![tablename];

    // TODO: I only passed in a single table name, but this seems to be for arbitrary many tables that the SeqScan is over, probably
    // we'll need to tweak the logic here to make it work for multiple tables
    let table = proto::read_rel::ReadType::NamedTable(proto::read_rel::NamedTable {
        names: table_names,
        advanced_extension: None,
    });
    // TODO: fix << WARNING:  relcache reference leak: relation "test_table" not closed >>

    // following duckdb, create a new schema and fill it with column names
    let mut base_schema = proto::NamedStruct::default();
    let mut col_names: Vec<String> = vec![];
    let mut col_types = proto::r#type::Struct::default();
    col_types.set_nullability(proto::r#type::Nullability::Required);
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
            info!("enumerating through target list");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
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
                            info!("column name {:?}", col_name_str);
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
                                    if let Ok(substrait_type) =
                                        postgres_to_substrait_type(built_in_oid, att_not_null)
                                    {
                                        info!(
                                            "found attribute {:?} with type {:?}",
                                            col_name_str, substrait_type
                                        );
                                        col_names.push(col_name_str);
                                        // TODO: no unwrap, handle error
                                        col_types.types.push(substrait_type);
                                    } else {
                                        info!(
                                            "OID {:?} isn't convertable to substrait type yet",
                                            (*pg_att).atttypid
                                        );
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
            info!("(*plan).targetlist was null");
        }
    }
    base_schema.names = col_names;
    base_schema.r#struct = Some(col_types);
    unsafe {
        (*sget).base_schema = Some(base_schema);
        (*sget).read_type = Some(table)
    };
    info!("finished creating plan");
    Ok(())
    // let sget = proto::ReadRel {
    //     common: None,
    //     base_schema: Some(base_schema),
    //     filter: None,
    //     best_effort_filter: None,
    //     projection: None,
    //     advanced_extension: None,
    //     read_type: Some(table)
    // }
    // Ok(sget)
}

pub fn transform_modify_to_substrait(
    ps: *mut PlannedStmt,
    sput: *mut proto::WriteRel,
// ) -> Result<proto::WriteRel, Error> {
) -> Result<(), Error> {
    // Plan variables
    let plan = unsafe { (*ps).planTree };
    let modify = plan as *mut ModifyTable;
    // range table
    let rtable = unsafe { (*ps).rtable };

    // find the table we're supposed to be modifying by querying the range table
    // RangeTblEntry
    // scanrelid is index into the range table
    let rte = unsafe { rt_fetch((*modify).nominalRelation, rtable) };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };

    // let's enumerate all the fields in Plan, especially qual and initPlan
    unsafe {
        let list = (*plan).qual;
        if !list.is_null() {
            info!("enumerating through qual");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
                match (*list_cell_node).type_ {
                    NodeTag::T_Var => {
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        transform_var(var, rtable);
                    }
                    NodeTag::T_OpExpr => {
                        let op_expr = list_cell_node as *mut OpExpr;
                        transform_opexpr(op_expr, rtable);
                    }
                    _ => (),
                }
            }
        }
    }

    let tablename = format!("{}", pg_relation.oid());
    let table_names = vec![tablename];

    let table = proto::write_rel::WriteType::NamedTable(proto::NamedObjectWrite {
        names: table_names,
        advanced_extension: None,
    });

    let mut table_schema = proto::NamedStruct::default();
    let mut col_names: Vec<String> = vec![];
    let mut col_types = proto::r#type::Struct::default();
    col_types.set_nullability(proto::r#type::Nullability::Required);

    // Iterate through the targetlist, which kinda looks like the columns the `SELECT` pulls
    // The nodes are T_TargetEntry
    unsafe {
        let list = (*plan).targetlist;
        if !list.is_null() {
            info!("enumerating through target list");
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node =
                    (*elements.offset(i as isize)).ptr_value as *mut pgrx::pg_sys::Node;
                info!("node {:?} has type {:?}", i, (*list_cell_node).type_);
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
                            info!("column name {:?}", col_name_str);
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
                                    if let Ok(substrait_type) =
                                        postgres_to_substrait_type(built_in_oid, att_not_null)
                                    {
                                        info!(
                                            "found attribute {:?} with type {:?}",
                                            col_name_str, substrait_type
                                        );
                                        col_names.push(col_name_str);
                                        // TODO: no unwrap, handle error
                                        col_types.types.push(substrait_type);
                                    } else {
                                        info!(
                                            "OID {:?} isn't convertable to substrait type yet",
                                            (*pg_att).atttypid
                                        );
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
            info!("(*plan).targetlist was null");
        }
    }
    table_schema.names = col_names;
    table_schema.r#struct = Some(col_types);
    unsafe {
        (*sput).table_schema = Some(table_schema);
        (*sput).write_type = Some(table);
        (*sput).op = match (*modify).operation {
            CmdType_CMD_INSERT => proto::write_rel::WriteOp::Insert,
            CmdType_CMD_UPDATE => proto::write_rel::WriteOp::Update,
            CmdType_CMD_DELETE => proto::write_rel::WriteOp::Delete,
            _ => proto::write_rel::WriteOp::Unspecified,
        };
        // TODO: handle the case in which we want to return tuples
        (*sput).output = proto::write_rel::OutputMode::NoOutput;
    };
    info!("finished creating plan");
    Ok(())
}

// TODO: figure out what the return type of each transform function should be (a rextype??)
unsafe fn transform_expr_to_substrait(expr: *mut Node) -> Result<proto::Expression, Error> {
    let node_tag = (*expr).type_;
    match node_tag {
        // TODO: these transform functions should return results so errors can propagate. I think.
        NodeTag::T_Const => {
            // literal
            Ok(transform_const(expr as *mut Const))
        }
        NodeTag::T_Var => {
            Ok(proto::Expression::default())
            // field reference
        }
        NodeTag::T_OpExpr => {
            Ok(proto::Expression::default())
            // expression
        }
        _ => Ok(proto::Expression::default()),
    }
}

// This function takes in a Postgres node (e.g. SeqScan), which is analogous to a
// DuckDB LogicalOperator, and it converts it to a Substrait Rel
// Note: We assume the Postgres plan tree is being walked outside of this function, but
// we could combine the two
//
// In Postgres, pretty much everything is a node, which is confusing. Here is a list of all
// the node types I found in the source code:
//
// These are nodes that I don't think are relevant to our work for the query plan, but listing here for reference:
// [NOT RELEVANT HERE] Executor state nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/execnodes.h
// [NOT RELEVANT HERE] Path nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/pathnodes.h
// [NOT RELEVANT HERE] Value nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/value.h
// [NOT RELEVANT HERE] Primitive nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/primnodes.h
// [NOT RELEVANT HERE] Memory Context nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/memnodes.h
// [NOT RELEVANT HERE] Miscellaneous nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/miscnodes.h
// [NOT RELEVANT HERE] Param nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/params.h
// [NOT RELEVANT HERE] Replication grammar parse nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/replnodes.h
//
// These are nodes which I believe we need to handle:
// TODO: The important ones are probably the Query plan nodes, we can start with that and decide later for the rest. I doubt we will need to do all of them.
// Query plan nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/plannodes.h
// Execution nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/backend/executor/execAmi.c#L428
// Expression nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/backend/nodes/nodeFuncs.c#L79
// Utility statement nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/backend/tcop/utility.c#L139
// Extensible nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/execnodes.h
// Tagged nodes -- https://github.com/postgres/postgres/blob/fd5e8b440dfd633be74e3dd3382d4a9038dba24f/src/include/nodes/miscnodes.h
pub fn transform_plan_to_substrait(node: *mut Node) -> Result<proto::Rel, Error> {
    let node_tag = unsafe { (*node).type_ };
    match node_tag {
        NodeTag::T_SeqScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_SampleScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_IndexScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_IndexOnlyScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_BitmapIndexScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_BitmapHeapScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_TidScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_TidRangeScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_SubqueryScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_FunctionScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_TableFuncScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_ValuesScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_CteScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_WorkTableScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_NamedTuplestoreScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_ForeignScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_CustomScan => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Join => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_NestLoop => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_MergeJoin => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_HashJoin => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Material => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Sort => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_IncrementalSort => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Group => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Agg => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_WindowAgg => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Unique => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Gather => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_GatherMerge => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Hash => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_SetOp => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_LockRows => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Limit => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_NestLoopParam => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_PlanRowMark => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Append => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_MergeAppend => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_RecursiveUnion => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Result => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_ProjectSet => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Memoize => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_ModifyTable => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_ExtensibleNode => {
            // TODO: Add function
            Ok(proto::Rel::default())
        }
        NodeTag::T_Invalid => {
            // TODO: Log error instead
            Ok(proto::Rel::default())
        }
        _ => {
            // TODO: Log error instead
            Ok(proto::Rel::default())
        }
    }
}
