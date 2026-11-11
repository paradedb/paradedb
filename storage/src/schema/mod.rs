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

pub use common::schema::*;

use crate::api::{CTID_FIELD_NAME, FieldName, HashMap};
use crate::index::utils::load_index_schema;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::options::{BM25IndexOptions, SortByDirection, SortByField};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::{is_datetime_type, is_pgoid_datetime_type};
use crate::postgres::utils::{ExtractedFieldAttribute, resolve_base_type};
use anyhow::{Result, bail};
use pgrx::{PgOid, pg_sys};
use std::cell::{Ref, RefCell};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::LazyLock;
use tantivy::index::{IndexSortByField, Order};
use tantivy::schema::{Field, FieldEntry, FieldType, Schema};
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

/// Derive the SearchFieldType from the tantivy schema, using PostgreSQL metadata for OID/scale.
///
/// This function determines the correct SearchFieldType by examining what's actually
/// stored in the tantivy schema, then augmenting with PostgreSQL metadata (OID, scale).
///
/// This ensures backwards compatibility: legacy indexes that stored NUMERIC as F64
/// will be correctly identified as F64, while new indexes use Numeric64/NumericBytes
/// based on the actual tantivy field type.
fn derive_field_type_from_schema(
    field_entry: &FieldEntry,
    options: &BM25IndexOptions,
    field_name: &FieldName,
) -> SearchFieldType {
    use tantivy::schema::FieldType;

    // Get the computed type from options - this has the PostgreSQL metadata we need
    let computed_type = options.get_field_type(field_name).unwrap_or_else(|| {
        panic!("`{field_name}`'s configuration not found in index WITH options")
    });

    // For most types, the tantivy schema matches what we computed.
    // The exceptions are:
    // - NUMERIC, where legacy indexes used F64 but new code computes Numeric64/NumericBytes.
    // - NUMERIC with a scale outside -18..=18, which older indexes stored as I64.
    // - TIMESTAMP, TIMESTAMPTZ, TIME, TIMETZ, DATE, where legacy indexes used Date but new code computes I64
    match (field_entry.field_type(), computed_type) {
        // If computed type was Numeric64/NumericBytes but stored type is F64,
        // this is a legacy index - use F64
        (FieldType::F64(_), _) if computed_type.is_numeric() => {
            SearchFieldType::F64(computed_type.typeoid().value())
        }
        // If computed_type was i64 but stored type is Date, this is a
        // legacy index - use Date.
        (FieldType::Date(_), SearchFieldType::I64(oid)) if is_datetime_type(oid) => {
            SearchFieldType::Date(oid)
        }
        // BACKWARDS COMPATIBILITY (#6387): an index built before NUMERIC routing bounded the
        // scale can store an out-of-range scale such as `numeric(3,20)` as `I64`. Keep that
        // layout until `REINDEX`, or the first value indexed as bytes breaks the index.
        //
        // TODO: remove this arm, and the `numeric_scale` upgrade test case, once every
        // deployment has rebuilt its indexes on a release that includes #6387.
        (FieldType::I64(_), SearchFieldType::NumericBytes(oid, Some(scale))) => {
            SearchFieldType::Numeric64(oid, scale)
        }
        _ => {
            // For all other types, the computed type is correct
            computed_type
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchIndexSchema {
    schema: Schema,
    bm25_options: BM25IndexOptions,
    categorized: Rc<RefCell<Vec<(SearchField, CategorizedFieldData)>>>,
}

impl From<SearchIndexSchema> for Schema {
    fn from(search_index_schema: SearchIndexSchema) -> Self {
        search_index_schema.schema
    }
}

/// Is `name` a path into a columnar JSON field?
///
/// A segment writes a column only for the JSON paths its own documents carry, so this names the
/// one case where a columnar field has no column in a given segment. A declared field gets its
/// column everywhere, so its absence there is a bug and the reader should say so.
///
/// Takes the tantivy `Schema` and not a [`SearchIndexSchema`], because the latter caches
/// backend-thread state behind an `Rc` and a DataFusion node has to stay `Send`.
pub fn is_columnar_json_path(schema: &Schema, name: &str) -> bool {
    schema.find_field(name).is_some_and(|(field, path)| {
        let entry = schema.get_field_entry(field);
        !path.is_empty() && entry.is_fast() && entry.field_type().is_json()
    })
}

impl SearchIndexSchema {
    pub fn open(indexrel: &PgSearchRelation) -> tantivy::Result<Self> {
        Ok(load_index_schema(indexrel)?
            .map(|schema| Self {
                schema,
                bm25_options: indexrel.options().clone(),
                categorized: Default::default(),
            })
            .unwrap_or_else(|| Self {
                schema: Schema::builder().build(),
                bm25_options: indexrel.options().clone(),
                categorized: Default::default(),
            }))
    }

    pub fn tantivy_schema(&self) -> &Schema {
        &self.schema
    }

    pub fn ctid_field(&self) -> Field {
        self.schema
            .get_field(CTID_FIELD_NAME)
            .expect("ctid field should be present in the index")
    }

    pub fn index_search_tokenizer(&self) -> Option<SearchTokenizer> {
        self.bm25_options.search_tokenizer()
    }

    /// Convert sort_by configuration to Tantivy's IndexSortByField.
    ///
    /// Validates that the sort field exists in the schema and is columnar.
    /// Returns None if sort_by is empty (no segment sorting).
    ///
    /// This is an associated function (not a method) because it's also used during
    /// index creation when only the Tantivy Schema is available.
    pub fn build_sort_by_field(
        sort_by: &[SortByField],
        schema: &Schema,
    ) -> Option<IndexSortByField> {
        // Empty sort_by means no segment sorting
        if sort_by.is_empty() {
            return None;
        }

        // Multi-field validation is done in options.rs during parsing
        let sort_field = &sort_by[0];
        let field_name = sort_field.field_name.as_ref();

        // Validate field exists in schema
        let field = schema.get_field(field_name).unwrap_or_else(|_| {
            panic!(
                "sort_by field '{}' does not exist in the index schema",
                field_name
            )
        });

        // Validate field is columnar
        let field_entry = schema.get_field_entry(field);
        if !field_entry.is_fast() {
            panic!(
                "sort_by field '{}' must be columnar. Add it to the index with 'columnar=true'",
                field_name
            );
        }

        // Convert direction
        let order = match sort_field.direction {
            SortByDirection::Asc => Order::Asc,
            SortByDirection::Desc => Order::Desc,
        };

        Some(IndexSortByField {
            field: field_name.to_string(),
            order,
        })
    }

    pub fn get_field_type(&self, name: impl AsRef<str>) -> Option<SearchFieldType> {
        self.bm25_options
            .get_field_type(&FieldName::from(name.as_ref()))
    }

    pub fn search_field_with_data(
        &self,
        name: impl AsRef<str>,
    ) -> Option<(SearchField, CategorizedFieldData)> {
        let search_field = self.search_field(name)?;
        let data = self
            .categorized_fields()
            .iter()
            .find(|(sf, _)| sf == &search_field)
            .map(|(_, d)| d.clone())?;
        Some((search_field, data))
    }

    /// `attno` is the 1-based attno
    pub fn search_field_with_data_by_attno(
        &self,
        attno: pg_sys::AttrNumber,
    ) -> Option<(SearchField, CategorizedFieldData)> {
        if attno < 0 {
            return None;
        }
        let attno = (attno - 1) as usize;
        let fields = self.categorized_fields();
        let matches =
            |(_, d): &&(SearchField, CategorizedFieldData)| d.source.heap_attno() == Some(attno);
        // A column indexed both plainly and through a tokenizer cast: prefer the plain one.
        fields
            .iter()
            .filter(matches)
            .find(|(_, d)| matches!(d.source, FieldSource::Heap { .. }))
            .or_else(|| fields.iter().find(matches))
            .cloned()
    }

    /// The field type and declared scale of `name` when the column is NUMERIC,
    /// else `None`. Legacy indexes that store NUMERIC as F64 land in the `None`
    /// arm too: their column data really is `Float64`, so the native aggregates
    /// apply.
    ///
    /// Callers use the variant to pick the storage-specific aggregate and the
    /// scale to render results at the column's declared scale; `None` scale
    /// means the column is an unbounded NUMERIC.
    pub fn numeric_field_type(
        &self,
        name: impl AsRef<str>,
    ) -> Option<(SearchFieldType, Option<i16>)> {
        self.get_field_type(name)
            .filter(SearchFieldType::is_numeric)
            .map(|field_type| (field_type, field_type.numeric_scale()))
    }

    pub fn search_field(&self, name: impl AsRef<str>) -> Option<SearchField> {
        let field_name = FieldName::from(name.as_ref());
        match self.schema.get_field(&field_name.root()) {
            Ok(field) => Some(SearchField::new(field, &self.bm25_options, &self.schema)),
            Err(_) => None,
        }
    }

    /// Check if a field supports aggregate pushdown on the Tantivy backend.
    ///
    /// Returns `false` for NUMERIC fields: Tantivy aggregations compute in f64
    /// (losing precision and mishandling NaN/Infinity sentinels) and cannot read
    /// the decimal-bytes storage at all. Standard SQL aggregates over NUMERIC
    /// route to the DataFusion backend instead; `pdb.agg()` has no such backend
    /// and declines. Returns `false` if the field doesn't exist.
    pub fn supports_tantivy_aggregate(&self, name: impl AsRef<str>) -> bool {
        self.search_field(name)
            .is_some_and(|f| !f.field_type().is_numeric())
    }

    /// Whether a `pdb::Query::Term` on `name` has a constant BM25 score across all matching
    /// documents, allowing `RankJoinExec` to treat it as a `0.0`-score filter while preserving
    /// the same row ranking as `HashJoinExec`.
    pub fn is_term_filter_field(&self, name: impl AsRef<str>) -> bool {
        let Some(f) = self.search_field(name) else {
            return false;
        };
        if f.is_tokenized_text() || f.is_json() {
            return false;
        }
        let varies_by_length_or_freq = f.field_entry().has_fieldnorms()
            || f.field_entry()
                .field_type()
                .get_index_record_option()
                .is_some_and(|opt| opt != tantivy::schema::IndexRecordOption::Basic);
        if !varies_by_length_or_freq {
            return true;
        }
        !self
            .categorized_fields()
            .iter()
            .any(|(cf, data)| cf.field() == f.field() && data.is_array)
    }

    pub fn fields(&self) -> impl Iterator<Item = (Field, &FieldEntry)> {
        self.schema.fields()
    }

    pub fn has_vector_field(&self) -> bool {
        self.fields().any(|(_, field_entry)| {
            let field_name: FieldName = field_entry.name().into();
            matches!(
                self.bm25_options.get_field_type(&field_name),
                Some(SearchFieldType::Vector(..))
            )
        })
    }

    /// A lookup from a Postgres column name to search fields that have
    /// marked it as their source column with the 'column' key.
    pub fn alias_lookup(&self) -> HashMap<String, Vec<SearchField>> {
        let mut lookup = HashMap::default();
        let aliased_text_configs = self.bm25_options.aliased_text_configs();
        let aliased_json_configs = self.bm25_options.aliased_json_configs();

        for (alias_name, config) in aliased_text_configs {
            let alias = config
                .alias()
                .expect("aliased text config must have an alias");
            let alias_field = self
                .search_field(alias_name)
                .expect("aliased text config must have a search field");
            lookup
                .entry(alias.to_string())
                .or_insert_with(Vec::new)
                .push(alias_field);
        }

        for (alias_name, config) in aliased_json_configs {
            let alias = config
                .alias()
                .expect("aliased json config must have an alias");
            let alias_field = self
                .search_field(alias_name)
                .expect("aliased json config must have a search field");
            lookup
                .entry(alias.to_string())
                .or_insert_with(Vec::new)
                .push(alias_field);
        }

        lookup
    }

    /// The indexed fields that draw their data from the heap column `column`: any field that
    /// named it with the 'column' key, plus a field of that same name if one is indexed
    /// directly. Drains `alias_lookup` so a caller walking every column visits each aliased
    /// field once.
    fn take_fields_sourced_from(
        &self,
        alias_lookup: &mut HashMap<String, Vec<SearchField>>,
        column: impl AsRef<str>,
    ) -> Vec<SearchField> {
        let column = column.as_ref();
        let mut fields = alias_lookup.remove(column).unwrap_or_default();
        if let Some(direct) = self.search_field(column) {
            fields.push(direct);
        }
        fields
    }

    /// The indexed fields that draw their data from the heap column `column`.
    pub fn fields_sourced_from(&self, column: impl AsRef<str>) -> Vec<SearchField> {
        let mut alias_lookup = self.alias_lookup();
        self.take_fields_sourced_from(&mut alias_lookup, column)
    }

    pub fn categorized_fields(&self) -> Ref<'_, Vec<(SearchField, CategorizedFieldData)>> {
        let is_empty = self.categorized.borrow().is_empty();
        if is_empty {
            let mut categorized = self.categorized.borrow_mut();
            let mut alias_lookup = self.alias_lookup();
            for (
                attname,
                ExtractedFieldAttribute {
                    attno,
                    source,
                    pg_type,
                    tantivy_type,
                    inner_typoid,
                    ..
                },
            ) in self.bm25_options.attributes().iter()
            {
                let search_fields = self.take_fields_sourced_from(&mut alias_lookup, attname);

                for search_field in search_fields {
                    let (base_oid, is_array) = resolve_base_type(PgOid::from_untagged(
                        *inner_typoid,
                    ))
                    .unwrap_or_else(|| {
                        pgrx::error!(
                            "Failed to resolve base type for column {} with type {:?}",
                            attname,
                            tantivy_type.typeoid()
                        )
                    });
                    let is_json = matches!(
                        base_oid,
                        PgOid::BuiltIn(pg_sys::BuiltinOid::JSONBOID | pg_sys::BuiltinOid::JSONOID)
                    );
                    categorized.push((
                        search_field,
                        CategorizedFieldData {
                            attno: *attno,
                            source: *source,
                            pg_type: *pg_type,
                            base_oid,
                            is_array,
                            is_json,
                        },
                    ));
                }
            }
        }

        self.categorized.borrow()
    }
}

#[derive(Debug, Clone)]
pub struct SearchField {
    field: Field,
    field_name: FieldName,
    field_entry: FieldEntry,
    field_type: SearchFieldType,
    field_config: SearchFieldConfig,
}

impl Hash for SearchField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.field.hash(state);
    }
}

