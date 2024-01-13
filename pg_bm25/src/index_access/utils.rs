use pgrx::pg_sys::Datum;
use pgrx::*;
use serde::Deserialize;

use serde::de::DeserializeOwned;

use std::default::Default;

use std::error::Error;
use std::str::FromStr;

use crate::index_access::options::ParadeOptions;
use crate::json::builder::JsonBuilder;
use crate::parade_index::index::ParadeIndex;

type ConversionFunc = dyn Fn(&mut JsonBuilder, String, pg_sys::Datum, pg_sys::Oid);
pub struct CategorizedAttribute {
    pub attname: String,
    pub typoid: pg_sys::Oid,
    pub attno: usize,
}

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

fn handle_as_generic_string(
    is_array: bool,
    base_type_oid: pg_sys::Oid,
    builder: &mut JsonBuilder,
    attname: String,
    datum: Datum,
) {
    let mut output_func = pg_sys::InvalidOid;
    let mut is_varlena = false;

    unsafe {
        pg_sys::getTypeOutputInfo(base_type_oid, &mut output_func, &mut is_varlena);
    }

    if is_array {
        let array: Array<pg_sys::Datum> = unsafe { Array::from_datum(datum, false).unwrap() };

        let mut values = Vec::with_capacity(array.len());
        for element_datum in array.iter().flatten() {
            if base_type_oid == pg_sys::TEXTOID || base_type_oid == pg_sys::VARCHAROID {
                values.push(Some(
                    unsafe { String::from_datum(element_datum, false) }.unwrap(),
                ));
            } else {
                let result = unsafe {
                    std::ffi::CStr::from_ptr(pg_sys::OidOutputFunctionCall(
                        output_func,
                        element_datum,
                    ))
                };
                values.push(Some(result.to_str().unwrap().to_string()));
            }
        }

        builder.add_string_array(attname, values)
    } else {
        let result = unsafe { String::from_datum(datum, false) }.unwrap();

        builder.add_string(attname, result);
    }
}

pub unsafe fn row_to_json<'a>(
    row: pg_sys::Datum,
    tupdesc: &PgTupleDesc,
    builder: &mut JsonBuilder,
) {
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
    for (attno, attribute) in tupdesc.iter().enumerate() {
        if attribute.is_dropped() {
            dropped += 1;
            continue;
        }
        let attname = attribute.name().to_string();
        let attribute_type_oid = attribute.type_oid();

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
                    builder.add_bool(attname, unsafe { bool::from_datum(datum, false) }.unwrap());
                }
                PgBuiltInOids::INT2OID => {
                    builder.add_i16(attname, unsafe { i16::from_datum(datum, false) }.unwrap());
                }
                PgBuiltInOids::INT4OID => {
                    builder.add_i32(attname, unsafe { i32::from_datum(datum, false) }.unwrap());
                }
                PgBuiltInOids::INT8OID => {
                    builder.add_i64(attname, unsafe { i64::from_datum(datum, false) }.unwrap());
                }
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                    builder.add_u32(attname, unsafe { u32::from_datum(datum, false) }.unwrap())
                }
                PgBuiltInOids::FLOAT4OID => {
                    builder.add_f32(attname, unsafe { f32::from_datum(datum, false) }.unwrap())
                }
                PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                    builder.add_f64(attname, unsafe { f64::from_datum(datum, false) }.unwrap())
                }
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    handle_as_generic_string(is_array, base_oid.value(), builder, attname, datum)
                }
                PgBuiltInOids::JSONOID => builder.add_json_string(
                    attname,
                    unsafe { pgrx::JsonString::from_datum(datum, false) }.unwrap(),
                ),
                PgBuiltInOids::JSONBOID => {
                    builder.add_jsonb(attname, unsafe { JsonB::from_datum(datum, false) }.unwrap())
                }

                _ => {}
            },
            PgOid::Invalid => {
                panic!("{} has a type oid of InvalidOid", attribute.name())
            }
            _ => {}
        }
    }
}

pub fn get_data_directory() -> String {
    unsafe {
        let option_name_cstr =
            std::ffi::CString::new("data_directory").expect("failed to create CString");
        String::from_utf8(
            std::ffi::CStr::from_ptr(pg_sys::GetConfigOptionByName(
                option_name_cstr.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_bytes()
            .to_vec(),
        )
        .expect("Failed to convert C string to Rust string")
    }
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
    use super::{handle_as_generic_string, lookup_index_tupdesc};
    use crate::json::builder::{JsonBuilder, JsonBuilderValue};
    use crate::operator::get_index_oid;
    use pgrx::*;
    use shared::testing::SETUP_SQL;
    use tantivy::schema::Field;

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

    #[pg_test]
    fn test_handle_as_generic_string() {
        let mut fields = std::collections::HashMap::new();
        let attname = "description";
        let field = Field::from_field_id(0);
        fields.insert(attname.to_string(), field);

        let mut builder = JsonBuilder::new(field, fields);
        // new OR track :)
        let datum = "Mirage".into_datum().expect("failed to convert to datum");
        handle_as_generic_string(
            false,
            PgBuiltInOids::VARCHAROID.into(),
            &mut builder,
            attname.to_string(),
            datum,
        );

        let value = builder.values.get(&field).unwrap();
        match value {
            JsonBuilderValue::string(val) => {
                assert_eq!(val, "Mirage");
            }
            _ => panic!("Expected string, found other."),
        }
    }

    #[pg_test]
    fn test_handle_as_generic_string_array() {
        let mut fields = std::collections::HashMap::new();
        let attname = "2023_Tracks";
        let field = Field::from_field_id(0);
        fields.insert(attname.to_string(), field);

        let mut builder = JsonBuilder::new(field, fields);
        // 2023 OR singles :)
        let singles = vec!["Counting Stars", "Mirage", "Ranaway"];
        let datum = singles
            .clone()
            .into_datum()
            .expect("failed to convert tracks to datum");

        handle_as_generic_string(
            true,
            PgBuiltInOids::VARCHAROID.into(),
            &mut builder,
            attname.to_string(),
            datum,
        );
        let value = builder.values.get(&field).unwrap();
        match value {
            JsonBuilderValue::string_array(values) => {
                assert_eq!(values.len(), singles.len());
                for (single, value) in singles.iter().zip(values.iter()) {
                    assert_eq!(value.clone().unwrap(), single.to_string());
                }
            }
            _ => panic!("Incorrect type: expected string_array."),
        }
    }
}
