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

use std::ffi::CString;
use std::ops::Bound;
use std::ptr::null_mut;

use pgrx::{PgBox, PgList, PgMemoryContexts, pg_sys};
use tantivy::Term;
use tantivy::columnar::MonotonicallyMappableToU64;
use tantivy::query::{
    BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, ExistsQuery, Occur, Query,
    RangeQuery, TermQuery, TermSetQuery,
};
use tantivy::schema::IndexRecordOption;

use crate::api::FieldName;
use crate::api::version::VersionInfo;
use crate::index::reader::index::DocsEstimate;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::node::NodeExt;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::make_simple_restrictinfo;
use crate::query::SearchQueryInput;
use crate::query::estimate_tree::QueryWithEstimates;
use crate::query::pdb_query::pdb;
use crate::scan::info::RowEstimate;
use crate::schema::SearchFieldType;

#[derive(Clone, Copy)]
pub(crate) struct Planner {
    pub root: *mut pg_sys::PlannerInfo,
    pub rti: pg_sys::Index,
}

struct Estimate {
    clause: *mut pg_sys::Node,
    cost: f64,
}

struct Estimator<'a> {
    reader: &'a SearchIndexReader,
    index: &'a PgSearchRelation,
    heap: PgSearchRelation,
    planner: Planner,
    standalone: bool,
}

pub(crate) fn estimate(
    reader: &SearchIndexReader,
    index: &PgSearchRelation,
    query: &SearchQueryInput,
    rows: RowEstimate,
    planner: Option<Planner>,
) -> DocsEstimate {
    let mut memory = PgMemoryContexts::new("pg_search selectivity");
    unsafe {
        memory.switch_to(|_| {
            let estimator = Estimator::new(reader, index, planner);
            let result = estimator.input(query);
            let selectivity = estimator.selectivity(result.clause);
            let total_docs = match rows {
                RowEstimate::Known(rows) if rows > 0 => rows,
                _ => reader.total_docs(),
            };
            let scale = total_docs as f64 / reader.total_docs().max(1) as f64;
            DocsEstimate {
                selectivity,
                matching_docs: (selectivity * total_docs as f64).ceil() as usize,
                total_docs,
                query_cost: (result.cost * scale).ceil() as u64,
            }
        })
    }
}