impl Eq for SearchField {}

impl PartialEq for SearchField {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field
    }
}

impl SearchField {
    pub fn new(field: Field, options: &BM25IndexOptions, schema: &Schema) -> Self {
        let field_entry = schema.get_field_entry(field).clone();
        let field_name: FieldName = field_entry.name().into();
        let mut field_config = options.field_config_or_default(&field_name);
        restore_legacy_key_field_tokenizer(&field_entry, &mut field_config);

        // Derive field type from the tantivy schema, using PostgreSQL metadata for OID/scale.
        // This ensures backwards compatibility with legacy indexes.
        let field_type = derive_field_type_from_schema(&field_entry, options, &field_name);

        Self {
            field,
            field_name,
            field_entry,
            field_type,
            field_config,
        }
    }

    pub fn field(&self) -> Field {
        self.field
    }

    pub fn field_name(&self) -> &FieldName {
        &self.field_name
    }

    pub fn field_entry(&self) -> &FieldEntry {
        &self.field_entry
    }

    pub fn field_type(&self) -> SearchFieldType {
        self.field_type
    }

    pub fn field_config(&self) -> &SearchFieldConfig {
        &self.field_config
    }

    pub fn is_raw_sortable(&self) -> bool {
        self.is_sortable(SearchNormalizer::Raw)
    }

