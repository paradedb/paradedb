// Copyright (c) 2023-2026 ParadeDB, Inc.
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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

pub use common::postgres::utils::*;

use crate::api::operator::row_expr_from_indexed_expr;
use crate::api::tokenizers::{
    AliasTypmod, UncheckedTypmod, type_can_be_tokenized, type_is_alias, type_is_tokenizer,
};
use crate::api::version::Version;
use crate::api::{FieldName, HashMap};
use crate::nodecast;
use crate::postgres::composite::{get_composite_fields_for_index, is_composite_type};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::node::NodeExt;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::TantivyValue;
use crate::schema::{CategorizedFieldData, SearchField, SearchFieldType};
use crate::vector::PgVector;
use crate::vector::metric::VectorMetric;
use pgrx::*;
use rustc_hash::FxHashMap;
use tokenizers::SearchNormalizer;

/// Extracts the field attributes from the index relation.
/// It returns a vector of tuples containing the field name and its type OID.
pub unsafe fn extract_field_attributes(
    indexrel: pg_sys::Relation,
) -> HashMap<FieldName, ExtractedFieldAttribute> {
    #[inline]
    unsafe fn is_text_lower(expression: *mut pg_sys::Node) -> bool {
        if let Some(coerce) = nodecast!(CoerceViaIO, expression)
            && let Some(func_expr) = nodecast!(FuncExpr, (*coerce).arg)
            && (*func_expr).funcid == text_lower_funcoid()
        {
            return true;
        }

        false
    }

    let heap_relation = PgSearchRelation::from_pg(indexrel).heap_relation().unwrap();
    let heap_tupdesc = heap_relation.tuple_desc();
    let pg_search_indexrel = PgSearchRelation::from_pg(indexrel);
    let created_by_version = pg_search_indexrel.created_by_version();
    let index_info = pg_search_indexrel.index_info();
    let expressions = pg_search_indexrel.index_expressions();
    let mut expressions_iter = expressions.iter_ptr().enumerate();
    let mut field_attributes: FxHashMap<FieldName, ExtractedFieldAttribute> = Default::default();
    for attno in 0..(*index_info).ii_NumIndexAttrs {
        let heap_attno = (*index_info).ii_IndexAttrNumbers[attno as usize];
        let (attname, attribute_type_oid, att_typmod, source, expression, inner_typoid, normalizer) =
            if heap_attno == 0 {
                // Is an expression.
                let Some((expression_idx, expression)) = expressions_iter.next() else {
                    panic!("Expected expression for index attribute {attno}.");
                };
                let node = expression.cast();
                let source = FieldSource::Expression {
                    att_idx: expression_idx,
                    attno: nodecast!(Var, strip_tokenizer_cast(node))
                        .map(|var| (*var).varattno)
                        .filter(|&attno| attno > 0)
                        .map(|attno| (attno - 1) as usize),
                };

                let typoid = pg_sys::exprType(node);
                let mut attname = None;
                let mut typmod = -1;
                let mut inner_typoid = typoid;
                let mut normalizer = None;

                // Check if this expression is a composite type
                if is_composite_type(typoid) {
                    // Get composite fields (validates for nested composites, etc.)
                    let composite_fields =
                        get_composite_fields_for_index(typoid).unwrap_or_else(|e| panic!("{e}"));
                    let row_args =
                        row_expr_from_indexed_expr(expression.cast()).map(|row_expr| unsafe {
                            PgList::<pg_sys::Node>::from_pg((*row_expr).args)
                        });

                    // Add each field from the composite to the index
                    for comp_field in composite_fields {
                        if comp_field.is_dropped {
                            continue;
                        }

                        // Check for duplicate field name (reuse existing logic)
                        if field_attributes.contains_key(&FieldName::from(&comp_field.field_name)) {
                            panic!(
                                "indexed attribute {} defined more than once",
                                comp_field.field_name
                            );
                        }

                        let pg_type = PgOid::from_untagged(comp_field.type_oid);
                        let tantivy_type = SearchFieldType::try_from_type_info(
                            pg_type,
                            comp_field.typmod,
                            comp_field.type_oid,
                            created_by_version,
                        )
                        .unwrap_or_else(|e| panic!("{e}"));

                        field_attributes.insert(
                            comp_field.field_name.clone().into(),
                            ExtractedFieldAttribute {
                                attno: attno as usize,
                                source: FieldSource::CompositeField {
                                    index_attno: attno as usize,
                                    expression_idx,
                                    field_idx: comp_field.field_index,
                                    composite_type_oid: typoid,
                                },
                                pg_type,
                                tantivy_type,
                                inner_typoid: row_args
                                    .as_ref()
                                    .and_then(|args| args.get_ptr(comp_field.field_index))
                                    .map(|arg| {
                                        let inner_node = strip_tokenizer_cast(arg.cast());
                                        pg_sys::exprType(inner_node)
                                    })
                                    .unwrap_or(comp_field.type_oid),
                                normalizer: None,
                            },
                        );
                    }

                    // Skip normal expression handling for composite types
                    continue;
                }

                if type_is_tokenizer(typoid) {
                    typmod = pg_sys::exprTypmod(node);

                    let parsed_typmod =
                        UncheckedTypmod::try_from(typmod).unwrap_or_else(|e| e.report());
                    let vars = node.collect_nodes::<pg_sys::Var>();

                    normalizer = parsed_typmod.normalizer();
                    attname = parsed_typmod.alias();

                    // Attempt to determine inner_typoid by peeling the tokenizer cast/function.
                    // This handles cases like `(a || b)::pdb.literal('alias=...')` where vars.len() > 1.
                    if inner_typoid == typoid {
                        let inner_node = strip_tokenizer_cast(expression.cast());
                        inner_typoid = pg_sys::exprType(inner_node);
                    }

                    if attname.is_none() && vars.len() == 1 {
                        let var = vars[0];
                        inner_typoid = pg_sys::exprType(var as *mut pg_sys::Node);
                        if is_text_lower(expression.cast()) {
                            normalizer = Some(SearchNormalizer::Lowercase);
                        } else if let Some(relabel) = nodecast!(RelabelType, expression) {
                            if is_a((*relabel).arg.cast(), pg_sys::NodeTag::T_CoerceViaIO) {
                                inner_typoid = pg_sys::exprType((*relabel).arg.cast());
                            }

                            if is_text_lower((*relabel).arg.cast()) {
                                normalizer = Some(SearchNormalizer::Lowercase);
                            }
                        }
                        if attname.is_none() {
                            let heap_attname = heap_relation
                                .tuple_desc()
                                .get((*var).varattno as usize - 1)
                                .unwrap()
                                .name()
                                .to_string();
                            attname = Some(heap_attname);
                        }
                    }

                    if type_is_alias(typoid) {
                        if type_can_be_tokenized(inner_typoid) {
                            panic!(
                                "To alias a text or JSON type, cast it to a tokenizer with an `alias` argument instead of `pdb.alias`"
                            );
                        } else {
                            // if we have a non text field text to `pdb.alias`, unwrap it to get the inner typoid
                            if let Some(relabel) = nodecast!(RelabelType, expression)
                                && let Some(func) = nodecast!(FuncExpr, (*relabel).arg)
                            {
                                let args = PgList::<pg_sys::Node>::from_pg((*func).args);
                                if args.len() == 1
                                    && let Some(arg) = args.get_ptr(0)
                                {
                                    if let Some(inner_func) = nodecast!(FuncExpr, arg) {
                                        inner_typoid = (*inner_func).funcresulttype;
                                    } else if let Some(op_expr) = nodecast!(OpExpr, T_OpExpr, arg) {
                                        inner_typoid = (*op_expr).opresulttype;
                                    }
                                }
                            }

                            // use the alias name as the field name instead of the heap attribute name
                            let alias_typmod =
                                AliasTypmod::try_from(typmod).unwrap_or_else(|e| e.report());
                            attname = alias_typmod.alias();
                        }
                    }
                }

                let Some(attname) = attname else {
                    let expr_str = deparse_expr(None, &heap_relation, expression.cast());
                    panic!(
                        "indexed expression requires a tokenizer cast with an alias: {expr_str}"
                    );
                };

                (
                    attname,
                    typoid,
                    typmod,
                    source,
                    Some(expression),
                    inner_typoid,
                    normalizer,
                )
            } else {
                // Is a field -- get the field name from the heap relation.
                let attno = (heap_attno - 1) as usize;
                let att = heap_tupdesc.get(attno).expect("attribute should exist");
                (
                    att.name().to_owned(),
                    att.type_oid().value(),
                    att.type_mod(),
                    FieldSource::Heap { attno },
                    None,
                    att.type_oid().value(),
                    None,
                )
            };

        if field_attributes.contains_key(&FieldName::from(&attname)) {
            panic!("indexed attribute {attname} defined more than once");
        }

        let pg_type = PgOid::from_untagged(attribute_type_oid);
        let mut tantivy_type = SearchFieldType::try_from_type_info(
            pg_type,
            att_typmod,
            inner_typoid,
            created_by_version,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        if let SearchFieldType::Vector(oid, dims, _) = tantivy_type {
            let metric =
                VectorMetric::from_index_attr(indexrel, attno as usize).unwrap_or_default();
            tantivy_type = SearchFieldType::Vector(oid, dims, metric);
        }

        // non-plain-attribute expressions that aren't cast to a tokenizer type are forced to use our `pdb.literal` tokenizer
        let missing_tokenizer_cast = expression.is_some()
            && att_typmod == -1
            && matches!(tantivy_type, SearchFieldType::Text(..));
        if missing_tokenizer_cast {
            let expr_str = deparse_expr(None, &heap_relation, expression.unwrap().cast());
            panic!("indexed expression must be cast to a tokenizer: {expr_str}");
        }

        field_attributes.insert(
            attname.into(),
            ExtractedFieldAttribute {
                attno: attno as usize,
                source,
                pg_type,
                tantivy_type,
                inner_typoid,
                normalizer,
            },
        );
    }
    field_attributes
}

pub unsafe fn row_to_search_document<'a>(
    categorized_fields: impl Iterator<
        Item = (
            pg_sys::Datum,
            bool,
            &'a SearchField,
            &'a CategorizedFieldData,
        ),
    >,
    document: &mut tantivy::TantivyDocument,
    created_by_version: Option<Version>,
) {
    for (
        datum,
        isnull,
        search_field,
        CategorizedFieldData {
            pg_type,
            base_oid,
            is_array,
            is_json,
            ..
        },
    ) in categorized_fields
    {
        if isnull {
            continue;
        }

        // For pdb.alias/tokenizer types, get the underlying type if it's not a text type.
        let actual_datum = unsafe { unwrap_alias_datum(datum, *pg_type) };

        if *is_array {
            let converted_array = match search_field.field_type() {
                SearchFieldType::Numeric64(_, scale) => {
                    TantivyValue::try_from_numeric_array_i64(actual_datum, scale)
                }
                SearchFieldType::NumericBytes(..) => {
                    TantivyValue::try_from_numeric_array_bytes(actual_datum, created_by_version)
                }
                // Legacy pre-v0.22.0 indexes stored NUMERIC arrays as F64 in the tantivy schema.
                SearchFieldType::F64(oid) if oid == pg_sys::NUMERICOID => {
                    TantivyValue::try_from_numeric_array_f64(actual_datum)
                }
                _ => TantivyValue::try_from_datum_array(actual_datum, *base_oid),
            }
            .unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            });
            for value in converted_array {
                document.add_field_value(
                    search_field.field(),
                    &value.into_tantivy_value(created_by_version),
                );
            }
        } else if *is_json {
            for value in
                TantivyValue::try_from_datum_json(actual_datum, *base_oid).unwrap_or_else(|e| {
                    panic!("could not parse field `{}`: {e}", search_field.field_name())
                })
            {
                document.add_field_value(
                    search_field.field(),
                    &value.into_tantivy_value(created_by_version),
                );
            }
        } else if matches!(search_field.field_type(), SearchFieldType::Vector(..)) {
            let vec = unsafe {
                PgVector::from_datum(actual_datum, false)
                    .expect("vector field datum should not be NULL")
                    .0
            };
            document.add_vector(search_field.field(), &vec);
        } else {
            let tv = unsafe {
                scalar_datum_to_tantivy_value(
                    actual_datum,
                    search_field.field_type(),
                    *base_oid,
                    created_by_version,
                )
            }
            .unwrap_or_else(|e| {
                panic!("could not parse field `{}`: {e}", search_field.field_name())
            });
            document.add_field_value(
                search_field.field(),
                &tv.into_tantivy_value(created_by_version),
            );
        }
    }
}
