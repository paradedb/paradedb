use pgrx::*;
use serde::Deserialize;
use serde_json::json;

use serde::de::DeserializeOwned;

use std::default::Default;

use std::str::FromStr;

use crate::json::builder::JsonBuilder;
use crate::parade_index::index::ParadeIndex;
use crate::index_access::options::ParadeOptions;

type ConversionFunc = dyn Fn(&mut JsonBuilder, String, pg_sys::Datum, pg_sys::Oid);
pub struct CategorizedAttribute {
    pub attname: String,
    pub typoid: pg_sys::Oid,
    pub conversion_func: Box<ConversionFunc>,
    pub attno: usize,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub config: SearchQueryConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct SearchQueryConfig {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    #[serde(default, deserialize_with = "from_csv")]
    pub fuzzy_fields: Vec<String>,
    #[serde(default, deserialize_with = "from_csv")]
    pub prefix_fields: Vec<String>,
}

impl FromStr for SearchQuery {
    type Err = serde_qs::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.rsplitn(2, ":::");

        let config_part = parts.next().unwrap_or("");
        let query = parts.next().unwrap_or_default().to_string();

        if query.is_empty() {
            Ok(SearchQuery {
                query: config_part.to_string(),
                config: SearchQueryConfig::default(),
            })
        } else {
            let config: SearchQueryConfig = serde_qs::from_str(config_part)?;
            Ok(SearchQuery { query, config })
        }
    }
}

pub fn create_parade_index(index_name: String, table_name: String, options: PgBox<ParadeOptions>) -> ParadeIndex {
    ParadeIndex::new(index_name, table_name, options)
}

pub fn get_parade_index(index_name: String) -> ParadeIndex {
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
        let typoid = attribute.type_oid();

        let conversion_func: Box<ConversionFunc> = {
            let attribute_type_oid = attribute.type_oid();

            match &attribute_type_oid {
                PgOid::BuiltIn(builtin) => match builtin {
                    PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                        handle_as_generic_string(attribute_type_oid.value())
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
            typoid: typoid.value(),
            conversion_func,
            attno,
        });
    }

    categorized_attributes
}

fn handle_as_generic_string(base_type_oid: pg_sys::Oid) -> Box<ConversionFunc> {
    let mut output_func = pg_sys::InvalidOid;
    let mut is_varlena = false;

    unsafe {
        pg_sys::getTypeOutputInfo(base_type_oid, &mut output_func, &mut is_varlena);
    }

    Box::new(move |builder, name, datum, _oid| {
        let result =
            unsafe { std::ffi::CStr::from_ptr(pg_sys::OidOutputFunctionCall(output_func, datum)) };
        let result_str = result
            .to_str()
            .expect("failed to convert unsupported type to a string");

        builder.add_string(name, result_str.to_string());
    })
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
