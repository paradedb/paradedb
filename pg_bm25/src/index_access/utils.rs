use pgrx::*;
use serde::Deserialize;
use serde_json::json;

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
    pub conversion_func: Box<ConversionFunc>,
    pub attno: usize,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct SearchConfig {
    pub query: String,
    pub schema_name: String,
    pub index_name: String,
    pub table_name: String,
    pub key_field: String,
    pub table_schema_name: String,
    pub highlight: Option<String>,
    pub rank: Option<bool>,
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
) -> Result<ParadeIndex, Box<dyn Error>> {
    ParadeIndex::new(index_name, heap_relation, options)
}

pub fn get_parade_index(index_name: &str) -> ParadeIndex {
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

#[allow(clippy::cognitive_complexity)]
pub fn categorize_tupdesc(tupdesc: &PgTupleDesc) -> Vec<CategorizedAttribute> {
    let mut categorized_attributes = Vec::with_capacity(tupdesc.len());

    for (attno, attribute) in tupdesc.iter().enumerate() {
        if attribute.is_dropped() {
            continue;
        }
        let attname = attribute.name();
        let attribute_type_oid = attribute.type_oid();

        let conversion_func: Box<ConversionFunc> = {
            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
                (PgOid::from(array_type), true)
            } else {
                (attribute_type_oid, false)
            };

            match &base_oid {
                PgOid::BuiltIn(builtin) => match builtin {
                    PgBuiltInOids::BOOLOID => Box::new(|builder, name, datum, _oid| {
                        builder.add_bool(name, unsafe { bool::from_datum(datum, false) }.unwrap())
                    }),
                    PgBuiltInOids::INT2OID => Box::new(|builder, name, datum, _oid| {
                        builder.add_i16(name, unsafe { i16::from_datum(datum, false) }.unwrap())
                    }),
                    PgBuiltInOids::INT4OID => Box::new(|builder, name, datum, _oid| {
                        builder.add_i32(name, unsafe { i32::from_datum(datum, false) }.unwrap())
                    }),
                    PgBuiltInOids::INT8OID => Box::new(|builder, name, datum, _oid| {
                        builder.add_i64(name, unsafe { i64::from_datum(datum, false) }.unwrap())
                    }),
                    PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                        Box::new(|builder, name, datum, _oid| {
                            builder.add_u32(name, unsafe { u32::from_datum(datum, false) }.unwrap())
                        })
                    }
                    PgBuiltInOids::FLOAT4OID => Box::new(|builder, name, datum, _oid| {
                        builder.add_f32(name, unsafe { f32::from_datum(datum, false) }.unwrap())
                    }),
                    PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                        Box::new(|builder, name, datum, _oid| {
                            builder.add_f64(name, unsafe { f64::from_datum(datum, false) }.unwrap())
                        })
                    }
                    PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                        handle_as_generic_string(is_array, base_oid.value())
                    }
                    PgBuiltInOids::JSONOID => Box::new(|builder, name, datum, _oid| {
                        builder.add_json_string(
                            name,
                            unsafe { pgrx::JsonString::from_datum(datum, false) }.unwrap(),
                        )
                    }),
                    PgBuiltInOids::JSONBOID => Box::new(|builder, name, datum, _oid| {
                        builder.add_jsonb(name, unsafe { JsonB::from_datum(datum, false) }.unwrap())
                    }),

                    _ => Box::new(|_, _, _, _| {}),
                },
                PgOid::Invalid => {
                    panic!("{} has a type oid of InvalidOid", attribute.name())
                }
                _ => Box::new(|_, _, _, _| {}),
            }
        };

        let attname = serde_json::to_string(&json! {attname})
            .expect("failed to convert attribute name to json");
        categorized_attributes.push(CategorizedAttribute {
            attname,
            typoid: attribute_type_oid.value(),
            conversion_func,
            attno,
        });
    }

    categorized_attributes
}

fn handle_as_generic_string(is_array: bool, base_type_oid: pg_sys::Oid) -> Box<ConversionFunc> {
    let mut output_func = pg_sys::InvalidOid;
    let mut is_varlena = false;

    unsafe {
        pg_sys::getTypeOutputInfo(base_type_oid, &mut output_func, &mut is_varlena);
    }

    if is_array {
        Box::new(move |builder, name, datum, _oid| {
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

            builder.add_string_array(name, values)
        })
    } else {
        Box::new(move |builder, name, datum, _oid| {
            let result = unsafe {
                std::ffi::CStr::from_ptr(pg_sys::OidOutputFunctionCall(output_func, datum))
            };
            let result_str = result
                .to_str()
                .expect("failed to convert unsupported type to a string");

            builder.add_string(name, result_str.to_string());
        })
    }
}

