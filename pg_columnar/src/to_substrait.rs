/*
 * This file contains utility functions for converting a Postgres query plan
 * into a Substrait query plan.
 * */

use pgrx::prelude::*;
use pgrx::spi::Error;
use pg_sys::{PlannedStmt, SeqScan, RelationIdGetRelation, RangeTblEntry, NameData, pgrx_list_nth, namestrcpy};
use std::ffi::CStr;
use substrait::proto;
use substrait::proto::r#type;

// from chapter 8.1 of the postgres docs
pub enum PostgresType {
    Boolean,
    Integer,
    BigInt,
    Text,
    SmallInt,
    Decimal, //variable
    Real,
    Double,
    Char,
    VarChar,
    BpChar,
    // done: numeric types and character types
    // TODO: unlimited vs limited length, variable precision
}

impl PostgresType {
    fn type_oid(&self) -> u32 {
        match self {
            PostgresType::Boolean => 16,   // OID for boolean
            PostgresType::Integer => 23,   // OID for integer
            PostgresType::BigInt => 20,    // OID for bigint
            PostgresType::Text => 25,      // OID for text
            // TODO: Add missing types
        }
    }
}

// This function converts a PostgresType to a SubstraitType
pub fn postgres_to_substrait_type(p_type: PostgresType, not_null: bool) -> Result<proto::Type, Error> {
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
        },
        PostgresType::Integer => {
            let mut int_type = proto::r#type::I32::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I32(int_type));
        },
        PostgresType::BigInt => {
            let mut bigint_type = proto::r#type::I64::default();
            bigint_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I64(bigint_type));
        },
        PostgresType::Text | PostgresType::VarChar | PostgresType::BpChar=> {
            let mut text_type = proto::r#type::VarChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::VarChar(text_type));
        },
        // TODO: Add missing types
        PostgresType::SmallInt => {
            let mut int_type = proto::r#type::I16::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::I16(int_type));
        },
        PostgresType::Decimal => {
            // TODO: user-specified precision
            let mut decimal_type = proto::r#type::Decimal::default();
            decimal_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Decimal(decimal_type));
        },
        PostgresType::Real => {
            let mut float_type = proto::r#type::Fp32::default();
            float_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Fp32(float_type));
        },
        PostgresType::Double => {
            let mut float_type = proto::r#type::Fp64::default();
            float_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::Fp64(float_type));
        },
        PostgresType::Char => {
            let mut text_type = proto::r#type::FixedChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(proto::r#type::Kind::FixedChar(text_type));
        }
    }
    Ok(s_type) // Return the Substrait type
}

// This function converts a Postgres SeqScan to a Substrait ReadRel
pub fn transform_seqscan_to_substrait(ps: *mut PlannedStmt, sget: *mut proto::ReadRel) -> Result<(), Error> {
    // Plan variables
    let plan = unsafe { (*ps).planTree };
    let scan = plan as *mut SeqScan;
    let rtable = unsafe { (*ps).rtable };

    // RangeTblqEntry
    let rte = unsafe { pgrx_list_nth(rtable, ((*scan).scan.scanrelid - 1).try_into().unwrap()) as *mut RangeTblEntry };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };
    let relname = unsafe { &mut (*(*relation).rd_rel).relname as *mut NameData };

    // TODO: I think we can make this much simpler by exposing NameStr directly in pgrx::pg_sys
    let tablename = unsafe { CStr::from_ptr(relname as *const _ as *const i8) };
    unsafe { namestrcpy(relname, tablename.as_ptr()) };
    let tablename_str = unsafe { CStr::from_ptr(relname as *const _ as *const i8) }
    .to_string_lossy()  // Convert to a String
    .into_owned();
    let table_names = vec![tablename_str]; // Create a Vec<String> with the table name

    // TODO: I only passed in a single table name, but this seems to be for arbitrary many tables that the SeqScan is over, probably
    // we'll need to tweak the logic here to make it work for multiple tables
    let table = proto::read_rel::ReadType::NamedTable(proto::read_rel::NamedTable {
        names: table_names,
        advanced_extension: None
    });

    let base_schema = proto::NamedStruct {
        names: vec![],
        r#struct: Some(proto::r#type::Struct {
            types: vec![],
            type_variation_reference: 0,
            nullability: Into::into(proto::r#type::Nullability::Required),
        }),
    };

    // Iterate through the targetlist, which kinda looks like the columns the `SELECT` pulls
    let list = unsafe { (*plan).targetlist };




    // if (*plan).targetlist != pgrx::NULL {
    //     for i in 0..(*list).length {
    //         let list_cell = (*list).elements.offset(i);
    //         let list_cell_node = (*list_cell).ptr_value as *mut Node; // Corrected casting syntax
    //         let list_cell_node_tag = unsafe { (*list_cell_node).type_ };
    //         match list_cell_node_tag {
    //             NodeTag::T_Var => {
    //                 let var = list_cell_node_tag as *mut Var;
    //                 let list_cell_rte = list_nth((*ps).rtable, (*var).varno - 1);
    //                 base_schema.names.push(get_attname((*list_cell_rte).relid, (*var).varattno, false));
    //             }
    //             base_schema.struct.types.push()
    //         }
    //         // TODO: nullability constraints and type conversion
    //         //       see `DuckDBToSubstrait::DuckToSubstraitType` for type conversion reference
    //     }
    // }




    // TODO: Make sure this is correct
    let sget = proto::ReadRel {
        common: None,
        base_schema: Some(base_schema),
        filter: None,
        best_effort_filter: None,
        projection: None,
        advanced_extension: None,
        read_type: None
    };

    Ok(())
}
