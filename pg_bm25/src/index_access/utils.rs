use pgrx::*;
use serde::Deserialize;

use serde::de::DeserializeOwned;

use std::collections::HashMap;
use std::default::Default;

use std::error::Error;
use std::str::FromStr;

use crate::index_access::options::ParadeOptions;
use crate::parade_index::index::ParadeIndex;
use crate::writer::{IndexEntry, IndexError, IndexKey, IndexValue};

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct SearchConfig {
    pub query: String,
    pub schema_name: String,
    pub index_name: String,
    pub table_name: String,
    pub key_field: String,
    pub offset_rows: Option<usize>,
    pub limit_rows: Option<usize>,
    #[serde(default, deserialize_with = "from_csv")]
    pub fuzzy_fields: Vec<String>,
    pub distance: Option<u8>,
    pub transpose_cost_one: Option<bool>,
    pub prefix: Option<bool>,
    #[serde(default, deserialize_with = "from_csv")]
    pub regex_fields: Vec<String>,
    pub max_num_chars: Option<usize>,
    pub highlight_field: Option<String>,
}

impl SearchConfig {
    pub fn from_jsonb(JsonB(config_json_value): JsonB) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config_json_value)
    }
}

impl FromStr for SearchConfig {
    type Err = serde_path_to_error::Error<json5::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut deserializer = json5::Deserializer::from_str(s).expect("input is not valid json");
        serde_path_to_error::deserialize(&mut deserializer)
    }
}

pub fn create_parade_index(
    index_name: String,
    heap_relation: &PgRelation,
    options: PgBox<ParadeOptions>,
) -> Result<&mut ParadeIndex, Box<dyn Error>> {
    ParadeIndex::new(index_name, heap_relation, options)
}

pub fn get_parade_index(index_name: &str) -> &'static mut ParadeIndex {
    ParadeIndex::from_index_name(index_name)
}

pub fn lookup_index_tupdesc(indexrel: &PgRelation) -> PgTupleDesc<'static> {
    let tupdesc = indexrel.tuple_desc();

    let typid = tupdesc
        .get(0)
        .expect("no attribute #0 on tupledesc")
        .type_oid()
        .value();
    let typmod = tupdesc
        .get(0)
        .expect("no attribute #0 on tupledesc")
        .type_mod();

    // lookup the tuple descriptor for the rowtype we're *indexing*, rather than
    // using the tuple descriptor for the index definition itself
    unsafe {
        PgMemoryContexts::TopTransactionContext.switch_to(|_| {
            PgTupleDesc::from_pg_is_copy(pg_sys::lookup_rowtype_tupdesc_copy(typid, typmod))
        })
    }
}