impl<'a> Estimator<'a> {
    unsafe fn new(
        reader: &'a SearchIndexReader,
        index: &'a PgSearchRelation,
        planner: Option<Planner>,
    ) -> Self {
        let heap = index.heap_relation().expect("index must have a heap");
        let planner = planner.filter(|planner| {
            let root = planner.root;
            !root.is_null()
                && planner.rti > 0
                && (planner.rti as i32) < (*root).simple_rel_array_size
                && !(*root).simple_rel_array.is_null()
                && !(*root).simple_rte_array.is_null()
                && !(*(*root).simple_rel_array.add(planner.rti as usize)).is_null()
                && {
                    let rte = *(*root).simple_rte_array.add(planner.rti as usize);
                    !rte.is_null()
                        && (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION
                        && (*rte).relid == heap.oid()
                }
        });
        let standalone = planner.is_none();
        let planner = planner.unwrap_or_else(|| {
            let pstate = pg_sys::make_parsestate(null_mut());
            pg_sys::addRangeTableEntryForRelation(
                pstate,
                heap.as_ptr(),
                pg_sys::AccessShareLock as i32,
                null_mut(),
                false,
                true,
            );
            let mut parse = PgBox::<pg_sys::Query>::alloc_node(pg_sys::NodeTag::T_Query);
            parse.rtable = (*pstate).p_rtable;
            #[cfg(not(feature = "pg15"))]
            {
                parse.rteperminfos = (*pstate).p_rteperminfos;
            }
            let mut root = PgBox::<pg_sys::PlannerInfo>::alloc_node(pg_sys::NodeTag::T_PlannerInfo);
            root.parse = parse.into_pg();
            root.glob =
                PgBox::<pg_sys::PlannerGlobal>::alloc_node(pg_sys::NodeTag::T_PlannerGlobal)
                    .into_pg();
            root.query_level = 1;
            root.planner_cxt = pg_sys::CurrentMemoryContext;
            pg_sys::setup_simple_rel_arrays(root.as_ptr());
            if pg_sys::check_enable_rls(heap.oid(), pg_sys::InvalidOid, true)
                == pg_sys::CheckEnableRlsResult::RLS_ENABLED as i32
            {
                let mut security_quals = PgList::<pg_sys::Node>::new();
                security_quals.push(pg_sys::makeBoolConst(true, false).cast());
                (**root.simple_rte_array.add(1)).securityQuals = security_quals.into_pg();
            }

            let mut rel = PgBox::<pg_sys::RelOptInfo>::alloc_node(pg_sys::NodeTag::T_RelOptInfo);
            rel.reloptkind = pg_sys::RelOptKind::RELOPT_BASEREL;
            rel.relid = 1;
            rel.relids = pg_sys::bms_make_singleton(1);
            rel.rtekind = pg_sys::RTEKind::RTE_RELATION;
            rel.tuples = heap
                .reltuples()
                .map(f64::from)
                .unwrap_or(reader.total_docs() as f64);
            rel.rows = rel.tuples;
            *root.simple_rel_array.add(1) = rel.into_pg();
            pg_sys::free_parsestate(pstate);
            Planner {
                root: root.into_pg(),
                rti: 1,
            }
        });
        Self {
            reader,
            index,
            heap,
            planner,
            standalone,
        }
    }

    fn selectivity(&self, clause: *mut pg_sys::Node) -> f64 {
        unsafe {
            pg_sys::clause_selectivity(
                self.planner.root,
                clause,
                self.planner.rti as i32,
                pg_sys::JoinType::JOIN_INNER,
                null_mut(),
            )
            .clamp(0.0, 1.0)
        }
    }

    fn prior(&self, selectivity: f64, cost: f64) -> Estimate {
        unsafe {
            let mut info =
                PgBox::<pg_sys::RestrictInfo>::alloc_node(pg_sys::NodeTag::T_RestrictInfo);
            info.clause = pg_sys::makeBoolConst(true, false).cast();
            info.norm_selec = selectivity.clamp(0.0, 1.0);
            info.outer_selec = info.norm_selec;
            Estimate {
                clause: info.into_pg().cast(),
                cost,
            }
        }
    }

    fn fallback(&self, selectivity: f64) -> Estimate {
        self.prior(selectivity, self.reader.total_docs() as f64)
    }

    fn combine(&self, op: pg_sys::BoolExprType::Type, parts: Vec<Estimate>) -> Estimate {
        if parts.is_empty() {
            return self.prior(
                if op == pg_sys::BoolExprType::AND_EXPR {
                    1.0
                } else {
                    0.0
                },
                0.0,
            );
        }
        let cost = parts.iter().map(|e| e.cost).sum();
        let mut args = PgList::<pg_sys::Node>::new();
        for part in parts {
            unsafe {
                if (*part.clause).type_ == pg_sys::NodeTag::T_BoolExpr
                    && (*part.clause.cast::<pg_sys::BoolExpr>()).boolop == op
                    && op != pg_sys::BoolExprType::NOT_EXPR
                {
                    for arg in PgList::<pg_sys::Node>::from_pg(
                        (*part.clause.cast::<pg_sys::BoolExpr>()).args,
                    )
                    .iter_ptr()
                    {
                        args.push(arg);
                    }
                } else {
                    args.push(part.clause);
                }
            }
        }
        let clause = unsafe { pg_sys::makeBoolExpr(op, args.into_pg(), -1).cast() };
        Estimate { clause, cost }
    }

    fn boolean(
        &self,
        must: Vec<Estimate>,
        should: Vec<Estimate>,
        must_not: Vec<Estimate>,
        minimum: usize,
    ) -> Estimate {
        if must.is_empty() && should.is_empty() {
            return self.prior(0.0, 0.0);
        }
        let minimum = if must.is_empty() {
            minimum.max(1)
        } else {
            minimum
        };
        let optional_cost = if minimum == 0 {
            should.iter().map(|e| e.cost).sum()
        } else {
            0.0
        };
        let mut clauses = must;
        if minimum > 0 {
            clauses.push(self.threshold(should, minimum));
        }
        for negative in must_not {
            clauses.push(self.combine(pg_sys::BoolExprType::NOT_EXPR, vec![negative]));
        }
        let mut result = self.combine(pg_sys::BoolExprType::AND_EXPR, clauses);
        result.cost += optional_cost;
        result
    }

    fn threshold(&self, parts: Vec<Estimate>, minimum: usize) -> Estimate {
        if minimum > parts.len() {
            return self.prior(0.0, 0.0);
        }
        if minimum == 1 {
            return self.combine(pg_sys::BoolExprType::OR_EXPR, parts);
        }
        if minimum == parts.len() {
            return self.combine(pg_sys::BoolExprType::AND_EXPR, parts);
        }
        let cost = parts.iter().map(|e| e.cost).sum();
        let probabilities: Vec<_> = parts.iter().map(|e| self.selectivity(e.clause)).collect();
        // PostgreSQL has no minimum-should-match operator. Assume independent clauses.
        if parts.len().saturating_mul(minimum) > 1_000_000 {
            return self.prior(
                (probabilities.iter().sum::<f64>() / minimum as f64).min(1.0),
                cost,
            );
        }
        let mut counts = vec![0.0; minimum];
        counts[0] = 1.0;
        for p in probabilities {
            for j in (1..minimum).rev() {
                counts[j] = counts[j] * (1.0 - p) + counts[j - 1] * p;
            }
            counts[0] *= 1.0 - p;
        }
        self.prior(1.0 - counts.iter().sum::<f64>(), cost)
    }

    fn input(&self, input: &SearchQueryInput) -> Estimate {
        match input {
            SearchQueryInput::All => self.fallback(1.0),
            SearchQueryInput::Empty => self.prior(0.0, 0.0),
            SearchQueryInput::Uninitialized | SearchQueryInput::PostgresExpression { .. } => {
                self.fallback(crate::PARAMETERIZED_SELECTIVITY)
            }
            SearchQueryInput::Boolean {
                must,
                should,
                must_not,
                minimum_should_match,
            } => self.boolean(
                must.iter().map(|q| self.input(q)).collect(),
                should.iter().map(|q| self.input(q)).collect(),
                must_not.iter().map(|q| self.input(q)).collect(),
                minimum_should_match.unwrap_or(0) as usize,
            ),
            SearchQueryInput::Boost { query, .. }
            | SearchQueryInput::ConstScore { query, .. }
            | SearchQueryInput::WithIndex { query, .. } => self.input(query),
            SearchQueryInput::DisjunctionMax { disjuncts, .. } => self.combine(
                pg_sys::BoolExprType::OR_EXPR,
                disjuncts.iter().map(|q| self.input(q)).collect(),
            ),
            SearchQueryInput::ScoreFilter { query, bounds } => {
                let inner = query
                    .as_ref()
                    .map(|q| self.input(q))
                    .unwrap_or_else(|| self.fallback(1.0));
                let filters = bounds
                    .iter()
                    .map(|(lo, hi)| {
                        self.prior(
                            if matches!((lo, hi), (Bound::Unbounded, Bound::Unbounded)) {
                                1.0
                            } else {
                                range_prior(lo, hi)
                            },
                            0.0,
                        )
                    })
                    .collect();
                let scores = self.combine(pg_sys::BoolExprType::OR_EXPR, filters);
                self.combine(pg_sys::BoolExprType::AND_EXPR, vec![inner, scores])
            }
            SearchQueryInput::MoreLikeThis { .. } => self.fallback(pg_sys::DEFAULT_MATCHING_SEL),
            SearchQueryInput::Parse { .. } => {
                self.tantivy(self.reader.make_query(input, None).as_ref())
            }
            SearchQueryInput::TermSet { terms } => self.combine(
                pg_sys::BoolExprType::OR_EXPR,
                terms
                    .iter()
                    .map(|t| {
                        self.fielded(
                            &t.field,
                            &pdb::Query::Term {
                                value: t.value.clone(),
                            },
                        )
                    })
                    .collect(),
            ),
            SearchQueryInput::HeapFilter {
                indexed_query,
                always_filters,
                recheck_filters,
                ..
            } => {
                let mut parts = vec![self.input(indexed_query)];
                for filter in always_filters.iter().chain(recheck_filters) {
                    parts.push(self.heap_filter(unsafe { filter.get_expression_node() }));
                }
                self.combine(pg_sys::BoolExprType::AND_EXPR, parts)
            }
            SearchQueryInput::FieldedQuery { field, query } => self.fielded(field, query),
        }
    }

    fn heap_filter(&self, mut clause: *mut pg_sys::Node) -> Estimate {
        unsafe {
            if clause.is_null() {
                return self.fallback(1.0);
            }
            if self.standalone {
                let vars = clause.collect_nodes::<pg_sys::Var>();
                if vars.iter().any(|var| (**var).varlevelsup != 0)
                    || vars
                        .windows(2)
                        .any(|pair| (*pair[0]).varno != (*pair[1]).varno)
                {
                    return self.fallback(pg_sys::DEFAULT_MATCH_SEL);
                }
                clause = pg_sys::copyObjectImpl(clause.cast()).cast();
                if let Some(var) = vars.first() {
                    pg_sys::ChangeVarNodes(clause, (**var).varno as _, self.planner.rti as _, 0);
                }
            }
            if (*clause).type_ == pg_sys::NodeTag::T_BoolExpr {
                let boolean = &*clause.cast::<pg_sys::BoolExpr>();
                return self.combine(
                    boolean.boolop,
                    PgList::<pg_sys::Node>::from_pg(boolean.args)
                        .iter_ptr()
                        .map(|node| self.heap_filter(node))
                        .collect(),
                );
            }
            Estimate {
                clause: make_simple_restrictinfo(self.planner.root, clause.cast()).cast(),
                cost: self.reader.total_docs() as f64,
            }
        }
    }

    fn fielded(&self, field: &FieldName, query: &pdb::Query) -> Estimate {
        use pdb::Query as Q;
        match query {
            Q::All => self.fallback(1.0),
            Q::Empty => self.prior(0.0, 0.0),
            Q::ScoreAdjusted { query, .. } => self.fielded(field, query),
            Q::MoreLikeThis { .. } => self.fallback(pg_sys::DEFAULT_MATCHING_SEL),
            Q::Exists => self.exists(field),
            Q::Term { value } if !self.is_text(field) => self
                .comparison(field, "=", value, false)
                .unwrap_or_else(|| self.fallback(pg_sys::DEFAULT_EQ_SEL)),
            Q::TermSet { terms } => self.combine(
                pg_sys::BoolExprType::OR_EXPR,
                terms
                    .iter()
                    .map(|v| self.fielded(field, &Q::Term { value: v.clone() }))
                    .collect(),
            ),
            Q::Range {
                lower_bound,
                upper_bound,
            } => self.range(field, lower_bound, upper_bound),
            Q::RangeContains {
                lower_bound,
                upper_bound,
            } => self.range_relation(field, "@>", lower_bound, upper_bound),
            Q::RangeIntersects {
                lower_bound,
                upper_bound,
            } => self.range_relation(field, "&&", lower_bound, upper_bound),
            Q::RangeWithin {
                lower_bound,
                upper_bound,
            } => self.range_relation(field, "<@", lower_bound, upper_bound),
            Q::RangeTerm { value } => self
                .comparison(field, "@>", value, true)
                .unwrap_or_else(|| self.fallback(pg_sys::DEFAULT_RANGE_INEQ_SEL)),
            Q::FastFieldRangeWeight {
                lower_bound,
                upper_bound,
            } => {
                let decode = |v: &u64| self.fast_value(field, *v);
                match (
                    map_bound(lower_bound, decode),
                    map_bound(upper_bound, decode),
                ) {
                    (Some(lo), Some(hi)) => self.range(field, &lo, &hi),
                    _ => self.fallback(range_prior(lower_bound, upper_bound)),
                }
            }
            Q::Proximity { .. } => self.fallback(pg_sys::DEFAULT_MATCH_SEL),
            Q::UnclassifiedString { .. } | Q::UnclassifiedArray { .. } => {
                self.fallback(pg_sys::DEFAULT_MATCH_SEL)
            }
            Q::Term { .. }
            | Q::FuzzyTerm { .. }
            | Q::Match { .. }
            | Q::MatchArray { .. }
            | Q::Parse { .. }
            | Q::ParseWithField { .. }
            | Q::Phrase { .. }
            | Q::PhraseArray { .. }
            | Q::PhrasePrefix { .. }
            | Q::TokenizedPhrase { .. }
            | Q::Regex { .. }
            | Q::RegexPhrase { .. } => {
                let input = SearchQueryInput::FieldedQuery {
                    field: field.clone(),
                    query: query.clone(),
                };
                self.tantivy(self.reader.make_query(&input, None).as_ref())
            }
        }
    }

    fn is_text(&self, field: &FieldName) -> bool {
        self.reader.schema().search_field(field).is_some_and(|f| {
            matches!(
                f.field_type(),
                SearchFieldType::Text(_)
                    | SearchFieldType::Tokenized(..)
                    | SearchFieldType::Json(_)
            )
        })
    }

    fn tantivy(&self, query: &dyn Query) -> Estimate {
        if let Some(query) = query.downcast_ref::<Box<dyn Query>>() {
            return self.tantivy(query.as_ref());
        }
        if let Some(query) = query.downcast_ref::<BoostQuery>() {
            return self.tantivy(query.query());
        }
        if let Some(query) = query.downcast_ref::<ConstScoreQuery>() {
            return self.tantivy(query.query());
        }
        if let Some(query) = query.downcast_ref::<BooleanQuery>() {
            let mut must = vec![];
            let mut should = vec![];
            let mut must_not = vec![];
            for (occur, query) in query.clauses() {
                let part = self.tantivy(query.as_ref());
                match occur {
                    Occur::Must => must.push(part),
                    Occur::Should => should.push(part),
                    Occur::MustNot => must_not.push(part),
                }
            }
            return self.boolean(
                must,
                should,
                must_not,
                query.get_minimum_number_should_match(),
            );
        }
        if let Some(query) = query.downcast_ref::<DisjunctionMaxQuery>() {
            return self.combine(
                pg_sys::BoolExprType::OR_EXPR,
                query
                    .disjuncts()
                    .iter()
                    .map(|q| self.tantivy(q.as_ref()))
                    .collect(),
            );
        }
        if let Some(query) = query.downcast_ref::<TermSetQuery>() {
            return self.combine(
                pg_sys::BoolExprType::OR_EXPR,
                query
                    .terms()
                    .map(|t| self.tantivy(&TermQuery::new(t.clone(), IndexRecordOption::Basic)))
                    .collect(),
            );
        }
        if let Some(query) = query.downcast_ref::<TermQuery>() {
            let field = self.term_field(query.term());
            if !self.is_text(&field) {
                return self
                    .term_value(query.term())
                    .and_then(|v| self.comparison(&field, "=", &v, false))
                    .unwrap_or_else(|| self.fallback(pg_sys::DEFAULT_EQ_SEL));
            }
        }
        if let Some(query) = query.downcast_ref::<RangeQuery>() {
            let (lo, hi) = query.bounds();
            let field = self
                .reader
                .schema()
                .fields()
                .find(|(f, _)| *f == query.field())
                .map(|(_, f)| FieldName::from(f.name()));
            if let (Some(field), Some(lo), Some(hi)) = (
                field,
                map_bound(lo, |t| self.term_value(t)),
                map_bound(hi, |t| self.term_value(t)),
            ) {
                return self.range(&field, &lo, &hi);
            }
            return self.fallback(range_prior(lo, hi));
        }
        if let Some(query) = query.downcast_ref::<ExistsQuery>() {
            return self.exists(&FieldName::from(query.field_name()));
        }
        if query.is::<tantivy::query::EmptyQuery>() {
            return self.prior(0.0, 0.0);
        }
        if query.is::<tantivy::query::AllQuery>() {
            return self.fallback(1.0);
        }
        let total = self.reader.total_docs();
        let mut covered = 0u64;
        let mut matches = 0.0;
        let mut cost = 0.0;
        for segment in self.reader.searcher().segment_readers() {
            let live = u64::from(segment.num_docs());
            covered += live;
            match query.estimate_docs(segment).ok().flatten() {
                Some((count, work)) => {
                    matches += f64::from(count) / f64::from(segment.max_doc().max(1)) * live as f64;
                    cost += work as f64;
                }
                None => {
                    matches += pg_sys::DEFAULT_MATCH_SEL * live as f64;
                    cost += live as f64;
                }
            }
        }
        let mutable = total.saturating_sub(covered) as f64;
        matches += mutable * pg_sys::DEFAULT_MATCH_SEL;
        cost += mutable;
        self.prior(
            if total == 0 {
                0.0
            } else {
                matches / total as f64
            },
            cost,
        )
    }

    fn term_field(&self, term: &Term) -> FieldName {
        let name = self
            .reader
            .schema()
            .fields()
            .find(|(f, _)| *f == term.field())
            .unwrap()
            .1
            .name()
            .to_owned();
        FieldName::from(match term.get_json_path() {
            Some(path) if !path.is_empty() => format!("{name}.{path}"),
            _ => name,
        })
    }

    fn term_value(&self, term: &Term) -> Option<PdbOwnedValue> {
        let value = term.value();
        let field = self.reader.schema().search_field(&self.term_field(term))?;
        match field.field_type() {
            SearchFieldType::Date(_)
                if self
                    .reader
                    .index_created_by_version()
                    .stores_datetimes_in_i64() =>
            {
                return PostgresDateTime::try_from_raw(value.as_i64()?)
                    .ok()
                    .map(PdbOwnedValue::Date);
            }
            SearchFieldType::Numeric64(_, scale) => {
                return Some(PdbOwnedValue::Str(
                    decimal_bytes::Decimal64NoScale::from_raw(value.as_i64()?)
                        .to_string_with_scale(i32::from(scale)),
                ));
            }
            SearchFieldType::NumericBytes(..) => {
                return Some(PdbOwnedValue::Str(
                    crate::postgres::types_arrow::decimal_bytes_to_anynumeric(
                        value.as_bytes()?,
                        None,
                    )
                    .ok()?
                    .to_string(),
                ));
            }
            _ => {}
        }
        value
            .as_i64()
            .map(PdbOwnedValue::I64)
            .or_else(|| value.as_u64().map(PdbOwnedValue::U64))
            .or_else(|| value.as_f64().map(PdbOwnedValue::F64))
            .or_else(|| value.as_bool().map(PdbOwnedValue::Bool))
            .or_else(|| value.as_str().map(|s| PdbOwnedValue::Str(s.to_owned())))
            .or_else(|| value.as_date().map(PdbOwnedValue::from))
            .or_else(|| value.as_ip_addr().map(PdbOwnedValue::IpAddr))
    }

    fn fast_value(&self, field: &FieldName, value: u64) -> Option<PdbOwnedValue> {
        Some(
            match self.reader.schema().search_field(field)?.field_type() {
                SearchFieldType::I64(_) => PdbOwnedValue::I64(i64::from_u64(value)),
                SearchFieldType::U64(_) => PdbOwnedValue::U64(value),
                SearchFieldType::F64(_) => PdbOwnedValue::F64(f64::from_u64(value)),
                SearchFieldType::Bool(_) => PdbOwnedValue::Bool(bool::from_u64(value)),
                SearchFieldType::Date(_) => {
                    if self
                        .reader
                        .index_created_by_version()
                        .stores_datetimes_in_i64()
                    {
                        PdbOwnedValue::Date(
                            PostgresDateTime::try_from_raw(i64::from_u64(value)).ok()?,
                        )
                    } else {
                        PdbOwnedValue::from(tantivy::DateTime::from_u64(value))
                    }
                }
                SearchFieldType::Numeric64(_, scale) => PdbOwnedValue::Str(
                    decimal_bytes::Decimal64NoScale::from_raw(i64::from_u64(value))
                        .to_string_with_scale(i32::from(scale)),
                ),
                _ => return None,
            },
        )
    }

    fn variable(&self, field: &FieldName) -> Option<*mut pg_sys::Var> {
        if field.path().is_some() {
            return None;
        }
        let fields = self.reader.schema().categorized_fields();
        let (_, data) = fields.iter().find(|(f, _)| f.field_name() == field)?;
        let attno = data.source.heap_attno(self.index)? as i16 + 1;
        unsafe {
            let mut typ = pg_sys::InvalidOid;
            let mut typmod = -1;
            let mut coll = pg_sys::InvalidOid;
            pg_sys::get_atttypetypmodcoll(self.heap.oid(), attno, &mut typ, &mut typmod, &mut coll);
            Some(pg_sys::makeVar(
                self.planner.rti as i32,
                attno,
                typ,
                typmod,
                coll,
                0,
            ))
        }
    }

    fn comparison(
        &self,
        field: &FieldName,
        op: &str,
        value: &PdbOwnedValue,
        range_element: bool,
    ) -> Option<Estimate> {
        let text = value_text(value)?;
        unsafe {
            let var = self.variable(field)?;
            let element = pg_sys::get_element_type((*var).vartype);
            let mut typ = if element != pg_sys::InvalidOid {
                element
            } else {
                (*var).vartype
            };
            if range_element {
                typ = pg_sys::get_range_subtype(typ);
            }
            if typ == pg_sys::InvalidOid {
                return None;
            }
            if element != pg_sys::InvalidOid && op != "=" {
                return None;
            }
            let constant = constant(typ, &text)?;
            let opname = operator_name(op);
            let operator = pg_sys::oper(
                null_mut(),
                opname,
                if element != pg_sys::InvalidOid {
                    typ
                } else {
                    (*var).vartype
                },
                typ,
                true,
                -1,
            );
            if operator.is_null() {
                return None;
            }
            pg_sys::ReleaseSysCache(operator);
            let clause = if element != pg_sys::InvalidOid {
                pg_sys::make_scalar_array_op(
                    null_mut(),
                    opname,
                    true,
                    constant.cast(),
                    var.cast(),
                    -1,
                )
            } else {
                pg_sys::make_op(
                    null_mut(),
                    opname,
                    var.cast(),
                    constant.cast(),
                    null_mut(),
                    -1,
                )
            };
            Some(Estimate {
                clause: make_simple_restrictinfo(self.planner.root, clause).cast(),
                cost: self.reader.total_docs() as f64,
            })
        }
    }

    fn range(
        &self,
        field: &FieldName,
        lo: &Bound<PdbOwnedValue>,
        hi: &Bound<PdbOwnedValue>,
    ) -> Estimate {
        let mut parts = vec![];
        for (bound, inclusive, exclusive) in [(lo, ">=", ">"), (hi, "<=", "<")] {
            let (op, value) = match bound {
                Bound::Included(v) => (inclusive, v),
                Bound::Excluded(v) => (exclusive, v),
                Bound::Unbounded => continue,
            };
            let Some(part) = self.comparison(field, op, value, false) else {
                return self.fallback(range_prior(lo, hi));
            };
            parts.push(part);
        }
        if parts.is_empty() {
            self.exists(field)
        } else {
            self.combine(pg_sys::BoolExprType::AND_EXPR, parts)
        }
    }

    fn range_relation(
        &self,
        field: &FieldName,
        op: &str,
        lo: &Bound<PdbOwnedValue>,
        hi: &Bound<PdbOwnedValue>,
    ) -> Estimate {
        let quote = |b: &Bound<PdbOwnedValue>| match b {
            Bound::Unbounded => Some(String::new()),
            Bound::Included(v) | Bound::Excluded(v) => value_text(v)
                .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))),
        };
        if let (Some(lo_text), Some(hi_text)) = (quote(lo), quote(hi)) {
            let value = PdbOwnedValue::Str(format!(
                "{}{lo_text},{hi_text}{}",
                if matches!(lo, Bound::Included(_)) {
                    '['
                } else {
                    '('
                },
                if matches!(hi, Bound::Included(_)) {
                    ']'
                } else {
                    ')'
                }
            ));
            if let Some(result) = self.comparison(field, op, &value, false) {
                return result;
            }
        }
        self.fallback(if op == "&&" {
            pg_sys::DEFAULT_MATCHING_SEL
        } else {
            pg_sys::DEFAULT_EQ_SEL
        })
    }

    fn exists(&self, field: &FieldName) -> Estimate {
        if let Some(var) = self.variable(field) {
            unsafe {
                // Empty arrays and JSON nulls differ from SQL NULL.
                if pg_sys::get_element_type((*var).vartype) == pg_sys::InvalidOid
                    && !matches!((*var).vartype, pg_sys::JSONOID | pg_sys::JSONBOID)
                {
                    let mut test =
                        PgBox::<pg_sys::NullTest>::alloc_node(pg_sys::NodeTag::T_NullTest);
                    test.arg = var.cast();
                    test.nulltesttype = pg_sys::NullTestType::IS_NOT_NULL;
                    test.location = -1;
                    return Estimate {
                        clause: make_simple_restrictinfo(self.planner.root, test.into_pg().cast())
                            .cast(),
                        cost: self.reader.total_docs() as f64,
                    };
                }
            }
        }
        self.fallback(pg_sys::DEFAULT_NOT_UNK_SEL)
    }
}