    pub fn is_lower_sortable(&self) -> bool {
        self.is_sortable(SearchNormalizer::Lowercase)
    }

    pub fn is_fast(&self) -> bool {
        self.field_entry.is_fast()
    }

    /// returns the search field type only if that type is numeric
    pub fn numeric_field_type(&self) -> Option<SearchFieldType> {
        self.field_type.is_numeric().then(|| self.field_type())
    }

    pub fn is_numeric_fast(&self) -> bool {
        match self.field_entry.field_type() {
            FieldType::I64(options) => options.is_fast(),
            FieldType::U64(options) => options.is_fast(),
            FieldType::F64(options) => options.is_fast(),
            FieldType::Bool(options) => options.is_fast(),
            FieldType::Date(options) => options.is_fast(),
            _ => false,
        }
    }

    fn is_sortable(&self, desired_normalizer: SearchNormalizer) -> bool {
        // Range fields are stored as a tantivy JSON object, so they'd otherwise fall into the
        // `JsonObject` arm below. They are sortable via `SortByRange`, which reads the bound
        // sub-columns and compares them the way Postgres' `range_cmp` does. Only raw sorting:
        // range bounds are never tokenized, so there is no lowercased variant to sort by.
        // A true here is not enough on its own: `SortByErasedType` can't read a range, so
        // `sortable_at_position` restricts it to the leading key.
        if matches!(self.field_type, SearchFieldType::Range(_)) {
            return matches!(desired_normalizer, SearchNormalizer::Raw) && self.is_fast();
        }

        // NOTE: This list of supported field types must be synced with the field types which are
        // specialized (in a few spots!) in SearchIndexReader.
        match self.field_entry.field_type() {
            #[allow(deprecated)]
            FieldType::Str(options) => {
                options.is_fast()
                    && options.get_fast_field_tokenizer_name() == Some(desired_normalizer.name())
            }
            FieldType::I64(options) => options.is_fast(),
            FieldType::U64(options) => options.is_fast(),
            FieldType::F64(options) => options.is_fast(),
            FieldType::Bool(options) => options.is_fast(),
            FieldType::Date(options) => options.is_fast(),
            FieldType::Bytes(options) => options.is_fast(),
            // TODO: JSON fields are not yet sortable by us. Range fields are, and are handled
            // above before this match.
            FieldType::JsonObject(_) => false,
            _ => false,
        }
    }

