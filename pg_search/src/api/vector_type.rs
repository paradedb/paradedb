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
#![allow(unsafe_op_in_unsafe_fn)]

use crate::vector::{MAX_VECTOR_DIMENSIONS, PgVector, invalid_vector};
use pgrx::{Array, Internal, IntoDatum, direct_function_call, extension_sql, pg_extern, pg_sys};
use std::cmp::Ordering;
use std::ffi::{CStr, CString};

extension_sql!(
    "CREATE SCHEMA IF NOT EXISTS pdb; CREATE TYPE pdb.vector;",
    name = "pdb_vector_shell"
);

#[pg_extern(immutable, parallel_safe)]
fn vector_in(input: &CStr, _typoid: pg_sys::Oid, typmod: i32) -> PgVector {
    let input = input.to_str().expect("vector input must be UTF-8").trim();
    let elements = input
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or_else(|| invalid_vector("vector must start with '[' and end with ']'"));
    let mut values = Vec::new();
    for element in elements.split(',') {
        if values.len() >= MAX_VECTOR_DIMENSIONS {
            invalid_vector(format!(
                "vector cannot have more than {MAX_VECTOR_DIMENSIONS} dimensions"
            ));
        }
        let element = CString::new(element.trim()).expect("vector element must not contain NUL");
        let value =
            unsafe { direct_function_call::<f32>(pg_sys::float4in, &[element.into_datum()]) }
                .expect("vector element must not be NULL");
        values.push(value);
    }
    let vector = PgVector(values);
    vector.validate(typmod);
    vector
}

