use pgrx::{spi, Spi};
use tantivy::schema::{Field, Schema, INDEXED, STORED, TEXT};

pub fn extract_table_def(table_name: &str) -> Result<Vec<(String, String)>, spi::Error> {
    let query = format!(
        "SELECT attname::text            AS column_name,
                atttypid::regtype::text  AS data_type
         FROM   pg_attribute
         WHERE  attrelid = '{table_name}'::regclass
         AND    attnum > 0
         AND    NOT attisdropped
         ORDER  BY attnum;"
    );

    Spi::connect(|client| {
        let mut results: Vec<(String, String)> = Vec::new();
        let tup_table = client.select(&query, None, None)?;

        for row in tup_table {
            let column_name = row["column_name"]
                .value::<String>()
                .expect("no column name")
                .unwrap();

            let data_type = row["data_type"]
                .value::<String>()
                .expect("no data type")
                .unwrap();

            results.push((column_name, data_type));
        }

        Ok(results)
    })
}

pub fn build_tantivy_schema(
    table_name: &str,
    column_names: &[String],
) -> (Schema, Vec<(String, String, Field)>) {
    let table_def: Vec<(String, String)> =
        extract_table_def(table_name).expect("failed to return table definition");
    let mut schema_builder = Schema::builder();
    let mut fields = Vec::new();

    for (col_name, data_type) in &table_def {
        if column_names.contains(col_name) {
            // TODO: Support JSON, byte, and date fields
            match data_type.as_str() {
                "text" | "varchar" => {
                    let field = schema_builder.add_text_field(col_name, TEXT | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                "int2" | "int4" | "int8" => {
                    let field = schema_builder.add_i64_field(col_name, INDEXED | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                "float4" | "float8" | "numeric" => {
                    let field = schema_builder.add_f64_field(col_name, INDEXED | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                "bool" => {
                    let field = schema_builder.add_bool_field(col_name, STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
                _ => {
                    let field = schema_builder.add_text_field(col_name, TEXT | STORED);
                    fields.push((col_name.clone(), data_type.clone(), field));
                }
            }
        }
    }

    (schema_builder.build(), fields)
}
