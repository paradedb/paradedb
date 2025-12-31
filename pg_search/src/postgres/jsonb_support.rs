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

use pgrx::{pg_sys, AnyNumeric, FromDatum};
use std::str::{FromStr, Utf8Error};

// we redefine these functions because we don't want pgrx' per-function FFI wrappers -- we intend
// to use them, together, in a single FFI wrapper closure
extern "C-unwind" {
    pub fn JsonbIteratorInit(container: *mut pg_sys::JsonbContainer) -> *mut pg_sys::JsonbIterator;
    pub fn JsonbIteratorNext(
        it: *mut *mut pg_sys::JsonbIterator,
        val: *mut pg_sys::JsonbValue,
        skip_nested: bool,
    ) -> pg_sys::JsonbIteratorToken::Type;

}

/// Efficiently convert a raw `jsonb` [`pg_sys::Datum`] given to us by Postgres into a [`serde_json::Value`]
/// structure.  The provided datum does not need to be detoasted as this function handles it.
///
/// # Safety
///
/// This function is unsafe as it cannot assert the provided `datum` is really a jsonb datum.  This is
/// the caller's responsibility.
pub unsafe fn jsonb_datum_to_serde_json_value(
    datum: pg_sys::Datum,
) -> Option<Result<serde_json::Value, Utf8Error>> {
    if datum.is_null() {
        return None;
    }
    #[cfg(any(feature = "pg14", feature = "pg15"))]
    let jsonb = pg_sys::pg_detoast_datum(datum.cast_mut_ptr()).cast::<pg_sys::Jsonb>();
    #[cfg(not(any(feature = "pg14", feature = "pg15")))]
    let jsonb = pg_sys::DatumGetJsonbP(datum);
    let container = &mut (*jsonb).root;
    let result = pg_sys::submodules::ffi::pg_guard_ffi_boundary(|| {
        let mut iter = JsonbIteratorInit(container);
        jsonb_iterate(&mut iter)
    });

    if !std::ptr::eq(datum.cast_mut_ptr(), jsonb) {
        pg_sys::pfree(jsonb.cast());
    }
    Some(result)
}