#[pg_extern(immutable, parallel_safe)]
fn vector_out(input: PgVector) -> CString {
    CString::new(format!(
        "[{}]",
        input
            .0
            .iter()
            .map(f32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
    .expect("vector output must not contain NUL")
}

#[pg_extern(immutable, parallel_safe)]
fn vector_typmod_in(parts: Array<&CStr>) -> i32 {
    if parts.len() != 1 {
        invalid_vector("vector requires exactly one dimension modifier");
    }
    let dimensions = parts
        .get(0)
        .flatten()
        .and_then(|part| part.to_str().ok())
        .and_then(|part| part.parse::<usize>().ok())
        .filter(|&dimensions| (1..=MAX_VECTOR_DIMENSIONS).contains(&dimensions))
        .unwrap_or_else(|| {
            invalid_vector(format!(
                "vector dimensions must be between 1 and {MAX_VECTOR_DIMENSIONS}"
            ))
        });
    dimensions as i32
}

#[pg_extern(immutable, parallel_safe)]
fn vector_typmod_out(typmod: i32) -> CString {
    CString::new(if typmod < 0 {
        String::new()
    } else {
        format!("({typmod})")
    })
    .unwrap()
}

#[pg_extern(immutable, parallel_safe)]
unsafe fn vector_recv(mut input: Internal, _typoid: pg_sys::Oid, typmod: i32) -> PgVector {
    let input = input
        .get_mut::<pg_sys::StringInfoData>()
        .expect("vector input must not be NULL");
    let dimensions = pg_sys::pq_getmsgint(input, size_of::<i16>() as _) as usize;
    let reserved = pg_sys::pq_getmsgint(input, size_of::<i16>() as _);
    if !(1..=MAX_VECTOR_DIMENSIONS).contains(&dimensions) || reserved != 0 {
        invalid_vector("invalid binary vector header");
    }
    let vector = PgVector(
        (0..dimensions)
            .map(|_| pg_sys::pq_getmsgfloat4(input))
            .collect(),
    );
    pg_sys::pq_getmsgend(input);
    vector.validate(typmod);
    vector
}

#[pg_extern(immutable, parallel_safe)]
fn vector_send(input: PgVector) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 * size_of::<i16>() + input.0.len() * size_of::<f32>());
    bytes.extend_from_slice(&(input.0.len() as i16).to_be_bytes());
    bytes.extend_from_slice(&0_i16.to_be_bytes());
    for value in input.0 {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    bytes
}

extension_sql!(
    r#"
CREATE TYPE pdb.vector (
    INPUT = paradedb.vector_in,
    OUTPUT = paradedb.vector_out,
    RECEIVE = paradedb.vector_recv,
    SEND = paradedb.vector_send,
    TYPMOD_IN = paradedb.vector_typmod_in,
    TYPMOD_OUT = paradedb.vector_typmod_out,
    INTERNALLENGTH = VARIABLE,
    ALIGNMENT = int4,
    STORAGE = external
);
"#,
    name = "pdb_vector_type",
    creates = [Type(PgVector)],
    requires = [
        "pdb_vector_shell",
        vector_in,
        vector_out,
        vector_recv,
        vector_send,
        vector_typmod_in,
        vector_typmod_out
    ]
);

#[pg_extern(immutable, parallel_safe)]
fn vector_coerce(input: PgVector, typmod: i32, _explicit: bool) -> PgVector {
    input.validate(typmod);
    input
}

#[pg_extern(immutable, parallel_safe)]
fn float4_array_to_vector(input: Array<f32>, typmod: i32, _explicit: bool) -> PgVector {
    let vector = PgVector(
        input
            .iter()
            .map(|value| {
                value.unwrap_or_else(|| invalid_vector("vector elements must not be NULL"))
            })
            .collect(),
    );
    if unsafe { (*input.into_array_type()).ndim } != 1 {
        invalid_vector("vector array must be one-dimensional");
    }
    vector.validate(typmod);
    vector
}

#[pg_extern(immutable, parallel_safe)]
fn float8_array_to_vector(input: Array<f64>, typmod: i32, _explicit: bool) -> PgVector {
    let vector = PgVector(
        input
            .iter()
            .map(|value| {
                value.unwrap_or_else(|| invalid_vector("vector elements must not be NULL")) as f32
            })
            .collect(),
    );
    if unsafe { (*input.into_array_type()).ndim } != 1 {
        invalid_vector("vector array must be one-dimensional");
    }
    vector.validate(typmod);
    vector
}

#[pg_extern(immutable, parallel_safe)]
fn vector_to_float4_array(input: PgVector) -> Vec<f32> {
    input.0
}

#[pg_extern(immutable, parallel_safe)]
fn vector_dims(input: PgVector) -> i32 {
    input.0.len() as i32
}

fn matching_dimensions(left: &PgVector, right: &PgVector) {
    if left.0.len() != right.0.len() {
        invalid_vector(format!(
            "vector dimensions differ: {} and {}",
            left.0.len(),
            right.0.len()
        ));
    }
}

#[pg_extern(immutable, parallel_safe)]
fn vector_l2_distance(left: PgVector, right: PgVector) -> f64 {
    matching_dimensions(&left, &right);
    let squared: f32 = left
        .0
        .iter()
        .zip(&right.0)
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    f64::from(squared).sqrt()
}

#[pg_extern(immutable, parallel_safe)]
fn vector_negative_inner_product(left: PgVector, right: PgVector) -> f64 {
    matching_dimensions(&left, &right);
    -f64::from(left.dot(&right.0))
}

#[pg_extern(immutable, parallel_safe)]
fn vector_cosine_distance(left: PgVector, right: PgVector) -> f64 {
    matching_dimensions(&left, &right);
    let mut dot = 0.0_f64;
    let mut norm_left = 0.0_f64;
    let mut norm_right = 0.0_f64;
    for (&a, &b) in left.0.iter().zip(&right.0) {
        let (a, b) = (f64::from(a), f64::from(b));
        dot += a * b;
        norm_left += a * a;
        norm_right += b * b;
    }
    1.0 - (dot / (norm_left * norm_right).sqrt()).clamp(-1.0, 1.0)
}

fn compare(left: &PgVector, right: &PgVector) -> Ordering {
    left.0
        .iter()
        .zip(&right.0)
        .map(|(a, b)| a.partial_cmp(b).expect("vector elements must be finite"))
        .find(|&ordering| ordering != Ordering::Equal)
        .unwrap_or_else(|| left.0.len().cmp(&right.0.len()))
}

#[pg_extern(immutable, parallel_safe)]
fn vector_cmp(left: PgVector, right: PgVector) -> i32 {
    match compare(&left, &right) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

macro_rules! comparison {
    ($name:ident, $test:expr) => {
        #[pg_extern(immutable, parallel_safe)]
        fn $name(left: PgVector, right: PgVector) -> bool {
            ($test)(compare(&left, &right))
        }
    };
}
comparison!(vector_lt, |order: Ordering| order.is_lt());
comparison!(vector_le, |order: Ordering| order.is_le());
comparison!(vector_eq, |order: Ordering| order.is_eq());
comparison!(vector_ne, |order: Ordering| order.is_ne());
comparison!(vector_ge, |order: Ordering| order.is_ge());
comparison!(vector_gt, |order: Ordering| order.is_gt());

extension_sql!(
    r#"
CREATE CAST (pdb.vector AS pdb.vector)
    WITH FUNCTION paradedb.vector_coerce(pdb.vector, integer, boolean) AS IMPLICIT;
CREATE CAST (real[] AS pdb.vector)
    WITH FUNCTION paradedb.float4_array_to_vector(real[], integer, boolean) AS ASSIGNMENT;
CREATE CAST (double precision[] AS pdb.vector)
    WITH FUNCTION paradedb.float8_array_to_vector(double precision[], integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.vector AS real[])
    WITH FUNCTION paradedb.vector_to_float4_array(pdb.vector);
CREATE OPERATOR pg_catalog.<-> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_l2_distance, COMMUTATOR = OPERATOR(pg_catalog.<->));
CREATE OPERATOR pg_catalog.<#> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_negative_inner_product, COMMUTATOR = OPERATOR(pg_catalog.<#>));
CREATE OPERATOR pg_catalog.<=> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_cosine_distance, COMMUTATOR = OPERATOR(pg_catalog.<=>));
CREATE OPERATOR pg_catalog.< (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_lt, COMMUTATOR = OPERATOR(pg_catalog.>), NEGATOR = OPERATOR(pg_catalog.>=));
CREATE OPERATOR pg_catalog.<= (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_le, COMMUTATOR = OPERATOR(pg_catalog.>=), NEGATOR = OPERATOR(pg_catalog.>));
CREATE OPERATOR pg_catalog.= (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_eq, COMMUTATOR = OPERATOR(pg_catalog.=), NEGATOR = OPERATOR(pg_catalog.<>), MERGES);
CREATE OPERATOR pg_catalog.<> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_ne, COMMUTATOR = OPERATOR(pg_catalog.<>), NEGATOR = OPERATOR(pg_catalog.=));
CREATE OPERATOR pg_catalog.>= (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_ge, COMMUTATOR = OPERATOR(pg_catalog.<=), NEGATOR = OPERATOR(pg_catalog.<));
CREATE OPERATOR pg_catalog.> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_gt, COMMUTATOR = OPERATOR(pg_catalog.<), NEGATOR = OPERATOR(pg_catalog.<=));
CREATE OPERATOR CLASS pdb.vector_ops DEFAULT FOR TYPE pdb.vector USING btree AS
    OPERATOR 1 pg_catalog.<, OPERATOR 2 pg_catalog.<=, OPERATOR 3 pg_catalog.=, OPERATOR 4 pg_catalog.>=, OPERATOR 5 pg_catalog.>,
    FUNCTION 1 paradedb.vector_cmp(pdb.vector, pdb.vector);
