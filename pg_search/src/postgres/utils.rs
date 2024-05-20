use crate::index::SearchIndex;
use crate::schema::{SearchDocument, SearchIndexSchema};
use crate::writer::{IndexError, WriterDirectory};
use pgrx::{
    pg_sys, varsize, Array, FromDatum, JsonB, JsonString, PgBuiltInOids, PgOid, PgRelation,
    PgTupleDesc,
};
use serde_json::Map;

static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

pub fn get_search_index(index_name: &str) -> &'static mut SearchIndex {
    let directory = WriterDirectory::from_index_name(index_name);
    SearchIndex::from_cache(&directory)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"))
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
    unsafe { PgTupleDesc::from_pg_is_copy(pg_sys::lookup_rowtype_tupdesc_copy(typid, typmod)) }
}

fn datetime_components_to_tantivy_date(
    ymd: Option<(i32, u8, u8)>,
    hms_micro: (u8, u8, u8, u32),
) -> tantivy::schema::Value {
    let naive_dt = match ymd {
        Some(ymd) => chrono::NaiveDate::from_ymd_opt(ymd.0, ymd.1.into(), ymd.2.into()).unwrap(),
        None => chrono::NaiveDateTime::UNIX_EPOCH.date(),
    }
    .and_hms_micro_opt(
        hms_micro.0.into(),
        hms_micro.1.into(),
        hms_micro.2.into(),
        hms_micro.3 % MICROSECONDS_IN_SECOND,
    )
    .unwrap()
    .and_utc();

    tantivy::schema::Value::Date(tantivy::DateTime::from_timestamp_micros(
        naive_dt.timestamp_micros(),
    ))
}

pub fn pgrx_time_to_tantivy_value(value: pgrx::Time) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn pgrx_timetz_to_tantivy_value(value: pgrx::TimeWithTimeZone) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn pgrx_date_to_tantivy_value(value: pgrx::Date) -> tantivy::schema::Value {
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (0, 0, 0, 0),
    )
}

pub fn pgrx_timestamp_to_tantivy_value(value: pgrx::Timestamp) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}

pub fn pgrx_timestamptz_to_tantivy_value(
    value: pgrx::TimestampWithTimeZone,
) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}

pub unsafe fn row_to_search_document(
    tupdesc: &PgTupleDesc,
    values: *mut pg_sys::Datum,
    schema: &SearchIndexSchema,
) -> Result<SearchDocument, IndexError> {
    pgrx::info!("row_to_search_document");
    let row = std::slice::from_raw_parts(values, 1)[0];
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
    let mut document = schema.new_document();
    for (attno, attribute) in tupdesc.iter().enumerate() {
        // Skip attributes that have been dropped.
        if attribute.is_dropped() {
            dropped += 1;
            continue;
        }
        // Skip attributes that have null values.
        if let Some(is_null) = nulls.get(attno) {
            if *is_null {
                continue;
            }
        }

        let attname = attribute.name().to_string();
        let attribute_type_oid = attribute.type_oid();

        // If we can't lookup the attribute name in the field_lookup parameter,
        // it means that this field is not part of the index. We should skip it.
        let search_field = if let Some(index_field) = schema.get_search_field(&attname.into()) {
            index_field
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
                    document.insert(search_field.id, value.into());
                }
                PgBuiltInOids::INT2OID => {
                    let value = i16::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, (value as i64).into());
                }
                PgBuiltInOids::INT4OID => {
                    let value = i32::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, (value as i64).into());
                }
                PgBuiltInOids::INT8OID => {
                    let value = i64::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, value.into());
                }
                PgBuiltInOids::OIDOID => {
                    let value = u32::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, (value as u64).into());
                }
                PgBuiltInOids::FLOAT4OID => {
                    let value = f32::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, (value as f64).into());
                }
                PgBuiltInOids::FLOAT8OID => {
                    let value = f64::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, value.into());
                }
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    if is_array {
                        let array: Array<pg_sys::Datum> =
                            Array::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                        for element_datum in array.iter().flatten() {
                            let value = String::from_datum(element_datum, false)
                                .ok_or(IndexError::DatumDeref)?;
                            document.insert(search_field.id, value.into())
                        }
                    } else {
                        let value =
                            String::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                        document.insert(search_field.id, value.into())
                    }
                }
                PgBuiltInOids::JSONOID => {
                    let JsonString(value) =
                        JsonString::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    document.insert(
                        search_field.id,
                        serde_json::from_str::<Map<String, serde_json::Value>>(&value)?.into(),
                    );
                }
                PgBuiltInOids::JSONBOID => {
                    let JsonB(serde_value) =
                        JsonB::from_datum(datum, false).ok_or(IndexError::DatumDeref)?;
                    let value = serde_json::to_vec(&serde_value)?;
                    document.insert(
                        search_field.id,
                        serde_json::from_slice::<Map<String, serde_json::Value>>(&value)?.into(),
                    );
                }
                PgBuiltInOids::DATEOID => {
                    let value = pgrx::datum::Date::from_datum(datum, false)
                        .ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, pgrx_date_to_tantivy_value(value));
                }
                PgBuiltInOids::TIMESTAMPOID => {
                    let value = pgrx::datum::Timestamp::from_datum(datum, false)
                        .ok_or(IndexError::DatumDeref)?;

                    document.insert(search_field.id, pgrx_timestamp_to_tantivy_value(value));
                }
                PgBuiltInOids::TIMESTAMPTZOID => {
                    let value = pgrx::datum::TimestampWithTimeZone::from_datum(datum, false)
                        .ok_or(IndexError::DatumDeref)?;

                    document.insert(search_field.id, pgrx_timestamptz_to_tantivy_value(value));
                }
                PgBuiltInOids::TIMEOID => {
                    let value = pgrx::datum::Time::from_datum(datum, false)
                        .ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, pgrx_time_to_tantivy_value(value));
                }
                PgBuiltInOids::TIMETZOID => {
                    let value = pgrx::datum::TimeWithTimeZone::from_datum(datum, false)
                        .ok_or(IndexError::DatumDeref)?;
                    document.insert(search_field.id, pgrx_timetz_to_tantivy_value(value));
                }
                unsupported => Err(IndexError::UnsupportedValue(
                    search_field.name.0.to_string(),
                    format!("{unsupported:?}"),
                ))?,
            },
            _ => Err(IndexError::InvalidOid(search_field.name.0.to_string()))?,
        }
    }
    Ok(document)
}
