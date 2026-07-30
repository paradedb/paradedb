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

use crate::nodecast;
use crate::postgres::customscan::score_funcoids;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{extension_sql, pg_extern, pg_guard, pg_sys, AnyElement, FromDatum, PgList};
use std::ptr::addr_of_mut;

#[pgrx::pg_schema]
mod pdb {
    use pgrx::{default, extension_sql, pg_extern, AnyElement};

    #[allow(unused_variables)]
    #[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
    fn score_from_relation(relation_reference: AnyElement) -> f32 {
        panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
    }

    // `pdb.score` is overloaded (see `score_from_relation_typed`), so the
    // ALTER must name the exact signature.
    extension_sql!(
        r#"
    ALTER FUNCTION pdb.score(anyelement) SUPPORT paradedb.placeholder_support;
    "#,
        name = "score_placeholder",
        requires = [score_from_relation, placeholder_support]
    );

    /// `pdb.score(relation, type)` — typed score projection. The score type is
    /// one of `'bm25'`, `'vector'`, or `'hybrid'`; see
    /// [`super::ScoreKind`]. Deliberately has NO DEFAULT on the second
    /// argument: a defaulted overload would make plain `pdb.score(id)` calls
    /// ambiguous against the single-argument function.
    #[allow(unused_variables)]
    #[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
    fn score_from_relation_typed(relation_reference: AnyElement, score_type: String) -> f32 {
        panic!("pdb.score(relation, type) is only supported on queries executed by a ParadeDB custom scan. Please report unexpected cases at https://github.com/paradedb/paradedb/issues/new/choose");
    }

    // Rename the second parameter to `type` so callers can write
    // `pdb.score(id, type => 'hybrid')`. `type` is a Rust keyword and pgrx
    // renders `r#type` verbatim into SQL, so the pgrx-generated function is
    // dropped and recreated with the desired parameter name (Postgres
    // forbids renaming a named input parameter via CREATE OR REPLACE).
    extension_sql!(
        r#"
    DROP FUNCTION pdb."score"(anyelement, text);
    CREATE FUNCTION pdb."score"(
        "relation_reference" anyelement,
        "type" text
    ) RETURNS real
    STRICT STABLE PARALLEL SAFE COST 1
    LANGUAGE c
    AS 'MODULE_PATHNAME', 'score_from_relation_typed_wrapper';
    ALTER FUNCTION pdb.score(anyelement, text) SUPPORT paradedb.placeholder_support;
    "#,
        name = "score_typed_placeholder",
        requires = [score_from_relation_typed, placeholder_support]
    );

    /// `pdb.rrf(bm25_score, vector_distance, k, window_size)` — Reciprocal
    /// Rank Fusion of a BM25 ranking and a vector-distance ranking. Only
    /// meaningful as a TopK ORDER BY expression, e.g.
    /// `ORDER BY pdb.rrf(pdb.score(id), embedding <=> '[...]') LIMIT 10`.
    /// Returns the fused rank (1 = best), so ascending ORDER BY — the
    /// pgvector convention — puts the best match first.
    ///
    /// See also the fully-implied relation form `pdb.rrf(relation, k,
    /// window_size)` below, where both legs come from the WHERE clause.
    /// The distance leg here must NOT have a DEFAULT: a defaulted overload
    /// would make one-argument calls ambiguous against the relation form.
    /// (The function is deliberately NOT STRICT so a literal NULL leg still
    /// reaches the placeholder body and errors loudly when the query is not
    /// executed by a rank-fusion scan, instead of silently sorting by NULL.)
    ///
    /// `window_size` is the per-leg candidate pool (the overfetch): each leg
    /// contributes its top-`window_size` documents to the fusion. `0` (the
    /// default) auto-sizes it to `max(4 * (LIMIT + OFFSET), 100)`; explicit
    /// values are floored at `LIMIT + OFFSET` so the page cannot be
    /// truncated. (Named `window_size` rather than `window` because WINDOW
    /// is a reserved keyword and could not be used in `=>` notation.)
    ///
    /// Both ranking legs are `float8` so the legs can be written in either
    /// order: `pdb.score(...)` is `real` and casts up implicitly, while a
    /// `float8` distance could never cast down to a `real` parameter.
    ///
    /// No `placeholder_support` here: the call contains Vars for both the key
    /// column and the vector column, which the support function's
    /// single-`Var` PlaceHolderVar wrapping cannot represent.
    #[allow(unused_variables)]
    #[pg_extern(name = "rrf", stable, parallel_safe, cost = 1)]
    fn rrf_placeholder(
        bm25_score: Option<f64>,
        vector_distance: Option<f64>,
        k: default!(i32, 60),
        window_size: default!(i32, 0),
    ) -> f64 {
        panic!("pdb.rrf() requires a ParadeDB TopK scan: use `ORDER BY pdb.rrf(pdb.score(<key>), <vector_column> <op> <query_vector>) LIMIT <n>` on a table with a bm25-indexed vector column");
    }