    pub fn is_ctid(&self) -> bool {
        self.field_name().is_ctid()
    }

    pub fn is_datetime(&self) -> bool {
        self.field_entry.field_type().is_date()
    }

    pub fn is_text(&self) -> bool {
        self.field_entry.field_type().is_str()
    }

    /// Returns true if this field uses NumericBytes storage (decimal-bytes column).
    /// NumericBytes fields support direct equality/range pushdown because the
    /// encoding is lexicographically order-preserving.
    pub fn is_numeric_bytes(&self) -> bool {
        matches!(self.field_type, SearchFieldType::NumericBytes(..))
    }

    pub fn supports_positions(&self) -> bool {
        let tokenizer = self.field_config.tokenizer();

        // these tokenizers only emit one token, so they implicitly "support" positions
        #[allow(deprecated)]
        if matches!(
            tokenizer,
            Some(SearchTokenizer::Keyword)
                | Some(SearchTokenizer::KeywordDeprecated)
                | Some(SearchTokenizer::Raw(..))
                | Some(SearchTokenizer::LiteralNormalized(..))
        ) {
            return true;
        }

        let has_positions = self
            .field_entry
            .field_type()
            .get_index_record_option()
            .map(|opt| opt.has_positions())
            .unwrap_or(false);

        let ngram_supports_positions = match tokenizer {
            Some(SearchTokenizer::Ngram {
                min_gram,
                max_gram,
                positions: true,
                ..
            }) => min_gram == max_gram,
            Some(SearchTokenizer::Ngram { .. }) => false,
            _ => true,
        };

        (self.is_text() || self.is_json()) && has_positions && ngram_supports_positions
    }

