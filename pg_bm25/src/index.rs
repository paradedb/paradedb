use pgrx::{
    info,
    prelude::PgHeapTuple,
    spi::{SpiClient, SpiTupleTable},
    AllocatedByPostgres, Spi,
};
use tantivy::{
    schema::{Field, Schema},
    Document, Index, IndexSettings, SingleSegmentIndexWriter,
};

use crate::directory::SQLDirectory;
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
        self.create_docs_from_heap(new);
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
            self.create_docs_from_tup(tup_table);
        });
    }

    fn create_docs_from_heap(&self, heap: &PgHeapTuple<'_, AllocatedByPostgres>) {
        let mut writer =
            SingleSegmentIndexWriter::new(self.underlying_index.clone(), INDEX_WRITER_MEM_BUDGET)
                .expect("failed to create index writer");

        for (column_name, data_type, field) in self.fields.as_slice() {
            let mut doc: Document = Document::new();

            match data_type.as_str() {
                // TODO: Support JSON, byte, and date fields
                "text" | "varchar" | "character varying" => {
                    let value: String = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    info!("{}", value);
                    doc.add_text(*field, value);
                }
                "int2" | "int4" | "int8" | "integer" => {
                    let value: i64 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    info!("{}", value);
                    doc.add_i64(*field, value);
                }
                "float4" | "float8" | "numeric" => {
                    let value: f64 = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    info!("{}", value);
                    doc.add_f64(*field, value);
                }
                "bool" => {
                    let value: bool = heap
                        .get_by_name(column_name)
                        .expect("failed to get value for col")
                        .unwrap();
                    info!("{}", value);
                    doc.add_bool(*field, value);
                }
                _ => panic!("Unsupported data type: {}", data_type),
            }
            writer.add_document(doc).expect("failed to add document");
        }

        writer.commit().expect("failed to finalize index writer");
    }

    fn create_docs_from_tup(&self, mut tup_table: SpiTupleTable) {
        let mut writer =
            SingleSegmentIndexWriter::new(self.underlying_index.clone(), INDEX_WRITER_MEM_BUDGET)
                .expect("failed to create index writer");

        for (col_name, data_type, field) in self.fields.as_slice() {
            for row in tup_table.by_ref() {
                let mut doc: Document = Document::new();

                match data_type.as_str() {
                    // TODO: Support JSON, byte, and date fields
                    "text" | "varchar" | "character varying" => {
                        let value = row
                            .get_by_name::<String, &String>(col_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_text(*field, value);
                    }
                    "int2" | "int4" | "int8" | "integer" => {
                        let value = row
                            .get_by_name::<i64, &String>(col_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_i64(*field, value);
                    }
                    "float4" | "float8" | "numeric" => {
                        let value = row
                            .get_by_name::<f64, &String>(col_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_f64(*field, value);
                    }
                    "bool" => {
                        let value = row
                            .get_by_name::<bool, &String>(col_name)
                            .expect("failed to get value for col")
                            .unwrap();
                        doc.add_bool(*field, value);
                    }
                    _ => panic!("Unsupported data type: {}", data_type),
                }
                writer.add_document(doc).expect("failed to add document");
            }
        }

        writer.finalize().expect("failed to finalize index writer");
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