fn range_prior<T>(lo: &Bound<T>, hi: &Bound<T>) -> f64 {
    match (lo, hi) {
        (Bound::Unbounded, Bound::Unbounded) => pg_sys::DEFAULT_NOT_UNK_SEL,
        (Bound::Unbounded, _) | (_, Bound::Unbounded) => pg_sys::DEFAULT_INEQ_SEL,
        _ => pg_sys::DEFAULT_RANGE_INEQ_SEL,
    }
}

fn map_bound<T, U>(bound: &Bound<T>, convert: impl FnOnce(&T) -> Option<U>) -> Option<Bound<U>> {
    Some(match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(v) => Bound::Included(convert(v)?),
        Bound::Excluded(v) => Bound::Excluded(convert(v)?),
    })
}

fn value_text(value: &PdbOwnedValue) -> Option<String> {
    Some(match value {
        PdbOwnedValue::Str(v) => v.clone(),
        PdbOwnedValue::U64(v) => v.to_string(),
        PdbOwnedValue::I64(v) => v.to_string(),
        PdbOwnedValue::F64(v) => v.to_string(),
        PdbOwnedValue::Bool(v) => v.to_string(),
        PdbOwnedValue::Date(v) => v.to_rfc3339()?,
        PdbOwnedValue::IpAddr(v) => v.to_string(),
        _ => return None,
    })
}