unsafe fn jsonb_iterate(
    iter: &mut *mut pg_sys::JsonbIterator,
) -> Result<serde_json::Value, Utf8Error> {
    let mut jsonb_value = pg_sys::JsonbValue::default();

    let mut stack = Vec::new();
    let mut dangling_keys = Vec::new();
    let mut is_scalar = false;

    loop {
        let mut next = JsonbIteratorNext(iter, &mut jsonb_value, false);

        match next {
            pg_sys::JsonbIteratorToken::WJB_BEGIN_ARRAY => {
                if jsonb_value.val.array.rawScalar {
                    is_scalar = true;
                }
                stack.push(serde_json::Value::Array(Vec::with_capacity(
                    jsonb_value.val.array.nElems as usize,
                )));
            }
            pg_sys::JsonbIteratorToken::WJB_END_ARRAY => {
                if stack.len() > 1 {
                    let value = stack.pop().unwrap_unchecked();
                    let last = stack.last_mut().unwrap_unchecked();

                    if let Some(array) = last.as_array_mut() {
                        array.push(value);
                    } else if let Some(map) = last.as_object_mut() {
                        let Some(dangling_key) = dangling_keys.pop() else {
                            panic!("malformed jsonb structure: missing key for array value")
                        };
                        map.insert(dangling_key, value);
                    } else {
                        panic!("malformed jsonb structure")
                    }
                }
            }
            pg_sys::JsonbIteratorToken::WJB_BEGIN_OBJECT => {
                stack.push(serde_json::Value::Object(serde_json::Map::with_capacity(
                    jsonb_value.val.object.nPairs as usize,
                )));
            }
            pg_sys::JsonbIteratorToken::WJB_END_OBJECT => {
                if stack.len() > 1 {
                    let value = stack.pop().unwrap_unchecked();
                    let last = stack.last_mut().unwrap_unchecked();

                    if let Some(array) = last.as_array_mut() {
                        array.push(value);
                    } else if let Some(map) = last.as_object_mut() {
                        let Some(dangling_key) = dangling_keys.pop() else {
                            panic!("malformed jsonb structure: missing key for object value")
                        };
                        map.insert(dangling_key, value);
                    } else {
                        panic!("malformed jsonb structure")
                    };
                }
            }
            pg_sys::JsonbIteratorToken::WJB_KEY => {
                let key = jsonb_value_to_serde_value(&jsonb_value)?;
                let serde_json::Value::String(key) = key else {
                    panic!("key is not a string")
                };

                next = JsonbIteratorNext(iter, &mut jsonb_value, false);
                if next == pg_sys::JsonbIteratorToken::WJB_VALUE {
                    let value = jsonb_value_to_serde_value(&jsonb_value)?;
                    let last = stack.last_mut().unwrap_unchecked();
                    let Some(map) = last.as_object_mut() else {
                        panic!("malformed jsonb structure: current value is not an object")
                    };
                    map.insert(key, value);
                } else if next == pg_sys::JsonbIteratorToken::WJB_BEGIN_OBJECT {
                    dangling_keys.push(key);
                    stack.push(serde_json::Value::Object(serde_json::Map::with_capacity(
                        jsonb_value.val.object.nPairs as usize,
                    )));
                } else if next == pg_sys::JsonbIteratorToken::WJB_BEGIN_ARRAY {
                    dangling_keys.push(key);
                    stack.push(serde_json::Value::Array(Vec::with_capacity(
                        jsonb_value.val.array.nElems as usize,
                    )));
                } else {
                    panic!("malformed jsonb structure: unexpected token type after key: `{next}`")
                }
            }
            pg_sys::JsonbIteratorToken::WJB_VALUE => {
                stack.push(jsonb_value_to_serde_value(&jsonb_value)?);
            }
            pg_sys::JsonbIteratorToken::WJB_ELEM => {
                let value = jsonb_value_to_serde_value(&jsonb_value)?;
                let last = stack.last_mut().unwrap_unchecked();
                let Some(array) = last.as_array_mut() else {
                    panic!("malformed jsonb structure: current value is not an array")
                };
                array.push(value);
            }
            pg_sys::JsonbIteratorToken::WJB_DONE => break,

            _ => panic!("unexpected jsonb iterator token: {next}"),
        }
    }

    let result = stack.pop().unwrap();
    Ok(match result {
        serde_json::Value::Array(mut array) if is_scalar => array.pop().unwrap(),
        other => other,
    })
}

