use pgrx::{
    prelude::PgHeapTuple,
    spi::{SpiClient, SpiTupleTable},
    AllocatedByPostgres, Spi,
};
use tantivy::{
    schema::{Field, Schema},
    Document, Index, IndexSettings, SingleSegmentIndexWriter,
};

use crate::{directory::SQLDirectory, types::PostgresType};
const INDEX_WRITER_MEM_BUDGET: usize = 50_000_000;

pub struct ParadeIndex {
    name: String,
    table_name: String,
    fields: Vec<(String, String, Field)>,
    underlying_index: Index,
}

impl ParadeIndex {
    pub fn new(
        name: String,
        table_name: String,
        schema: Schema,
        fields: Vec<(String, String, Field)>,
        settings: IndexSettings,
    ) -> Self {
        let dir = SQLDirectory::new(name.to_string());
        let underlying_index = Index::builder()
            .schema(schema.clone())
            .settings(settings.clone())
            .open_or_create(dir)
            .expect("failed to create index");

        Self {
            name,
            table_name,
            fields,
            underlying_index,
        }
    }

    pub fn build(&mut self) {
        self.query_all();
        self.setup_trigger();
    }

    pub fn sync(&mut self, new: &PgHeapTuple<'_, AllocatedByPostgres>) {
        let _ = self.create_docs_from_heap(new);
    }

    fn query_all(&mut self) {
        let query: String = format!("SELECT * FROM {}", self.table_name);

        Spi::connect(|client: SpiClient| {
            let tup_table = client
                .select(&query, None, None)
                .expect("failed to query all");

            // Note: Call this function within the Spi connect context
            // to ensure the returned tuple table lives long enough.
            // Returning the tuple table outside of this context
            // may result in it becoming invalid when the Spi connect context is closed.
            let _ = self.create_docs_from_tup(tup_table);
        });
    }

    fn create_docs_from_heap(
        &self,
        heap: &PgHeapTuple<'_, AllocatedByPostgres>,
    ) -> Result<(), String> {
        let mut writer =
            SingleSegmentIndexWriter::new(self.underlying_index.clone(), INDEX_WRITER_MEM_BUDGET)
                .expect("failed to create index writer");

        let mut doc: Document = Document::new();

        for (column_name, data_type, field) in self.fields.as_slice() {
            let pg_type = PostgresType::from_str(data_type)
                .ok_or_else(|| format!("Unrecognized Postgres type '{}'", data_type))?;

            match pg_type {
                PostgresType::Text | PostgresType::CharacterVarying => {
                    let value: String = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_text(*field, &value);
                }
                PostgresType::SmallInt => {
                    let value: i16 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_i64(*field, value as i64);
                }
                PostgresType::Integer => {
                    let value: i32 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_i64(*field, value as i64);
                }
                PostgresType::BigInt => {
                    let value: i64 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_i64(*field, value);
                }
                PostgresType::Oid => {
                    let value: u32 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_i64(*field, value as i64);
                }
                PostgresType::Real => {
                    let value: f32 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_f64(*field, value as f64);
                }
                PostgresType::DoublePrecision => {
                    let value: f64 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_f64(*field, value);
                }
                PostgresType::Numeric => {
                    let any_numeric_value: pgrx::AnyNumeric = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();

                    let value_str = any_numeric_value.to_string();
                    let value: f64 = value_str.parse().expect("failed to convert numeric to f64");

                    doc.add_f64(*field, value);
                }
                PostgresType::Bool => {
                    let value: bool = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    doc.add_bool(*field, value);
                }
                PostgresType::Json => {
                    let value: pgrx::Json = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();

                    let pgrx::Json(serde_value) = value;
                    if let serde_json::Value::Object(map) = serde_value {
                        doc.add_json_object(*field, map);
                    } else {
                        return Err(format!(
                            "Expected JSON object for column '{}', but found a different type",
                            column_name
                        ));
                    }
                }
                PostgresType::JsonB => {
                    let value: pgrx::JsonB = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();

                    let pgrx::JsonB(serde_value) = value;
                    if let serde_json::Value::Object(map) = serde_value {
                        doc.add_json_object(*field, map);
                    } else {
                        return Err(format!(
                            "Expected JSON object for column '{}', but found a different type",
                            column_name
                        ));
                    }
                }
                _ => return Err(format!("Unhandled Postgres type for '{}'", column_name)),
            }
        }

        writer.add_document(doc).expect("failed to add document");

        writer
            .commit()
            .map(|_| ())
            .map_err(|_| "Failed to commit index writer".to_string())
    }