unsafe fn constant(typ: pg_sys::Oid, value: &str) -> Option<*mut pg_sys::Const> {
    let value = CString::new(value).ok()?;
    let mut input = pg_sys::InvalidOid;
    let mut io = pg_sys::InvalidOid;
    pg_sys::getTypeInputInfo(typ, &mut input, &mut io);
    let datum = pgrx::PgTryBuilder::new(std::panic::AssertUnwindSafe(|| {
        Some(pg_sys::OidInputFunctionCall(
            input,
            value.as_ptr().cast_mut(),
            io,
            -1,
        ))
    }))
    .catch_when(
        pgrx::PgSqlErrorCode::ERRCODE_INVALID_TEXT_REPRESENTATION,
        |_| None,
    )
    .catch_when(
        pgrx::PgSqlErrorCode::ERRCODE_NUMERIC_VALUE_OUT_OF_RANGE,
        |_| None,
    )
    .catch_when(
        pgrx::PgSqlErrorCode::ERRCODE_INVALID_DATETIME_FORMAT,
        |_| None,
    )
    .catch_when(
        pgrx::PgSqlErrorCode::ERRCODE_DATETIME_FIELD_OVERFLOW,
        |_| None,
    )
    .execute()?;
    let mut len = 0;
    let mut byval = false;
    pg_sys::get_typlenbyval(typ, &mut len, &mut byval);
    Some(pg_sys::makeConst(
        typ,
        -1,
        pg_sys::get_typcollation(typ),
        len as i32,
        datum,
        false,
        byval,
    ))
}