    pub fn is_json(&self) -> bool {
        self.field_entry.field_type().is_json()
    }

    #[allow(deprecated)]
    pub fn is_keyword(&self) -> bool {
        self.field_config
            .tokenizer()
            .map(|tokenizer| {
                (*tokenizer == SearchTokenizer::Keyword)
                    || (*tokenizer
                        == SearchTokenizer::Raw(SearchTokenizerFilters::keyword().clone()))
            })
            .unwrap_or(false)
    }

    /// A text field whose tokenizer splits values, so a term matches part of a value rather
    /// than one whole value.
    pub fn is_tokenized_text(&self) -> bool {
        self.is_text() && !self.is_keyword() && !self.is_numeric_bytes()
    }

    /// Whether this field's `.stats` min/max order the way query values compare, so a
    /// comparison against them can prove a segment holds no matching value. `is_sortable` does
    /// not advertise IP fields, but their fast-field and query representations are both
    /// `IpAddr` and share the ordering `.stats` records.
    pub fn stats_order_matches_values(&self) -> bool {
        self.is_raw_sortable()
            || (matches!(self.field_type, SearchFieldType::Inet(_)) && self.is_fast())
    }

    /// Whether `.stats` describe this field's indexed terms. Statistics hold whole columnar
    /// values, which are the terms only when no tokenizer splits them, so analyzed text fails
    /// open. Gated on the Tantivy field type because uuid columns also accept `text_fields`
    /// tokenizer configurations.
    pub fn stats_describe_terms(&self) -> bool {
        self.stats_order_matches_values()
            && (!matches!(self.field_entry.field_type(), FieldType::Str(_)) || self.is_keyword())
    }