"#,
    name = "pdb_vector_operators",
    requires = [
        "pdb_vector_type",
        vector_coerce,
        float4_array_to_vector,
        float8_array_to_vector,
        vector_to_float4_array,
        vector_l2_distance,
        vector_negative_inner_product,
        vector_cosine_distance,
        vector_cmp,
        vector_lt,
        vector_le,
        vector_eq,
        vector_ne,
        vector_ge,
        vector_gt
    ]
);

extension_sql!(
    r#"
CREATE OPERATOR CLASS pdb.vector_l2_ops DEFAULT FOR TYPE pdb.vector USING bm25 AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_cosine_ops FOR TYPE pdb.vector USING bm25 AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_ip_ops FOR TYPE pdb.vector USING bm25 AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_l2_ops DEFAULT FOR TYPE pdb.vector USING paradedb AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_cosine_ops FOR TYPE pdb.vector USING paradedb AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_ip_ops FOR TYPE pdb.vector USING paradedb AS STORAGE pdb.vector;
"#,
    name = "pdb_vector_opclasses",
    requires = ["pdb_vector_type", "bm25_ops_anyelement_operator"]
);

extension_sql!(
    r#"
CREATE FUNCTION pdb.setup_pgvector() RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $compat$
DECLARE
    vector_schema text;
    vector_type oid;
    native_type oid := 'pdb.vector'::regtype;
    cast_row record;
    existing_cast record;
    metric text;
    method text;
BEGIN
    SELECT n.nspname INTO vector_schema
    FROM pg_catalog.pg_extension e
    JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'vector';
    IF vector_schema IS NULL THEN
        RETURN;
    END IF;
    vector_type := pg_catalog.format('%I.vector', vector_schema)::regtype;
    FOR cast_row IN SELECT * FROM (VALUES
        (vector_type, native_type, 'i', 'IMPLICIT'),
        (native_type, vector_type, 'a', 'ASSIGNMENT')
    ) AS casts(source_type, target_type, context, keyword)
    LOOP
        SELECT castmethod, castcontext INTO existing_cast
        FROM pg_catalog.pg_cast
        WHERE castsource = cast_row.source_type AND casttarget = cast_row.target_type;
        IF FOUND THEN
            IF existing_cast.castmethod <> 'b' OR existing_cast.castcontext <> cast_row.context THEN
                RAISE EXCEPTION 'incompatible existing cast between % and %',
                    cast_row.source_type::regtype, cast_row.target_type::regtype;
            END IF;
        ELSE
            EXECUTE pg_catalog.format('CREATE CAST (%s AS %s) WITHOUT FUNCTION AS %s',
                cast_row.source_type::regtype, cast_row.target_type::regtype, cast_row.keyword);
        END IF;
        IF EXISTS (
            SELECT FROM pg_catalog.pg_depend d
            JOIN pg_catalog.pg_cast c ON c.oid = d.objid
            WHERE d.classid = 'pg_catalog.pg_cast'::regclass AND d.deptype = 'e'
                AND d.refobjid = (SELECT oid FROM pg_catalog.pg_extension WHERE extname = 'pg_search')
                AND c.castsource = cast_row.source_type AND c.casttarget = cast_row.target_type
        ) THEN
            EXECUTE pg_catalog.format('ALTER EXTENSION pg_search DROP CAST (%s AS %s)',
                cast_row.source_type::regtype, cast_row.target_type::regtype);
        END IF;
    END LOOP;
    FOREACH method IN ARRAY ARRAY['bm25', 'paradedb'] LOOP
        FOREACH metric IN ARRAY ARRAY['l2', 'cosine', 'ip'] LOOP
            IF NOT EXISTS (
                SELECT FROM pg_catalog.pg_opclass c
                JOIN pg_catalog.pg_am a ON a.oid = c.opcmethod
                WHERE c.opcintype = vector_type AND c.opcname = 'vector_' || metric || '_ops'
                    AND a.amname = method
            ) THEN
                EXECUTE pg_catalog.format(
                    'CREATE OPERATOR CLASS %I.%I %s FOR TYPE %I.vector USING %I AS STORAGE %I.vector',
                    vector_schema, 'vector_' || metric || '_ops',
                    CASE WHEN metric = 'l2' THEN 'DEFAULT' ELSE '' END,
                    vector_schema, method, vector_schema);
            END IF;
            IF EXISTS (
                SELECT FROM pg_catalog.pg_depend d
                JOIN pg_catalog.pg_opclass c ON c.oid = d.objid
                JOIN pg_catalog.pg_namespace n ON n.oid = c.opcnamespace
                JOIN pg_catalog.pg_am a ON a.oid = c.opcmethod
                JOIN pg_catalog.pg_extension e ON e.oid = d.refobjid
                WHERE d.classid = 'pg_catalog.pg_opclass'::regclass AND d.deptype = 'e'
                    AND e.extname = 'pg_search' AND n.nspname = vector_schema
                    AND c.opcname = 'vector_' || metric || '_ops' AND a.amname = method
            ) THEN
                EXECUTE pg_catalog.format('ALTER EXTENSION pg_search DROP OPERATOR CLASS %I.%I USING %I',
                    vector_schema, 'vector_' || metric || '_ops', method);
            END IF;
            IF EXISTS (
                SELECT FROM pg_catalog.pg_depend d
                JOIN pg_catalog.pg_opfamily f ON f.oid = d.objid
                JOIN pg_catalog.pg_namespace n ON n.oid = f.opfnamespace
                JOIN pg_catalog.pg_am a ON a.oid = f.opfmethod
                JOIN pg_catalog.pg_extension e ON e.oid = d.refobjid
                WHERE d.classid = 'pg_catalog.pg_opfamily'::regclass AND d.deptype = 'e'
                    AND e.extname = 'pg_search' AND n.nspname = vector_schema
                    AND f.opfname = 'vector_' || metric || '_ops' AND a.amname = method
            ) THEN
                EXECUTE pg_catalog.format('ALTER EXTENSION pg_search DROP OPERATOR FAMILY %I.%I USING %I',
                    vector_schema, 'vector_' || metric || '_ops', method);
            END IF;
        END LOOP;
    END LOOP;
END;
$compat$;

"#,
    name = "pdb_vector_pgvector_compat",
    requires = ["pdb_vector_opclasses", "pdb_vector_operators"]
);