unsafe fn operator_name(op: &str) -> *mut pg_sys::List {
    let op = CString::new(op).unwrap();
    let mut list = PgList::<pg_sys::String>::new();
    list.push(pg_sys::makeString(pg_sys::pstrdup(c"pg_catalog".as_ptr())));
    list.push(pg_sys::makeString(pg_sys::pstrdup(op.as_ptr())));
    list.into_pg()
}

pub(crate) fn explain(
    reader: &SearchIndexReader,
    index: &PgSearchRelation,
    query: &SearchQueryInput,
) -> QueryWithEstimates {
    let estimate = estimate(reader, index, query, RowEstimate::Unknown, None);
    let mut children = vec![];
    let label = match query {
        SearchQueryInput::Boolean {
            must,
            should,
            must_not,
            ..
        } => {
            for (label, clauses) in [("Must", must), ("Should", should), ("MustNot", must_not)] {
                for (i, child) in clauses.iter().enumerate() {
                    let child = explain(reader, index, child);
                    let mut wrapper = QueryWithEstimates::with_children(
                        SearchQueryInput::Empty,
                        format!("{label} Clause [{i}]"),
                        vec![child],
                    );
                    wrapper.estimated_docs = wrapper.children[0].estimated_docs;
                    children.push(wrapper);
                }
            }
            "Boolean Query".to_owned()
        }
        SearchQueryInput::Boost { query, factor } => {
            children.push(explain(reader, index, query));
            format!("Boost Query (factor: {factor})")
        }
        SearchQueryInput::ConstScore { query, score } => {
            children.push(explain(reader, index, query));
            format!("ConstScore Query (score: {score})")
        }
        SearchQueryInput::WithIndex { query, .. } => {
            children.push(explain(reader, index, query));
            "WithIndex Query".to_owned()
        }
        SearchQueryInput::ScoreFilter { query, .. } => {
            if let Some(query) = query {
                children.push(explain(reader, index, query));
            }
            "ScoreFilter Query".to_owned()
        }
        SearchQueryInput::DisjunctionMax {
            disjuncts,
            tie_breaker,
        } => {
            for (i, query) in disjuncts.iter().enumerate() {
                let child = explain(reader, index, query);
                let mut wrapper = QueryWithEstimates::with_children(
                    SearchQueryInput::All,
                    format!("Disjunct [{i}]"),
                    vec![child],
                );
                wrapper.estimated_docs = wrapper.children[0].estimated_docs;
                children.push(wrapper);
            }
            tie_breaker
                .map(|value| format!("DisjunctionMax Query (tie_breaker: {value})"))
                .unwrap_or_else(|| "DisjunctionMax Query".to_owned())
        }
        SearchQueryInput::HeapFilter { indexed_query, .. } => {
            children.push(explain(reader, index, indexed_query));
            "HeapFilter Query".to_owned()
        }
        SearchQueryInput::All => "All Query".to_owned(),
        SearchQueryInput::Empty => "Empty Query".to_owned(),
        SearchQueryInput::Uninitialized => "Uninitialized Query".to_owned(),
        SearchQueryInput::MoreLikeThis { .. } => "MoreLikeThis Query".to_owned(),
        SearchQueryInput::Parse { .. } => "Parse Query".to_owned(),
        SearchQueryInput::TermSet { .. } => "TermSet Query".to_owned(),
        SearchQueryInput::PostgresExpression { .. } => "Postgres Expression".to_owned(),
        SearchQueryInput::FieldedQuery { field, .. } => format!("FieldedQuery (field: {field})"),
    };
    let mut tree = QueryWithEstimates::with_children(query.clone(), label, children);
    tree.set_estimate(estimate.matching_docs);
    tree
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::index::reader::index::test_support::segmented_index_fixture;
    use pgrx::{Spi, pg_test};

    fn term(field: &str, value: impl Into<PdbOwnedValue>) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field: field.into(),
            query: pdb::Query::Term {
                value: value.into(),
            },
        }
    }

    fn boolean(
        must: Vec<SearchQueryInput>,
        should: Vec<SearchQueryInput>,
        must_not: Vec<SearchQueryInput>,
        minimum_should_match: Option<i64>,
    ) -> SearchQueryInput {
        SearchQueryInput::Boolean {
            must,
            should,
            must_not,
            minimum_should_match,
        }
    }

    #[pg_test]
    fn metadata_selectivity_uses_every_segment() {
        let (index, _) = segmented_index_fixture("metadata_every_segment", 4, false);
        let query = term("title", "silver".to_owned());
        let reader = SearchIndexReader::open_for_estimation(&index, &query).unwrap();
        assert_eq!(reader.searcher().segment_readers().len(), 4);
        let result = estimate(&reader, &index, &query, RowEstimate::Unknown, None);
        assert_eq!(result.matching_docs, 10);
        assert_eq!(result.selectivity, 0.25);
        assert_eq!(result.query_cost, 10);
        let scaled = estimate(&reader, &index, &query, RowEstimate::Known(400), None);
        assert_eq!(scaled.matching_docs, 100);
        assert_eq!(scaled.selectivity, result.selectivity);
    }

    #[pg_test]
    fn metadata_selectivity_combines_nested_queries() {
        let (index, _) = segmented_index_fixture("metadata_boolean", 4, false);
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        let silver = term("title", "silver".to_owned());
        let quiet = term("title", "quiet".to_owned());
        let cases = [
            (
                boolean(vec![silver.clone()], vec![quiet.clone()], vec![], None),
                0.25,
            ),
            (
                boolean(vec![], vec![silver.clone(), quiet.clone()], vec![], None),
                0.8125,
            ),
            (
                boolean(vec![], vec![silver.clone(), quiet.clone()], vec![], Some(2)),
                0.1875,
            ),
            (
                boolean(
                    vec![SearchQueryInput::All],
                    vec![],
                    vec![silver.clone()],
                    None,
                ),
                0.75,
            ),
            (boolean(vec![], vec![], vec![silver.clone()], None), 0.0),
            (
                SearchQueryInput::DisjunctionMax {
                    disjuncts: vec![silver, quiet],
                    tie_breaker: None,
                },
                0.8125,
            ),
        ];
        for (query, expected) in cases {
            let query = SearchQueryInput::Boost {
                factor: 2.0,
                query: Box::new(SearchQueryInput::ConstScore {
                    score: 3.0,
                    query: Box::new(query),
                }),
            };
            let result = estimate(&reader, &index, &query, RowEstimate::Unknown, None);
            assert!(
                (result.selectivity - expected).abs() < 1e-9,
                "{query:?}: {} != {expected}",
                result.selectivity
            );
        }
        for (conjunction, expected) in [(false, 0.8125), (true, 0.1875)] {
            let input = SearchQueryInput::FieldedQuery {
                field: "title".into(),
                query: pdb::Query::MatchArray {
                    tokens: vec!["silver".to_owned(), "quiet".to_owned()],
                    distance: None,
                    transposition_cost_one: None,
                    prefix: None,
                    conjunction_mode: Some(conjunction),
                },
            };
            let result = estimate(&reader, &index, &input, RowEstimate::Unknown, None);
            assert!((result.selectivity - expected).abs() < 1e-9);
        }
        let parsed = SearchQueryInput::Parse {
            query_string: "(title:silver AND title:quiet)^2".into(),
            lenient: None,
            conjunction_mode: None,
        };
        let parser_reader = SearchIndexReader::open_for_estimation(&index, &parsed).unwrap();
        assert!(
            (estimate(&parser_reader, &index, &parsed, RowEstimate::Unknown, None).selectivity
                - 0.1875)
                .abs()
                < 1e-9
        );
    }

    #[pg_test]
    fn metadata_selectivity_uses_postgres_column_statistics() {
        let (index, _) = segmented_index_fixture("metadata_scalar_stats", 4, false);
        Spi::run("ANALYZE metadata_scalar_stats").unwrap();
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        let equality = term("id", 1i64);
        let query = SearchQueryInput::FieldedQuery {
            field: "id".into(),
            query: pdb::Query::Range {
                lower_bound: Bound::Included(PdbOwnedValue::I64(10)),
                upper_bound: Bound::Included(PdbOwnedValue::I64(20)),
            },
        };
        let eq = estimate(&reader, &index, &equality, RowEstimate::Unknown, None);
        assert!((eq.selectivity - 0.025).abs() < 1e-9, "{}", eq.selectivity);
        let range = estimate(&reader, &index, &query, RowEstimate::Unknown, None);
        assert!(
            (range.selectivity - 0.275).abs() < 0.03,
            "{}",
            range.selectivity
        );
        let mixed = boolean(
            vec![query, term("title", "silver".to_owned())],
            vec![],
            vec![],
            None,
        );
        let result = estimate(&reader, &index, &mixed, RowEstimate::Unknown, None);
        assert!((result.selectivity - range.selectivity * 0.25).abs() < 1e-9);
    }

    #[pg_test]
    fn metadata_selectivity_has_defaults_without_statistics() {
        let (index, _) = segmented_index_fixture("metadata_missing_stats", 1, false);
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        let cases = [
            (term("id", 3i64), pg_sys::DEFAULT_EQ_SEL),
            (
                SearchQueryInput::FieldedQuery {
                    field: "id".into(),
                    query: pdb::Query::Range {
                        lower_bound: Bound::Included(PdbOwnedValue::I64(3)),
                        upper_bound: Bound::Unbounded,
                    },
                },
                pg_sys::DEFAULT_INEQ_SEL,
            ),
            (
                SearchQueryInput::FieldedQuery {
                    field: "id".into(),
                    query: pdb::Query::Range {
                        lower_bound: Bound::Included(PdbOwnedValue::I64(3)),
                        upper_bound: Bound::Included(PdbOwnedValue::I64(8)),
                    },
                },
                pg_sys::DEFAULT_RANGE_INEQ_SEL,
            ),
            (
                SearchQueryInput::FieldedQuery {
                    field: "id".into(),
                    query: pdb::Query::MoreLikeThis {
                        key_value: PdbOwnedValue::I64(-999),
                        fields: None,
                        options: Default::default(),
                    },
                },
                pg_sys::DEFAULT_MATCHING_SEL,
            ),
        ];
        for (query, expected) in cases {
            let result = estimate(&reader, &index, &query, RowEstimate::Unknown, None);
            assert!(
                (result.selectivity - expected).abs() < 1e-9,
                "{query:?}: {} != {expected}",
                result.selectivity
            );
        }
    }

    #[pg_test]
    fn metadata_selectivity_never_builds_a_weight() {
        #[derive(Clone, Debug)]
        struct MetadataOnly(bool);
        impl tantivy::query::QueryEstimate for MetadataOnly {
            fn estimate_docs(
                &self,
                _: &tantivy::SegmentReader,
            ) -> tantivy::Result<Option<(u32, u64)>> {
                Ok(self.0.then_some((2, 7)))
            }
        }
        impl Query for MetadataOnly {
            fn weight(
                &self,
                _: tantivy::query::EnableScoring<'_>,
            ) -> tantivy::Result<Box<dyn tantivy::query::Weight>> {
                panic!("estimation must not construct a weight")
            }
        }
        let (index, _) = segmented_index_fixture("metadata_no_scorer", 2, false);
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        let mut memory = PgMemoryContexts::new("estimate test");
        unsafe {
            memory.switch_to(|_| {
                let estimator = Estimator::new(&reader, &index, None);
                for (query, expected) in [
                    (MetadataOnly(true), 0.2),
                    (MetadataOnly(false), pg_sys::DEFAULT_MATCH_SEL),
                ] {
                    let result = estimator.tantivy(&query);
                    assert!((estimator.selectivity(result.clause) - expected).abs() < 1e-9);
                }
            });
        }
        Spi::run("SET paradedb.enable_heuristic_selectivity = off").unwrap();
        let mlt = SearchQueryInput::FieldedQuery {
            field: "id".into(),
            query: pdb::Query::MoreLikeThis {
                key_value: PdbOwnedValue::I64(-999),
                fields: None,
                options: Default::default(),
            },
        };
        assert_eq!(
            crate::api::operator::estimate_selectivity_and_cost(&index, mlt.clone(), None).0,
            Some(0.01)
        );
        assert!(
            reader
                .build_query_tree_with_estimates(mlt)
                .unwrap()
                .estimated_docs
                .is_some()
        );
    }

    #[pg_test]
    fn metadata_selectivity_matches_postgres_for_scalar_types() {
        Spi::run(r#"
            SET paradedb.global_mutable_segment_rows = 0;
            CREATE TABLE metadata_types (id bigint PRIMARY KEY, price int, amount numeric(12,2), huge numeric,
                day date, tags int[], period int4range, optional bigint, mirror int);
            INSERT INTO metadata_types SELECT g, g % 10, g / 10.0, g / 10.0,
                '2020-01-01'::date + g, ARRAY[g % 2], int4range(g, g+10), CASE WHEN g % 5 != 0 THEN g END, g % 10
                FROM generate_series(1, 1000) g;
            CREATE INDEX metadata_types_idx ON metadata_types USING paradedb
                (id, price, amount, huge, day, tags, period, optional, mirror)
                WITH (numeric_fields = '{"price":{"indexed":false}}');
            CREATE STATISTICS metadata_types_stats (mcv, dependencies) ON price, mirror FROM metadata_types;
            ANALYZE metadata_types;
            SET paradedb.enable_custom_scan = off;
        "#).unwrap();
        let index = crate::index::reader::index::test_support::open_index("metadata_types_idx");
        let reader =
            SearchIndexReader::open_for_estimation(&index, &SearchQueryInput::All).unwrap();
        assert!(
            !reader
                .schema()
                .search_field("price")
                .unwrap()
                .field_entry()
                .is_indexed()
        );
        let int_bound = |v| Bound::Included(PdbOwnedValue::I64(v));
        let string_bound = |v: &str| Bound::Included(PdbOwnedValue::Str(v.to_owned()));
        let cases = [
            (
                "price",
                pdb::Query::Term {
                    value: PdbOwnedValue::I64(3),
                },
                "price = 3",
            ),
            (
                "price",
                pdb::Query::Range {
                    lower_bound: int_bound(3),
                    upper_bound: int_bound(6),
                },
                "price >= 3 AND price <= 6",
            ),
            (
                "amount",
                pdb::Query::Range {
                    lower_bound: string_bound("10.50"),
                    upper_bound: string_bound("40.50"),
                },
                "amount >= 10.50 AND amount <= 40.50",
            ),
            (
                "huge",
                pdb::Query::Range {
                    lower_bound: string_bound("10.50"),
                    upper_bound: string_bound("40.50"),
                },
                "huge >= 10.50 AND huge <= 40.50",
            ),
            (
                "day",
                pdb::Query::Range {
                    lower_bound: string_bound("2020-02-01"),
                    upper_bound: string_bound("2020-03-01"),
                },
                "day >= '2020-02-01' AND day <= '2020-03-01'",
            ),
            (
                "tags",
                pdb::Query::Term {
                    value: PdbOwnedValue::I64(1),
                },
                "1 = ANY(tags)",
            ),
            (
                "period",
                pdb::Query::RangeIntersects {
                    lower_bound: int_bound(100),
                    upper_bound: Bound::Excluded(PdbOwnedValue::I64(200)),
                },
                "period && '[100,200)'::int4range",
            ),
            (
                "period",
                pdb::Query::RangeContains {
                    lower_bound: int_bound(100),
                    upper_bound: Bound::Excluded(PdbOwnedValue::I64(105)),
                },
                "period @> '[100,105)'::int4range",
            ),
            (
                "period",
                pdb::Query::RangeWithin {
                    lower_bound: int_bound(100),
                    upper_bound: Bound::Excluded(PdbOwnedValue::I64(200)),
                },
                "period <@ '[100,200)'::int4range",
            ),
            (
                "period",
                pdb::Query::RangeTerm {
                    value: PdbOwnedValue::I64(100),
                },
                "period @> 100",
            ),
            ("optional", pdb::Query::Exists, "optional IS NOT NULL"),
        ];
        for (field, query, predicate) in cases {
            let query = SearchQueryInput::FieldedQuery {
                field: field.into(),
                query,
            };
            let result = estimate(&reader, &index, &query, RowEstimate::Known(1000), None);
            let plan = Spi::get_one::<pgrx::Json>(&format!(
                "EXPLAIN (FORMAT JSON) SELECT * FROM metadata_types WHERE {predicate}"
            ))
            .unwrap()
            .unwrap()
            .0;
            let native = plan[0]["Plan"]["Plan Rows"].as_f64().unwrap();
            assert!(
                (result.selectivity * 1000.0 - native).abs() <= 1.0,
                "{predicate}: metadata={} PostgreSQL={native}",
                result.selectivity * 1000.0
            );
        }
        Spi::run("SET paradedb.enable_custom_scan = on; SET enable_indexscan = off; SET max_parallel_workers_per_gather = 0;").unwrap();
        let plan = Spi::get_one::<pgrx::Json>("EXPLAIN (FORMAT JSON) SELECT * FROM metadata_types WHERE id @@@ paradedb.range('price', int4range(3, 7))")
            .unwrap().unwrap().0;
        assert_eq!(plan[0]["Plan"]["Plan Rows"].as_u64(), Some(400), "{plan}");

        let correlated = Spi::get_one::<pgrx::Json>("EXPLAIN (FORMAT JSON) SELECT * FROM metadata_types WHERE id @@@ paradedb.boolean(must := ARRAY[paradedb.range('price', int4range(3, 4)), paradedb.term('mirror', 3)])")
            .unwrap().unwrap().0;
        assert_eq!(
            correlated[0]["Plan"]["Plan Rows"].as_u64(),
            Some(100),
            "{correlated}"
        );

        let mut memory = PgMemoryContexts::new("heap estimate test");
        unsafe {
            memory.switch_to(|_| {
                let estimator = Estimator::new(&reader, &index, None);
                let var = pg_sys::makeVar(8, 2, pg_sys::INT4OID, -1, pg_sys::InvalidOid, 0);
                let clause = pg_sys::make_op(
                    null_mut(),
                    operator_name("="),
                    var.cast(),
                    constant(pg_sys::INT4OID, "3").unwrap().cast(),
                    null_mut(),
                    -1,
                );
                let result = estimator.heap_filter(clause.cast());
                assert!((estimator.selectivity(result.clause) - 0.1).abs() < 1e-6);
                assert!(constant(pg_sys::INT4OID, "999999999999999").is_none());
                assert!(constant(pg_sys::UUIDOID, "not-a-uuid").is_none());
            });
        }
    }
}