    #[allow(deprecated)]
    pub fn uses_raw_tokenizer(&self) -> bool {
        self.field_config
            .tokenizer()
            .map(|tokenizer| matches!(tokenizer, SearchTokenizer::Raw(_)))
            .unwrap_or(false)
    }

    pub fn try_coerce(&self, value: &mut PdbOwnedValue) -> Result<()> {
        match (self.field_entry().field_type(), value.clone()) {
            (FieldType::Str(_), PdbOwnedValue::Str(_))
            | (FieldType::U64(_), PdbOwnedValue::U64(_))
            | (FieldType::I64(_), PdbOwnedValue::I64(_))
            | (FieldType::F64(_), PdbOwnedValue::F64(_))
            | (FieldType::Bool(_), PdbOwnedValue::Bool(_))
            | (FieldType::Date(_), PdbOwnedValue::Date(_))
            | (FieldType::Facet(_), PdbOwnedValue::Facet(_))
            | (FieldType::JsonObject(_), PdbOwnedValue::Object(_)) => Ok(()),
            (FieldType::Date(_), PdbOwnedValue::Str(s)) => {
                let typeoid = match self.field_type {
                    SearchFieldType::Date(oid) => PgOid::from(oid),
                    _ => bail!(
                        "field type mismatch: expected Date but got {:?}",
                        self.field_type
                    ),
                };
                let datetime = convert_pg_date_string(typeoid, &s);
                *value = PdbOwnedValue::Date(datetime);
                Ok(())
            }
            (FieldType::I64(_), PdbOwnedValue::Str(s))
                if is_pgoid_datetime_type(self.field_type.typeoid()) =>
            {
                match self.field_type.typeoid() {
                    PgOid::BuiltIn(pg_sys::BuiltinOid::TIMESTAMPOID) => {
                        let pg_dt = PostgresDateTime::try_from_timestamp_str(&s)?;
                        *value = PdbOwnedValue::I64(pg_dt.into_inner());
                    }
                    PgOid::BuiltIn(pg_sys::BuiltinOid::TIMESTAMPTZOID) => {
                        let pg_dt = PostgresDateTime::try_from_timestamptz_str(&s)?;
                        *value = PdbOwnedValue::I64(pg_dt.into_inner());
                    }
                    _ => unreachable!(),
                }
                Ok(())
            }
            (FieldType::U64(_), PdbOwnedValue::I64(v)) => {
                *value = PdbOwnedValue::U64(v.try_into()?);
                Ok(())
            }
            (FieldType::I64(_), PdbOwnedValue::U64(v)) => {
                *value = PdbOwnedValue::I64(v.try_into()?);
                Ok(())
            }
            _ => bail!(
                "cannot coerce value {:?} to field type {:?}",
                value,
                self.field_entry().field_type()
            ),
        }
    }
}