pub unsafe fn row_to_index_entries<'a>(
    tupdesc: &PgTupleDesc,
    values: *mut pg_sys::Datum,
    field_lookup: &HashMap<String, IndexKey>,
) -> Result<Vec<IndexEntry>, IndexError> {
    let row = unsafe { std::slice::from_raw_parts(values, 1)[0] };
    let td =
        pg_sys::pg_detoast_datum(row.cast_mut_ptr::<pg_sys::varlena>()) as pg_sys::HeapTupleHeader;

    let mut tmptup = pg_sys::HeapTupleData {
        t_len: varsize(td as *mut pg_sys::varlena) as u32,
        t_self: Default::default(),
        t_tableOid: pg_sys::Oid::INVALID,
        t_data: td,
    };

    let mut datums = vec![pg_sys::Datum::from(0); tupdesc.natts as usize];
    let mut nulls = vec![false; tupdesc.natts as usize];

    pg_sys::heap_deform_tuple(
        &mut tmptup,
        tupdesc.as_ptr(),
        datums.as_mut_ptr(),
        nulls.as_mut_ptr(),
    );

    let mut dropped = 0;
    let mut index_entries: Vec<IndexEntry> = vec![];
    for (attno, attribute) in tupdesc.iter().enumerate() {
        if attribute.is_dropped() {
            dropped += 1;
            continue;
        }
        let attname = attribute.name().to_string();
        let attribute_type_oid = attribute.type_oid();

        // If we can't lookup the attribute name in the field_lookup parameter,
        // it means that this field is not part of the index. We should skip it.
        let index_key = if let Some(index_key) = field_lookup.get(&attname) {
            index_key
        } else {
            continue;
        };

        let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
        let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
            (PgOid::from(array_type), true)
        } else {
            (attribute_type_oid, false)
        };

        let datum = datums[attno - dropped];

        match &base_oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => {
                    let value = bool::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::Bool(value)));
                }
                PgBuiltInOids::INT2OID => {
                    let value = i16::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::I16(value)));
                }
                PgBuiltInOids::INT4OID => {
                    let value = i32::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::I32(value)));
                }
                PgBuiltInOids::INT8OID => {
                    let value = i64::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::I64(value)));
                }
                PgBuiltInOids::OIDOID => {
                    let value = u32::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::U32(value)));
                }
                PgBuiltInOids::FLOAT4OID => {
                    let value = f32::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::F32(value)));
                }
                PgBuiltInOids::FLOAT8OID => {
                    let value = f64::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::F64(value)));
                }
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    if is_array {
                        let array: Array<pg_sys::Datum> =
                            Array::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                        for element_datum in array.iter().flatten() {
                            let value = String::from_datum(element_datum, false)
                                .ok_or(IndexError::DatumDeref)?;
                            index_entries
                                .push(IndexEntry::new(*index_key, IndexValue::String(value)));
                        }
                    } else {
                        let value =
                            String::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                        index_entries.push(IndexEntry::new(*index_key, IndexValue::String(value)));
                    }
                }
                PgBuiltInOids::JSONOID => {
                    let JsonString(value) =
                        JsonString::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::Json(value)));
                }
                PgBuiltInOids::JSONBOID => {
                    let JsonB(serde_value) =
                        JsonB::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    let value = serde_json::to_vec(&serde_value)?;
                    index_entries.push(IndexEntry::new(*index_key, IndexValue::JsonB(value)));
                }
                unsupported => Err(IndexError::UnsupportedValue(
                    attname.to_string(),
                    format!("{unsupported:?}"),
                ))?,
            },
            _ => Err(IndexError::InvalidOid(attname.to_string()))?,
        }
    }

    Ok(index_entries)
}

// Helpers to deserialize a comma-separated string, following all the rules
// of csv documents. This let's us easily use syntax like 1,2,3 or one,two,three
// in the SearchQuery input strings.

fn from_csv<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: DeserializeOwned + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    deserializer.deserialize_str(CSVVecVisitor::<T>::default())
}

/// Visits a string value of the form "v1,v2,v3" into a vector of bytes Vec<u8>
struct CSVVecVisitor<T: DeserializeOwned + std::str::FromStr>(std::marker::PhantomData<T>);

impl<T: DeserializeOwned + std::str::FromStr> Default for CSVVecVisitor<T> {
    fn default() -> Self {
        CSVVecVisitor(std::marker::PhantomData)
    }
}

impl<'de, T: DeserializeOwned + std::str::FromStr> serde::de::Visitor<'de> for CSVVecVisitor<T>
where
    <T as std::str::FromStr>::Err: std::fmt::Debug, // handle the parse error in a generic way
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a str")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Treat the comma-separated string as a single record in a CSV.
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(s.as_bytes());

        // Try to get the record and collect its values into a vector.
        let mut output = Vec::new();
        for result in rdr.records() {
            match result {
                Ok(record) => {
                    for field in record.iter() {
                        output.push(
                            field
                                .parse::<T>()
                                .map_err(|_| E::custom("Failed to parse field"))?,
                        );
                    }
                }
                Err(e) => {
                    return Err(E::custom(format!(
                        "could not deserialize sequence value: {:?}",
                        e
                    )));
                }
            }
        }

        Ok(output)
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::lookup_index_tupdesc;
    use crate::operator::get_index_oid;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    fn make_tuple() -> PgTupleDesc<'static> {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        let oid = get_index_oid("one_republic_songs_bm25_index", "bm25")
            .expect("failed to get index oid");

        let index = unsafe {
            pg_sys::index_open(oid.unwrap(), pg_sys::AccessShareLock as pg_sys::LOCKMODE)
        };
        let index_rel = unsafe { PgRelation::from_pg(index) };
        lookup_index_tupdesc(&index_rel)
    }

    #[pg_test]
    fn test_lookup_index_tupdesc() {
        crate::setup_background_workers();
        let tuple = make_tuple();
        assert_eq!(tuple.len(), tuple.natts as usize);
    }
}