    /// `pdb.rrf(relation, k, window_size)` — the fully-implied form of rank
    /// fusion: the BM25 leg is the WHERE clause's text query and the vector
    /// leg is its `~~~` knn predicate, e.g.
    ///
    /// ```sql
    /// WHERE description ||| 'shoes' OR embedding ~~~ '[...]'
    /// ORDER BY pdb.rrf(id)
    /// LIMIT 10;
    /// ```
    ///
    /// The relation reference plays the same role as in `pdb.score(id)`.
    #[allow(unused_variables)]
    #[pg_extern(name = "rrf", stable, parallel_safe, cost = 1)]
    fn rrf_from_relation(
        relation_reference: AnyElement,
        k: default!(i32, 60),
        window_size: default!(i32, 0),
    ) -> f64 {
        panic!("pdb.rrf() requires a ParadeDB TopK scan: use `ORDER BY pdb.rrf(<key>) LIMIT <n>` with a `~~~` predicate in the WHERE clause");
    }
}

/// The score type requested by a `pdb.score(relation, type)` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ScoreKind {
    /// The BM25 score of the row against the query's search predicate.
    Bm25 = 0,
    /// The vector similarity of the row against the query's vector ranking
    /// (the `pdb.rrf()` distance leg, or a vector-distance ORDER BY).
    Vector = 1,
    /// The Reciprocal Rank Fusion score of the BM25 and vector rankings —
    /// the positive fused score whose rank `pdb.rrf()` sorts by.
    Hybrid = 2,
    /// The 1-based fused rank — the same value `pdb.rrf()` projects.
    Rank = 3,
}

impl ScoreKind {
    pub const ALL: [ScoreKind; 4] = [
        ScoreKind::Bm25,
        ScoreKind::Vector,
        ScoreKind::Hybrid,
        ScoreKind::Rank,
    ];

    pub fn from_type_name(name: &str) -> Option<Self> {
        match name {
            "bm25" => Some(ScoreKind::Bm25),
            "vector" => Some(ScoreKind::Vector),
            "hybrid" => Some(ScoreKind::Hybrid),
            "rank" => Some(ScoreKind::Rank),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ScoreKind::Bm25 => "bm25",
            ScoreKind::Vector => "vector",
            ScoreKind::Hybrid => "hybrid",
            ScoreKind::Rank => "rank",
        }
    }
}

/// The set of `pdb.score(relation, type)` score types appearing in a query's
/// target list.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScoreKindSet([bool; 4]);

impl ScoreKindSet {
    pub fn insert(&mut self, kind: ScoreKind) {
        self.0[kind as usize] = true;
    }

    pub fn contains(&self, kind: ScoreKind) -> bool {
        self.0[kind as usize]
    }

    pub fn has_any(&self) -> bool {
        self.0.iter().any(|b| *b)
    }
}

/// Oid of the `pdb.score(anyelement, text)` typed-projection overload.
pub fn typed_score_funcoid() -> pg_sys::Oid {
    crate::postgres::utils::lookup_pdb_function("score", &[pg_sys::ANYELEMENTOID, pg_sys::TEXTOID])
}

/// Oids of the two `pdb.rrf` overloads: the explicit-legs form
/// `pdb.rrf(float8, float8, int, int)` and the fully-implied relation form
/// `pdb.rrf(anyelement, int, int)`.
pub fn rrf_funcoids() -> [pg_sys::Oid; 2] {
    [
        crate::postgres::utils::lookup_pdb_function(
            "rrf",
            &[
                pg_sys::FLOAT8OID,
                pg_sys::FLOAT8OID,
                pg_sys::INT4OID,
                pg_sys::INT4OID,
            ],
        ),
        crate::postgres::utils::lookup_pdb_function(
            "rrf",
            &[pg_sys::ANYELEMENTOID, pg_sys::INT4OID, pg_sys::INT4OID],
        ),
    ]
}

/// True when `funcid` is one of the (existing) `pdb.rrf` overloads.
pub fn is_rrf_funcoid(funcid: pg_sys::Oid) -> bool {
    funcid != pg_sys::Oid::INVALID && rrf_funcoids().contains(&funcid)
}

/// Extract the score type argument from a `pdb.score(relation, type)` call.
///
/// Returns `None` when the second argument is not a constant (e.g. a Var or
/// arbitrary expression). Raises an ERROR when the constant is NULL or not a
/// recognized score type.
pub unsafe fn typed_score_kind_from_funcexpr(funcexpr: *mut pg_sys::FuncExpr) -> Option<ScoreKind> {
    let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
    if args.len() != 2 {
        return None;
    }
    let const_ = nodecast!(Const, T_Const, args.get_ptr(1)?)?;
    if (*const_).constisnull {
        panic!("pdb.score: the score type must not be NULL");
    }
    let type_name = String::from_datum((*const_).constvalue, false)
        .expect("pdb.score: should be able to read the score type argument");
    match ScoreKind::from_type_name(&type_name) {
        Some(kind) => Some(kind),
        None => panic!(
            "pdb.score: unrecognized score type '{type_name}'; expected 'bm25', 'vector', 'hybrid', or 'rank'"
        ),
    }
}