/// Legacy key-field tokenizers were implicit in options, but persisted in the index schema.
fn restore_legacy_key_field_tokenizer(
    field_entry: &FieldEntry,
    field_config: &mut SearchFieldConfig,
) {
    #[allow(deprecated)]
    static LEGACY_KEY_FIELD_TOKENIZERS: LazyLock<[(String, SearchTokenizer); 3]> =
        LazyLock::new(|| {
            let text_or_uuid_key = SearchTokenizer::Raw(SearchTokenizerFilters::keyword().clone());
            let pre_019_text_or_uuid_key =
                SearchTokenizer::Raw(SearchTokenizerFilters::keyword_deprecated().clone());
            let json_key = SearchTokenizer::Raw(SearchTokenizerFilters::default());
            [
                (text_or_uuid_key.name(), text_or_uuid_key.clone()),
                (pre_019_text_or_uuid_key.name(), text_or_uuid_key),
                (json_key.name(), json_key),
            ]
        });

    let (indexing, tokenizer) = match (field_entry.field_type(), field_config) {
        (FieldType::Str(options), SearchFieldConfig::Text { tokenizer, .. }) => {
            (options.get_indexing_options(), tokenizer)
        }
        (FieldType::JsonObject(options), SearchFieldConfig::Json { tokenizer, .. }) => {
            (options.get_text_indexing_options(), tokenizer)
        }
        _ => return,
    };
    let Some(indexing) = indexing else {
        return;
    };

    for (name, key_field_tokenizer) in LEGACY_KEY_FIELD_TOKENIZERS.iter() {
        if indexing.tokenizer() == name {
            *tokenizer = key_field_tokenizer.clone();
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use pgrx::{PgOid, pg_sys};
    use rstest::rstest;
    use tantivy::schema::{IpAddrOptions, JsonObjectOptions, NumericOptions, TextOptions};

    use crate::schema::{SearchFieldConfig, SearchFieldType};

    #[rstest]
    fn test_search_text_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "fieldnorms": true,
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let search_text_option: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Text": config})).unwrap();
        let expected: TextOptions = search_text_option.into();

        let text_options: TextOptions = SearchFieldConfig::default_text().into();
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            text_options.get_fast_field_tokenizer_name()
        );

        let text_options = text_options.set_fast("index");
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }

    #[rstest]
    fn test_search_inet_options() {
        let json = r#"{
            "indexed": true,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Inet": config})).unwrap();
        let inet_options: IpAddrOptions = SearchFieldConfig::default_inet().into();

        assert_eq!(inet_options, expected.into());
    }

    #[rstest]
    fn test_search_numeric_options() {
        let json = r#"{
            "indexed": true,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Numeric": config})).unwrap();
        let int_options: NumericOptions = SearchFieldConfig::default_numeric().into();

        assert_eq!(int_options, expected.into());
    }

    #[rstest]
    fn test_search_boolean_options() {
        let json = r#"{
            "indexed": true,
            "fast": true
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let expected: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Boolean": config})).unwrap();
        let int_options: NumericOptions = SearchFieldConfig::default_numeric().into();

        assert_eq!(int_options, expected.into());
    }

    #[rstest]
    fn test_search_jsonobject_options() {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "expand_dots": true,
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let config: serde_json::Value = serde_json::from_str(json).unwrap();
        let search_json_option: SearchFieldConfig =
            serde_json::from_value(serde_json::json!({"Json": config})).unwrap();
        let expected: JsonObjectOptions = search_json_option.into();

        let json_object_options: JsonObjectOptions = SearchFieldConfig::default_json().into();
        assert_eq!(
            expected.get_fast_field_tokenizer_name(),
            json_object_options.get_fast_field_tokenizer_name()
        );
        assert_eq!(
            expected.is_expand_dots_enabled(),
            json_object_options.is_expand_dots_enabled()
        );

        let text_options = json_object_options.set_fast("index");
        assert_ne!(expected.is_fast(), text_options.is_fast());
    }

    #[rstest]
    fn test_ltree_typeoid() {
        // Ltree uses a custom OID; verify typeoid() extracts it correctly
        let oid: pg_sys::Oid = 99999.into();
        let ft = SearchFieldType::Ltree(oid);
        assert_eq!(ft.typeoid(), PgOid::from(oid));
    }

    #[rstest]
    fn test_default_config_ltree() {
        let config = SearchFieldConfig::default_ltree();
        assert!(matches!(config, SearchFieldConfig::Facet));
    }
}