pub unsafe fn row_to_json(
    row: pg_sys::Datum,
    tupdesc: &PgTupleDesc,
    natts: usize,
    dropped: &[bool],
    attributes: &[CategorizedAttribute],
) -> JsonBuilder {
    let mut builder = JsonBuilder::new(natts);

    for (attr, datum) in decon_row(row, tupdesc, natts, dropped, attributes)
        .filter(|item| item.is_some())
        .flatten()
    {
        (attr.conversion_func)(&mut builder, attr.attname.clone(), datum, attr.typoid);
    }

    builder
}

#[inline]
pub unsafe fn decon_row<'a>(
    row: pg_sys::Datum,
    tupdesc: &PgTupleDesc,
    natts: usize,
    dropped: &'a [bool],
    attributes: &'a [CategorizedAttribute],
) -> impl std::iter::Iterator<Item = Option<(&'a CategorizedAttribute, pg_sys::Datum)>> + 'a {
    let td =
        pg_sys::pg_detoast_datum(row.cast_mut_ptr::<pg_sys::varlena>()) as pg_sys::HeapTupleHeader;

    let mut tmptup = pg_sys::HeapTupleData {
        t_len: varsize(td as *mut pg_sys::varlena) as u32,
        t_self: Default::default(),
        t_tableOid: pg_sys::Oid::INVALID,
        t_data: td,
    };

    let mut datums = vec![pg_sys::Datum::from(0); natts];
    let mut nulls = vec![false; natts];

    pg_sys::heap_deform_tuple(
        &mut tmptup,
        tupdesc.as_ptr(),
        datums.as_mut_ptr(),
        nulls.as_mut_ptr(),
    );

    let mut drop_cnt = 0;
    (0..natts).map(move |idx| {
        let is_dropped = dropped[idx];

        if is_dropped {
            drop_cnt += 1;
            None
        } else if nulls[idx] {
            None
        } else {
            Some((&attributes[idx - drop_cnt], datums[idx]))
        }
    })
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
    use super::{categorize_tupdesc, handle_as_generic_string, lookup_index_tupdesc, SearchConfig};
    use crate::json::builder::{JsonBuilder, JsonBuilderValue};
    use crate::operator::get_index_oid;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn convert_str_to_search_query() {
        let query = "lyrics:im:::limit=10&offset=50";
        let expected = SearchConfig {
            query: "lyrics:im".to_string(),
            offset_rows: Some(50),
            limit_rows: Some(10),
            ..Default::default()
        };
        let search_query: SearchConfig = query.parse().expect("failed to parse query");
        assert_eq!(search_query, expected);
    }

    fn make_tuple() -> PgTupleDesc<'static> {
        Spi::run(SETUP_SQL).expect("failed to setup index");
        let oid = get_index_oid("idx_one_republic", "bm25").expect("failed to get index oid");

        let index = unsafe {
            pg_sys::index_open(oid.unwrap(), pg_sys::AccessShareLock as pg_sys::LOCKMODE)
        };
        let index_rel = unsafe { PgRelation::from_pg(index) };
        lookup_index_tupdesc(&index_rel)
    }

    #[pg_test]
    fn test_lookup_index_tupdesc() {
        let tuple = make_tuple();
        assert_eq!(tuple.len(), tuple.natts as usize);
    }

    #[pg_test]
    fn test_categorize_tupdesc() {
        let tuple = make_tuple();
        let categories = categorize_tupdesc(&tuple);

        assert_eq!(categories.len(), tuple.natts as usize);

        for (category, tuple_member) in categories.iter().zip(tuple.iter()) {
            let name: &str = serde_json::from_str(&category.attname).expect("failed to convert");
            assert_eq!(name, tuple_member.name());
        }
    }

    #[pg_test]
    fn test_handle_as_generic_string() {
        let func = handle_as_generic_string(false, PgBuiltInOids::VARCHAROID.into());
        let mut builder = JsonBuilder::new(1);
        let attname = "description";
        // new OR track :)
        let datum = "Mirage".into_datum().expect("failed to convert to datum");
        (func)(
            &mut builder,
            attname.to_string(),
            datum,
            PgBuiltInOids::VARCHAROID.into(),
        );

        let (_, value) = builder.values.first().unwrap();
        match value {
            JsonBuilderValue::string(val) => {
                assert_eq!(val, "Mirage");
            }
            _ => panic!("Expected string, found other."),
        }
    }

    #[pg_test]
    fn test_handle_as_generic_string_array() {
        let func = handle_as_generic_string(true, PgBuiltInOids::VARCHAROID.into());
        let mut builder = JsonBuilder::new(1);
        let attname = "2023_Tracks";
        // 2023 OR singles :)
        let singles = vec!["Counting Stars", "Mirage", "Ranaway"];
        let datum = singles
            .clone()
            .into_datum()
            .expect("failed to convert tracks to datum");
        (func)(
            &mut builder,
            attname.to_string(),
            datum,
            PgBuiltInOids::VARCHAROID.into(),
        );

        let (_, value) = builder.values.first().unwrap();
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