unsafe fn jsonb_value_to_serde_value(
    value: &pg_sys::JsonbValue,
) -> Result<serde_json::Value, Utf8Error> {
    let value = match value.type_ {
        pg_sys::jbvType::jbvNull => serde_json::Value::Null,
        pg_sys::jbvType::jbvString => {
            assert!(
                !value.val.string.val.is_null(),
                "string value must not be null"
            );
            let slice = std::slice::from_raw_parts(
                value.val.string.val.cast(),
                value.val.string.len as usize,
            );

            // SAFETY: we know that the slice is valid UTF-8 because it was created by Postgres
            serde_json::Value::String(std::str::from_utf8_unchecked(slice).to_string())
        }
        pg_sys::jbvType::jbvNumeric => {
            assert!(
                !value.val.numeric.is_null(),
                "numeric value must not be null"
            );
            let num = AnyNumeric::from_datum(pg_sys::Datum::from(value.val.numeric), false)
                .unwrap_unchecked();
            // yucko!  however, this is what Postgres does internally
            serde_json::Value::Number(
                serde_json::Number::from_str(num.to_string().as_str()).unwrap(),
            )
        }
        pg_sys::jbvType::jbvBool => serde_json::Value::Bool(value.val.boolean),
        pg_sys::jbvType::jbvArray => {
            assert!(
                !value.val.array.elems.is_null(),
                "array elements must not be null"
            );
            let elems =
                std::slice::from_raw_parts(value.val.array.elems, value.val.array.nElems as usize);
            let vec = elems
                .iter()
                .map(|value| jsonb_value_to_serde_value(value))
                .collect::<Result<Vec<_>, Utf8Error>>()?;
            serde_json::Value::Array(vec)
        }
        pg_sys::jbvType::jbvObject => {
            assert!(
                !value.val.object.pairs.is_null(),
                "object pairs must not be null"
            );
            let mut map = serde_json::Map::new();
            let pairs = std::slice::from_raw_parts(
                value.val.object.pairs,
                value.val.object.nPairs as usize,
            );
            for pair in pairs {
                let key = jsonb_value_to_serde_value(&pair.key)?;
                let value = jsonb_value_to_serde_value(&pair.value)?;

                let serde_json::Value::String(key) = key else {
                    panic!("key is not a string")
                };
                map.insert(key, value);
            }
            serde_json::Value::Object(map)
        }

        //
        // Postgres uses these internally.  we don't need to support them as they don't
        // come up when simply converting a jsonb value to a String, which is effectively what
        // we're doing in this routine
        //
        pg_sys::jbvType::jbvBinary => {
            unreachable!("found a `jbvBinary` jsonb value type")
        }
        pg_sys::jbvType::jbvDatetime => {
            unreachable!("found a `jbvDatetime` jsonb value type")
        }

        _ => panic!("unexpected jsonb value type: {}", value.type_),
    };

    Ok(value)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::postgres::jsonb_support::jsonb_datum_to_serde_json_value;
    use pgrx::{pg_test, IntoDatum, JsonB};
    use proptest::collection::{hash_map, vec as pvec};
    use proptest::prelude::*;
    use serde_json::{Number, Value};

    fn jsonb_deserialize(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let json_value: Value = serde_json::from_str(input)?;
        let datum = JsonB(json_value).into_datum();

        unsafe {
            jsonb_datum_to_serde_json_value(datum.unwrap())
                .unwrap()
                .map_err(|e| e.into())
        }
    }

    // Strategy: JSON leaf values (null, bool, number, string)
    fn json_leaf() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|i| Value::Number(Number::from(i))),
            any::<u64>().prop_map(|u| Value::Number(Number::from(u))),
            // limit range to avoid precision differences between Postgres and Rust
            (-100.0f64..=100.0f64).prop_map(|f| Value::Number(
                Number::from_f64((f * 1000.0).round() / 1000.0).unwrap()
            )),
            any::<String>().prop_map(Value::String),
        ]
    }

    fn json_value() -> impl Strategy<Value = Value> {
        json_leaf().prop_recursive(6, 64, 8, |inner| {
            let arr = pvec(inner.clone(), 0..8).prop_map(Value::Array);
            let obj = hash_map(any::<String>(), inner, 0..8)
                .prop_map(|m| Value::Object(m.into_iter().collect()));
            prop_oneof![arr, obj]
        })
    }

    fn jsonish_strings() -> impl Strategy<Value = String> {
        prop_oneof![
            json_value().prop_map(|v| serde_json::to_string(&v).unwrap()),
            json_value().prop_map(|v| serde_json::to_string_pretty(&v).unwrap()),
            any::<String>(),
        ]
    }

    #[pg_test]
    fn custom_jsonb_deserializer_matches_serde_json() {
        proptest!(|(v in json_value())| {
            let s = serde_json::to_string(&v).unwrap();

            let mine = jsonb_deserialize(&s).expect("custom deserializer failed on valid JSON");
            let canon: Value = serde_json::from_str(&s).expect("serde_json failed on its own output");

            // Deep equality of the semantic JSON values
            pgrx::warning!("{s}");
            prop_assert_eq!(mine, canon, "json={}", s);
        });

        // 2) Agreement on valid vs invalid, and equality when both succeed.
        proptest!(|(s in jsonish_strings())| {
            let mine = jsonb_deserialize(&s);
            let canon = serde_json::from_str::<Value>(&s);

            prop_assert_eq!(mine.is_ok(), canon.is_ok(), "parsers disagree on validity for input: {}", s);

            if let (Ok(mv), Ok(cv)) = (mine, canon) {
                prop_assert_eq!(mv, cv, "parsers produced different Values for input: {}", s);
            }
        });
    }
}
