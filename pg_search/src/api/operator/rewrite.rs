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

use super::{
    RHSValue, build_pdb_query_funcexpr, build_text_funcexpr, get_expr_result_type,
    is_pdb_query_castable, is_text_like, pdb_query_typoid, searchqueryinput_typoid,
    validate_lhs_type_as_text_compatible,
};
use crate::api::FieldName;
use crate::api::builder_fns::{
    match_conjunction, match_conjunction_array, match_disjunction, match_disjunction_array, parse,
    parse_with_field, phrase_array, phrase_string, proximity, term_set_str, term_str,
};
use crate::query::SearchQueryInput;
use crate::query::pdb_query::{pdb, to_search_query_input};
use pgrx::{IntoDatum, PgList, direct_function_call, pg_sys};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SearchOperator {
    Conjunction,
    Disjunction,
    Term,
    Phrase,
    Parse,
}

impl SearchOperator {
    fn name(self) -> &'static str {
        match self {
            Self::Conjunction => "&&&",
            Self::Disjunction => "|||",
            Self::Term => "===",
            Self::Phrase => "###",
            Self::Parse => "@@@",
        }
    }

    fn text_query(self, text: String) -> pdb::Query {
        match self {
            Self::Conjunction => match_conjunction(text),
            Self::Disjunction => match_disjunction(text),
            Self::Term => term_str(text),
            Self::Phrase => phrase_string(text),
            Self::Parse => parse_with_field(text, None, None),
        }
    }

    fn array_query(self, array: Vec<String>) -> pdb::Query {
        match self {
            Self::Conjunction => match_conjunction_array(array),
            Self::Disjunction => match_disjunction_array(array),
            Self::Term => term_set_str(array),
            Self::Phrase => phrase_array(array),
            Self::Parse => self.invalid_rhs(),
        }
    }

    pub(super) fn classify_query(self, query: pdb::Query) -> pdb::Query {
        let (query, score) = match query {
            pdb::Query::ScoreAdjusted { query, score } => (*query, Some(score)),
            pdb::Query::UnclassifiedArray {
                array,
                fuzzy_data,
                slop_data,
            } if matches!(self, Self::Conjunction | Self::Disjunction) => {
                // Bare match arrays use fuzzy term-set conversion before setting conjunction mode.
                let mut query = term_set_str(array);
                query.apply_fuzzy_data(fuzzy_data);
                query.apply_slop_data(slop_data);
                assert!(matches!(query, pdb::Query::MatchArray { .. }));
                let pdb::Query::MatchArray {
                    conjunction_mode, ..
                } = &mut query
                else {
                    unreachable!()
                };
                *conjunction_mode = Some(self == Self::Conjunction);
                return query;
            }
            query => (query, None),
        };
        let (mut query, fuzzy_data, slop_data) = match query {
            pdb::Query::UnclassifiedString {
                string,
                fuzzy_data,
                slop_data,
            } => (self.text_query(string), fuzzy_data, slop_data),
            pdb::Query::UnclassifiedArray {
                array,
                fuzzy_data,
                slop_data,
            } if self != Self::Parse => (self.array_query(array), fuzzy_data, slop_data),
            query => (query, None, None),
        };
        if self != Self::Phrase {
            query.apply_fuzzy_data(fuzzy_data);
        }
        query.apply_slop_data(slop_data);
        match score {
            Some(score) => pdb::Query::ScoreAdjusted {
                query: Box::new(query),
                score,
            },
            None => query,
        }
    }

    unsafe fn require_field(self, lhs: *mut pg_sys::Node, field: Option<FieldName>) -> FieldName {
        validate_lhs_type_as_text_compatible(lhs, self.name());
        field.unwrap_or_else(|| {
            panic!(
                "The left hand side of the `{}(field, TEXT)` operator must be a field.",
                self.name()
            )
        })
    }

    fn invalid_rhs(self) -> ! {
        match self {
            Self::Parse => {
                unreachable!("atatat_support should only ever be called with a text value")
            }
            Self::Term => unreachable!(
                "The right-hand side of the `===(field, TEXT)` operator must be a text or text array value"
            ),
            _ => panic!(
                "The right-hand side of the `{}(field, TEXT)` operator must be a text value.",
                self.name()
            ),
        }
    }

    pub(super) unsafe fn rewrite_const(
        self,
        lhs: *mut pg_sys::Node,
        field: Option<FieldName>,
        rhs: RHSValue,
    ) -> SearchQueryInput {
        let field = if self == Self::Parse {
            match rhs {
                RHSValue::Text(text) if field.is_none() => return parse(text, None, None),
                RHSValue::TextArray(_) => self.invalid_rhs(),
                _ => (),
            }
            assert!(field.is_some());
            field.unwrap()
        } else {
            self.require_field(lhs, field)
        };
        let query = match rhs {
            RHSValue::Text(text) => self.text_query(text),
            RHSValue::TextArray(array) => self.array_query(array),
            RHSValue::PdbQuery(
                query @ (pdb::Query::UnclassifiedString { .. }
                | pdb::Query::UnclassifiedArray { .. }
                | pdb::Query::ScoreAdjusted { .. }),
            ) => self.classify_query(query),
            RHSValue::PdbQuery(query) if self == Self::Parse => query,
            RHSValue::ProximityClause(prox) if self == Self::Parse => proximity(prox),
            _ => self.invalid_rhs(),
        };
        to_search_query_input(field, query)
    }

    pub(super) unsafe fn rewrite_exec(
        self,
        field: Option<FieldName>,
        lhs: *mut pg_sys::Node,
        rhs: *mut pg_sys::Node,
    ) -> pg_sys::FuncExpr {
        if self == Self::Parse {
            let expr_type = get_expr_result_type(rhs);
            let is_pdb_query = expr_type == pdb_query_typoid();
            if !(is_text_like(expr_type) || is_pdb_query) {
                panic!("The right-hand side of the `@@@` operator must be a text value");
            }
            let signature = if is_pdb_query {
                c"paradedb.to_search_query_input(paradedb.fieldname, pdb.query)"
            } else if field.is_some() {
                c"paradedb.parse_with_field(paradedb.fieldname, text, bool, bool)"
            } else {
                c"paradedb.parse(text, bool, bool)"
            };
            let funcid = direct_function_call::<pg_sys::Oid>(
                pg_sys::regprocedurein,
                &[signature.into_datum()],
            )
            .unwrap_or_else(|| panic!("`{}` should exist", signature.to_str().unwrap()));

            let mut args = PgList::<pg_sys::Node>::new();
            if let Some(field) = field {
                args.push(field.into_const().cast());
            } else {
                assert!(!is_pdb_query);
            }
            args.push(rhs);
            if !is_pdb_query {
                args.push(pg_sys::makeBoolConst(false, true));
                args.push(pg_sys::makeBoolConst(false, true));
            }

            return pg_sys::FuncExpr {
                xpr: pg_sys::Expr {
                    type_: pg_sys::NodeTag::T_FuncExpr,
                },
                funcid,
                funcresulttype: searchqueryinput_typoid(),
                funcretset: false,
                funcvariadic: false,
                funcformat: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                funccollid: pg_sys::Oid::INVALID,
                inputcollid: pg_sys::Oid::INVALID,
                args: args.into_pg(),
                location: -1,
            };
        }
        let field = self.require_field(lhs, field);
        if self == Self::Term {
            let rhs_type = get_expr_result_type(rhs);
            if is_pdb_query_castable(rhs_type) {
                return build_pdb_query_funcexpr(
                    field,
                    rhs,
                    rhs_type,
                    c"paradedb.term_search_query_input(paradedb.fieldname, pdb.query)",
                );
            }
        }
        let (text_fn, array_fn) = match self {
            Self::Conjunction => (
                c"paradedb.match_conjunction(paradedb.fieldname, text)",
                c"paradedb.match_conjunction(paradedb.fieldname, text[])",
            ),
            Self::Disjunction => (
                c"paradedb.match_disjunction(paradedb.fieldname, text)",
                c"paradedb.match_disjunction(paradedb.fieldname, text[])",
            ),
            Self::Term => (
                c"paradedb.term(paradedb.fieldname, text)",
                c"paradedb.term_set(paradedb.fieldname, text[])",
            ),
            Self::Phrase => (
                c"paradedb.phrase(paradedb.fieldname, text)",
                c"paradedb.phrase_array(paradedb.fieldname, text[])",
            ),
            Self::Parse => unreachable!(),
        };
        build_text_funcexpr(field, rhs, self.name(), text_fn, array_fn)
    }
}