/// Walk `node` collecting the score types of all `pdb.score(<var>, <type>)`
/// calls whose relation reference belongs to `rti`.
///
/// Raises an ERROR if a matching call's score type argument is not a constant.
pub unsafe fn collect_typed_score_kinds(
    node: *mut pg_sys::Node,
    typed_score_funcoid: pg_sys::Oid,
    rti: pg_sys::Index,
) -> ScoreKindSet {
    if typed_score_funcoid == pg_sys::Oid::INVALID {
        return ScoreKindSet::default();
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let data_ref = &mut *data.cast::<Data>();
            if (*funcexpr).funcid == data_ref.typed_score_funcoid {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                if let Some(var) = args.get_ptr(0).and_then(|arg| nodecast!(Var, T_Var, arg)) {
                    if (*var).varno as i32 == data_ref.rti as i32 {
                        match typed_score_kind_from_funcexpr(funcexpr) {
                            Some(kind) => data_ref.kinds.insert(kind),
                            None => panic!("pdb.score: the score type must be a constant"),
                        }
                    }
                }
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        typed_score_funcoid: pg_sys::Oid,
        rti: pg_sys::Index,
        kinds: ScoreKindSet,
    }

    let mut data = Data {
        typed_score_funcoid,
        rti,
        kinds: ScoreKindSet::default(),
    };

    walker(node, addr_of_mut!(data).cast());
    data.kinds
}

/// Walk `node` looking for a `pdb.rrf(...)` call containing any `Var`
/// belonging to `rti` (the relation reference, score leg, and distance leg
/// all qualify).
///
/// Raises an ERROR when two structurally different `pdb.rrf(...)` calls
/// appear: all calls share one per-row placeholder value (the ORDER BY's
/// fused rank), so distinct calls would silently project the wrong value.
pub unsafe fn uses_rrf(
    node: *mut pg_sys::Node,
    rrf_funcoids: [pg_sys::Oid; 2],
    rti: pg_sys::Index,
) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn contains_rti_var(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        if let Some(var) = nodecast!(Var, T_Var, node) {
            let rti = *data.cast::<pg_sys::Index>();
            if (*var).varno as i32 == rti as i32 {
                return true;
            }
        }
        expression_tree_walker(node, Some(contains_rti_var), data)
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let data_ref = &mut *data.cast::<Data>();
            if (*funcexpr).funcid != pg_sys::Oid::INVALID
                && data_ref.rrf_funcoids.contains(&(*funcexpr).funcid)
                && contains_rti_var(node, addr_of_mut!(data_ref.rti).cast())
            {
                if data_ref.first.is_null() {
                    data_ref.first = node;
                } else if !pg_sys::equal(data_ref.first.cast(), node.cast()) {
                    panic!("pdb.rrf: all pdb.rrf() calls in a query must be identical (same legs and k); project the fused rank once and reuse it");
                }
                // don't descend: a pdb.rrf() call cannot contain another
                return false;
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        rrf_funcoids: [pg_sys::Oid; 2],
        rti: pg_sys::Index,
        first: *mut pg_sys::Node,
    }

    let mut data = Data {
        rrf_funcoids,
        rti,
        first: std::ptr::null_mut(),
    };
    walker(node, addr_of_mut!(data).cast());
    !data.first.is_null()
}

// In `0.19.0`, we renamed the schema from `paradedb` to `pdb`.
// This is a backwards compatibility shim to ensure that old queries continue to work.
#[warn(deprecated)]
#[allow(unused_variables)]
#[pg_extern(name = "score", stable, parallel_safe, cost = 1)]
fn paradedb_score_from_relation(relation_reference: AnyElement) -> Option<f32> {
    panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
}

extension_sql!(
    r#"
    ALTER FUNCTION paradedb.score SUPPORT paradedb.placeholder_support;
    "#,
    name = "paradedb_score_placeholder",
    requires = [paradedb_score_from_relation, placeholder_support]
);

pub unsafe fn uses_scores(
    node: *mut pg_sys::Node,
    score_funcoids: [pg_sys::Oid; 2],
    rti: pg_sys::Index,
) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let data = data.cast::<Data>();
            if (*data).score_funcoids.contains(&(*funcexpr).funcid) {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                assert!(args.len() == 1, "score function must have 1 argument");
                if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                    if (*var).varno as i32 == (*data).rti as i32 {
                        return true;
                    }
                }
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        score_funcoids: [pg_sys::Oid; 2],
        rti: pg_sys::Index,
    }

    let mut data = Data {
        score_funcoids,
        rti,
    };

    walker(node, addr_of_mut!(data).cast())
}

pub unsafe fn is_score_func(node: *mut pg_sys::Node, rti: pg_sys::Index) -> bool {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if score_funcoids().contains(&(*funcexpr).funcid) {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            assert!(args.len() == 1, "score function must have 1 argument");
            if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                if (*var).varno as i32 == rti as i32 {
                    return true;
                }
            }
        }
    }

    false
}
