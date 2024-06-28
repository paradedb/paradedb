use anyhow::Result;
use pgrx::*;

extension_sql!(
    r#"
    CREATE EVENT TRIGGER auto_create_schema_trigger
    ON ddl_command_end
    WHEN TAG IN ('CREATE FOREIGN TABLE')
    EXECUTE FUNCTION auto_create_schema_hook();
    "#,
    name = "auto_create_schema_trigger",
    requires = [auto_create_schema_hook]
);

#[pg_extern(sql = "
    CREATE FUNCTION auto_create_schema_hook() 
    RETURNS event_trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn auto_create_schema_hook() {
    info!("event trigger");
}
