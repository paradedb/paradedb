use crate::types::{postgres_to_tantivy_map, PostgresType, TantivyType};
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

#[allow(clippy::type_complexity)]
pub fn build_tantivy_schema(
    table_name: &str,
    column_names: &[String],
) -> Result<(Schema, Vec<(String, String, Field)>), String> {
    let table_def: Vec<(String, String)> =
        extract_table_def(table_name).expect("failed to return table definition");
    let mut schema_builder = Schema::builder();
    let mut fields = Vec::new();
    let type_map = postgres_to_tantivy_map();

    for (col_name, data_type) in &table_def {
        if column_names.contains(col_name) {
            let pg_type = PostgresType::from_str(data_type).ok_or_else(|| {
                format!(
                    "Unrecognized PostgreSQL type '{}' for column '{}'",
                    data_type, col_name
                )
            })?;

            let tantivy_type = type_map.get(&pg_type).ok_or_else(|| {
                format!(
                    "Unrecognized Tantivy type for PostgreSQL type '{}' in column '{}'",
                    data_type, col_name
                )
            })?;

            let field = match tantivy_type {
                TantivyType::Text => schema_builder.add_text_field(col_name, TEXT | STORED),
                TantivyType::I64 => schema_builder.add_i64_field(col_name, INDEXED | STORED),
                TantivyType::F64 => schema_builder.add_f64_field(col_name, INDEXED | STORED),
                TantivyType::Bool => schema_builder.add_bool_field(col_name, INDEXED | STORED),
                TantivyType::Json => schema_builder.add_json_field(col_name, STORED),
            };

            fields.push((col_name.clone(), data_type.clone(), field));
        }
    }

    Ok((schema_builder.build(), fields))
}