    fn create_docs_from_tup(&self, mut tup_table: SpiTupleTable) -> Result<(), String> {
        let mut writer =
            SingleSegmentIndexWriter::new(self.underlying_index.clone(), INDEX_WRITER_MEM_BUDGET)
                .expect("failed to create index writer");

        for row in tup_table.by_ref() {
            let mut doc: Document = Document::new();

            for (column_name, data_type, field) in self.fields.as_slice() {
                let pg_type = PostgresType::from_str(data_type)
                    .ok_or_else(|| format!("Unrecognized Postgres type '{}'", data_type))?;

                match pg_type {
                    PostgresType::Text | PostgresType::CharacterVarying => {
                        let value: String = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_text(*field, &value);
                    }
                    PostgresType::SmallInt => {
                        let value: i16 = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_i64(*field, value as i64);
                    }
                    PostgresType::Integer => {
                        let value: i32 = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_i64(*field, value as i64);
                    }
                    PostgresType::BigInt => {
                        let value: i64 = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_i64(*field, value);
                    }
                    PostgresType::Oid => {
                        let value: u32 = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_i64(*field, value as i64);
                    }
                    PostgresType::Real => {
                        let value: f32 = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_f64(*field, value as f64);
                    }
                    PostgresType::DoublePrecision => {
                        let value: f64 = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_f64(*field, value);
                    }
                    PostgresType::Numeric => {
                        let any_numeric_value: pgrx::AnyNumeric = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();

                        let value_str = any_numeric_value.to_string();
                        let value: f64 =
                            value_str.parse().expect("failed to convert numeric to f64");

                        doc.add_f64(*field, value);
                    }
                    PostgresType::Bool => {
                        let value: bool = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_bool(*field, value);
                    }
                    PostgresType::Json => {
                        let value: pgrx::Json = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();

                        let pgrx::Json(serde_value) = value;
                        if let serde_json::Value::Object(map) = serde_value {
                            doc.add_json_object(*field, map);
                        } else {
                            return Err(format!(
                                "Expected JSON object for column '{}', but found a different type",
                                column_name
                            ));
                        }
                    }
                    PostgresType::JsonB => {
                        let value: pgrx::JsonB = row
                            .get_by_name(column_name)
                            .expect("failed to get value for col")
                            .unwrap();

                        let pgrx::JsonB(serde_value) = value;
                        if let serde_json::Value::Object(map) = serde_value {
                            doc.add_json_object(*field, map);
                        } else {
                            return Err(format!(
                                "Expected JSON object for column '{}', but found a different type",
                                column_name
                            ));
                        }
                    }
                    _ => return Err(format!("Unhandled Postgres type for '{}'", column_name)),
                }
            }

            writer.add_document(doc).expect("failed to add document");
        }

        writer
            .finalize()
            .map(|_| ())
            .map_err(|_| "Failed to finalize index writer".to_string())
    }

    fn setup_trigger(&self) {
        let trigger_name = format!("{}_index_trigger", self.table_name);
        let mut field_names_arg = String::from("{");

        for (index, (column_name, _, _)) in self.fields.iter().enumerate() {
            field_names_arg.push_str(column_name);

            if index < self.fields.len() - 1 {
                field_names_arg.push_str(", ");
            }
        }
        field_names_arg.push('}');

        let query = format!(
            "CREATE TRIGGER {} AFTER INSERT ON {} FOR EACH ROW EXECUTE PROCEDURE sync_index('{}', '{}')",
            trigger_name, self.table_name, self.name, field_names_arg
        );
        Spi::run(&query).expect("failed to create trigger")
    }
}
