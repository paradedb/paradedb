/*
 * This file contains utility functions for converting a Postgres query plan
 * into a Substrait query plan.
 * */

use pg_sys::{
    namestrcpy, pgrx_list_nth, NameData, PlannedStmt, RangeTblEntry, RelationIdGetRelation, SeqScan,
};
use pgrx::pg_sys::Oid;
use pgrx::prelude::*;
use pgrx::spi::Error;
use std::ffi::CStr;
use substrait::proto;
use substrait::proto::r#type;

// from chapter 8.1 of the postgres docs
#[repr(Oid)]
pub enum PostgresType {
    Boolean = 16,
    Integer = 23,
    BigInt = 20,
    Text = 25,
    SmallInt = 21,
    Decimal = 1700, //variable
    Real = 700,
    Double = 701,
    Char = 18,
    VarChar = 1043,
    BpChar = 1042,
    // done: numeric types and character types
    // TODO: unlimited vs limited length, variable precision
}

impl PostgresType {
    fn from_oid(oid: Oid) -> Option<PostgresType> {
        match oid {
            16 => Some(PostgresType::Boolean),
            23 => Some(PostgresType::Integer),
            20 => Some(PostgresType::BigInt),
            25 => Some(PostgresType::Text),
            21 => Some(PostgresType::SmallInt),
            1700 => Some(PostgresType::Decimal),
            700 => Some(PostgresType::Real),
            701 => Some(PostgresType::Double),
            18 => Some(PostgresType::Char),
            1043 => Some(PostgresType::VarChar),
            1042 => Some(PostgresType::BpChar),
            _ => None,
        }
    }
}

// This function converts a PostgresType to a SubstraitType
pub fn postgres_to_substrait_type(
    p_type: PostgresType,
    not_null: bool,
) -> Result<proto::Type, Error> {
    let mut s_type = proto::Type::default(); // Create a new Type instance.

    // Set the nullability.
    let type_nullability = if not_null {
        proto::r#type::Nullability::Required
    } else {
        proto::r#type::Nullability::Nullable
    };

    // Map each PostgresType to a Substrait type.
    match p_type {
        PostgresType::Boolean => {
            let mut bool_type = proto::r#type::Boolean::default();
            bool_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Bool(bool_type));
        }
        PostgresType::Integer => {
            let mut int_type = proto::r#type::I32::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I32(int_type));
        }
        PostgresType::BigInt => {
            let mut bigint_type = proto::r#type::I64::default();
            bigint_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I64(bigint_type));
        }
        PostgresType::Text | PostgresType::VarChar | PostgresType::BpChar => {
            let mut text_type = proto::r#type::VarChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::VarChar(text_type));
        }
        // TODO: Add missing types
        PostgresType::SmallInt => {
            let mut int_type = proto::r#type::I16::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I16(int_type));
        }
        PostgresType::Decimal => {
            // TODO: user-specified precision
            let mut decimal_type = proto::r#type::Decimal::default();
            decimal_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Decimal(decimal_type));
        }
        PostgresType::Real => {
            let mut float_type = proto::r#type::Fp32::default();
            float_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Fp32(float_type));
        }
        PostgresType::Double => {
            let mut float_type = proto::r#type::Fp64::default();
            float_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Fp64(float_type));
        }
        PostgresType::Char => {
            let mut text_type = proto::r#type::FixedChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::FixedChar(text_type));
        }
    }
    Ok(s_type) // Return the Substrait type
}

// This function converts a Postgres SeqScan to a Substrait ReadRel
pub fn transform_seqscan_to_substrait(
    ps: *mut PlannedStmt,
    sget: *mut proto::ReadRel,
) -> Result<(), Error> {
    // Plan variables
    let plan = unsafe { (*ps).planTree };
    let scan = plan as *mut SeqScan;
    // range table
    let rtable = unsafe { (*ps).rtable };

    // find the table we're supposed to be scanning by querying the range table
    // RangeTblqEntry
    // scanrelid is index into the range table
    let rte = unsafe { pgrx_list_nth(rtable, ((*scan).scan.scanrelid - 1)) as *mut RangeTblEntry };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    let relname = unsafe { &mut (*(*relation).rd_rel).relname as *mut NameData };

    // TODO: I think we can make this much simpler by exposing NameStr directly in pgrx::pg_sys
    let tablename = unsafe { CStr::from_ptr(relname as *const _ as *const i8) };
    unsafe { namestrcpy(relname, tablename.as_ptr()) };
    let tablename_str = unsafe { CStr::from_ptr(relname as *const _ as *const i8) }
        .to_string_lossy() // Convert to a String
        .into_owned();
    let table_names = vec![tablename_str]; // Create a Vec<String> with the table name

    // TODO: I only passed in a single table name, but this seems to be for arbitrary many tables that the SeqScan is over, probably
    // we'll need to tweak the logic here to make it work for multiple tables
    let table = proto::read_rel::ReadType::NamedTable(proto::read_rel::NamedTable {
        names: table_names,
        advanced_extension: None,
    });

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
    // we're only supposed to consider Vars, which correspond to columns
    unsafe {
        let list = (*plan).targetlist;
        if list != pgrx::NULL {
            let elements = (*list).elements;
            for i in 0..(*list).length {
                let list_cell_node = elements.offset(i).ptr_value as *mut pgrx::pg_sys::Node;
                match (*list_cell_node).type_ {
                    NodeTag::T_Var => {
                        // the varno and varattno identify the "semantic referent", which is a base-relation column
                        // unless the reference is to a join ...
                        // target list var no = scanrelid
                        let var = list_cell_node as *mut pgrx::pg_sys::Var;
                        // varno is the index of var's relation in the range table
                        let var_rte = pgrx_list_nth(rtable, (*var).varno) as *mut RangeTblEntry;
                        let var_relid = (*var_rte).relid;
                        // varattno is the attribute number, or 0 for all attributes
                        let att_name = get_attname(var_relid, (*var).varattno, false);
                        // vartype is the pg_type OID for the type of this var
                        let att_type = PostgresType::from_oid((*var).vartype);
                        if let Some(pg_type) = att_type {
                            col_names.push(att_name);
                            // TODO: fill out nullability
                            col_types
                                .types
                                .push(postgres_to_substrait_type(pg_type, false));
                        } else {
                            info!("Oid {} is not supported", (*var).vartype);
                        }
                    }
                }
            }
        }
    }
    base_schema.names = col_names;
    base_schema.r#struct = col_types;
    sget.base_schema = Some(base_schema);
    Ok(())
}
