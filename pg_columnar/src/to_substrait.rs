/*
 * This file contains utility functions for converting a Postgres query plan
 * into a Substrait query plan.
 * */

use pgrx::prelude::*;
use pgrx::spi::Error;
use pg_sys::{List, SeqScan, RelationIdGetRelation, RangeTblEntry, pgrx_list_nth};

pub enum PostgresType {
    Boolean,
    Integer,
    BigInt,
    Text,
    // TODO: Add missing types
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
pub fn postgres_to_substrait_type(p_type: PostgresType, not_null: bool) -> Result<substrait::proto::Type, Error> {
    let mut s_type = substrait::proto::Type::default(); // Create a new Type instance.

    // Set the nullability.
    let type_nullability = if not_null {
        substrait::proto::r#type::Nullability::Required
    } else {
        substrait::proto::r#type::Nullability::Nullable
    };

    // Map each PostgresType to a Substrait type.
    match p_type {
        PostgresType::Boolean => {
            let mut bool_type = substrait::proto::r#type::Boolean::default();
            bool_type.set_nullability(type_nullability);
            s_type.kind = Some(substrait::proto::r#type::Kind::Bool(bool_type));
        },
        PostgresType::Integer => {
            let mut int_type = substrait::proto::r#type::I32::default();
            int_type.set_nullability(type_nullability);
            s_type.kind = Some(substrait::proto::r#type::Kind::I32(int_type));
        },
        PostgresType::BigInt => {
            let mut bigint_type = substrait::proto::r#type::I64::default();
            bigint_type.set_nullability(type_nullability);
            s_type.kind = Some(substrait::proto::r#type::Kind::I64(bigint_type));
        },
        PostgresType::Text => {
            let mut text_type = substrait::proto::r#type::VarChar::default();
            text_type.set_nullability(type_nullability);
            s_type.kind = Some(substrait::proto::r#type::Kind::Varchar(text_type));
        },
        // TODO: Add missing types        
    }
    Ok(s_type) // Return the Substrait type
}










pub fn transform_seqscan_to_substrait(rtable: *mut List, p_scan: *mut SeqScan, sget: *mut substrait::proto::ReadRel) -> Result<(), Error> {
    // RangeTblqEntry
    let scan = p_scan;
    let rte = unsafe { pgrx_list_nth(rtable, ((*scan).scan.scanrelid - 1).try_into().unwrap()) as *mut RangeTblEntry };
    let relation = unsafe { RelationIdGetRelation((*rte).relid) };



//     let table = substrait::proto::read_rel::ReadType::NamedTable {
//         names: vec![(*(*relation).rd_rel).relname],
//         advanced_extension: None
//     };


// //     sget->mutable_named_table()->add_names(table.name);


// let base_schema = NamedStruct {
//     names: vec![],
//     r#struct: Struct {
//         types: vec![],
//         type_variation_reference: 0,
//         nullability: substrait::proto::r#type::Nullability::Required,
//     },
// };





//     auto type_info = new substrait::Type_Struct();
//     type_info->set_nullability(substrait::Type_Nullability_NULLABILITY_REQUIRED);
//     auto not_null_constraint = GetNotNullConstraintCol(table);
//     for (idx_t i = 0; i < dget.names.size(); i++) {
//         auto cur_type = dget.returned_types[i];
//         if (cur_type.id() == LogicalTypeId::STRUCT) {
//             throw std::runtime_error("Structs are not yet accepted in table scans");
//         }
//         base_schema->add_names(dget.names[i]);
//         auto column_statistics = dget.function.statistics(context, &table_scan_bind_data, i);
//         bool not_null = not_null_constraint.find(i) != not_null_constraint.end();
//         auto new_type = type_info->add_types();
//         *new_type = DuckToSubstraitType(cur_type, column_statistics.get(), not_null);
//     }
//     base_schema->set_allocated_struct_(type_info);
//     sget->set_allocated_base_schema(base_schema);
// }









            // let list = (*plan).targetlist;
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

            // let sget = ReadRel {
            //     common: None,
            //     base_schema: base_schema,
            //     filter: None,
            //     best_effort_filter: None,
            //     projection: None,
            //     advanced_extension: None,
            //     read_type: None
            // };




    Ok(())
}
