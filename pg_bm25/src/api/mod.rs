use pgrx::{iter::TableIterator, pg_sys::ItemPointerData, *};
use rustc_hash::{FxHashMap, FxHashSet};
use tantivy::schema::*;

use crate::index_access::utils::get_parade_index;
use crate::manager::get_executor_manager;
use crate::operator::scan_index;
use crate::parade_index::fields::ToString;

#[pg_extern]
pub fn rank_bm25(ctid: Option<ItemPointerData>) -> f32 {
    match ctid {
        Some(ctid) => get_executor_manager().get_score(ctid).unwrap_or(0.0f32),
        None => 0.0f32,
    }
}

#[pg_extern]
pub fn highlight_bm25(ctid: Option<ItemPointerData>, field_name: String) -> String {
    match ctid {
        Some(ctid) => get_executor_manager()
            .get_highlight(ctid, field_name)
            .unwrap_or("".to_string()),
        None => "".to_string(),
    }
}

#[pg_extern]
pub fn minmax_bm25(
    ctid: pg_sys::ItemPointerData,
    index_name: &str,
    query: &str,
    fcinfo: pg_sys::FunctionCallInfo,
) -> f32 {
    let indexrel =
        PgRelation::open_with_name_and_share_lock(index_name).expect("could not open index");
    let index_oid = indexrel.oid();
    let tid = Some(item_pointer_to_u64(ctid));

    match tid {
        Some(tid) => unsafe {
            let mut lookup_by_query = pg_func_extra(fcinfo, || {
                FxHashMap::<(pg_sys::Oid, Option<String>), FxHashSet<u64>>::default()
            });

            lookup_by_query
                .entry((index_oid, Some(String::from(query))))
                .or_insert_with(|| scan_index(query, index_oid))
                .contains(&tid);

            let max_score = get_executor_manager().get_max_score();
            let min_score = get_executor_manager().get_min_score();
            let raw_score = get_executor_manager().get_score(ctid).unwrap_or(0.0);

            if raw_score == 0.0 && min_score == max_score {
                return 0.0;
            }

            if min_score == max_score {
                return 1.0;
            }

            (raw_score - min_score) / (max_score - min_score)
        },
        None => 0.0,
    }
}

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn schema_bm25(
    index_name: &str,
) -> TableIterator<(
    name!(name, String),
    name!(field_type, String),
    name!(stored, bool),
    name!(indexed, bool),
    name!(fast, bool),
    name!(fieldnorms, bool),
    name!(expand_dots, Option<bool>),
    name!(tokenizer, Option<String>),
    name!(record, Option<String>),
    name!(normalizer, Option<String>),
)> {
    let parade_index = get_parade_index(index_name.to_string());
    let underlying_index = parade_index.underlying_index;
    let schema = underlying_index.schema();

    let mut field_rows = Vec::new();

    for field in schema.fields() {
        let (field, field_entry) = field;
        let name = schema.get_field_name(field).to_string();

        let (field_type, tokenizer, record, normalizer, expand_dots) =
            match field_entry.field_type() {
                FieldType::I64(_) => ("I64".to_string(), None, None, None, None),
                FieldType::U64(_) => ("U64".to_string(), None, None, None, None),
                FieldType::F64(_) => ("F64".to_string(), None, None, None, None),
                FieldType::Bool(_) => ("Bool".to_string(), None, None, None, None),
                FieldType::Str(text_options) => {
                    let indexing_options = text_options.get_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options.map(|opt| opt.index_option().to_string());
                    let normalizer = text_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    ("Str".to_string(), tokenizer, record, normalizer, None)
                }
                FieldType::JsonObject(json_options) => {
                    let indexing_options = json_options.get_text_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options.map(|opt| opt.index_option().to_string());
                    let normalizer = json_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    let expand_dots = Some(json_options.is_expand_dots_enabled());
                    (
                        "JsonObject".to_string(),
                        tokenizer,
                        record,
                        normalizer,
                        expand_dots,
                    )
                }
                _ => ("Other".to_string(), None, None, None, None),
            };

        let row = (
            name,
            field_type,
            field_entry.is_stored(),
            field_entry.is_indexed(),
            field_entry.is_fast(),
            field_entry.has_fieldnorms(),
            expand_dots,
            tokenizer,
            record,
            normalizer,
        );

        field_rows.push(row);
    }

    TableIterator::new(field_rows)
}
