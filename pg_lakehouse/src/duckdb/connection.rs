use anyhow::{anyhow, Result};
use duckdb::arrow::array::RecordBatch;
use duckdb::{Connection, Params, Statement};
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::Once;
use std::thread;
use supabase_wrappers::prelude::require_option;

use super::secret::{SecretType, UserMappingOptions};

// Global mutable static variables
static mut GLOBAL_CONNECTION: Option<UnsafeCell<Connection>> = None;
static mut GLOBAL_STATEMENT: Option<UnsafeCell<Option<Statement<'static>>>> = None;
static mut GLOBAL_ARROW: Option<UnsafeCell<Option<duckdb::Arrow<'static>>>> = None;
static INIT: Once = Once::new();

fn init_globals() {
    let conn = Connection::open_in_memory().expect("failed to open duckdb connection");
    unsafe {
        GLOBAL_CONNECTION = Some(UnsafeCell::new(conn));
        GLOBAL_STATEMENT = Some(UnsafeCell::new(None));
        GLOBAL_ARROW = Some(UnsafeCell::new(None));
    }

    thread::spawn(move || {
        let mut signals =
            Signals::new([SIGTERM, SIGINT, SIGQUIT]).expect("error registering signal listener");
        for _ in signals.forever() {
            let conn = unsafe { &mut *get_global_connection().get() };
            conn.interrupt();
        }
    });
}

fn get_global_connection() -> &'static UnsafeCell<Connection> {
    INIT.call_once(|| {
        init_globals();
    });
    unsafe {
        GLOBAL_CONNECTION
            .as_ref()
            .expect("Connection not initialized")
    }
}

fn get_global_statement() -> &'static UnsafeCell<Option<Statement<'static>>> {
    INIT.call_once(|| {
        init_globals();
    });
    unsafe {
        GLOBAL_STATEMENT
            .as_ref()
            .expect("Statement not initialized")
    }
}

fn get_global_arrow() -> &'static UnsafeCell<Option<duckdb::Arrow<'static>>> {
    INIT.call_once(|| {
        init_globals();
    });
    unsafe { GLOBAL_ARROW.as_ref().expect("Arrow not initialized") }
}

pub fn create_arrow(sql: &str) -> Result<bool> {
    unsafe {
        let conn = &mut *get_global_connection().get();
        let statement = conn.prepare(sql)?;
        let static_statement: Statement<'static> = std::mem::transmute(statement);

        *get_global_statement().get() = Some(static_statement);

        if let Some(static_statement) = get_global_statement().get().as_mut().unwrap() {
            let arrow = static_statement.query_arrow([])?;
            *get_global_arrow().get() = Some(std::mem::transmute::<
                duckdb::Arrow<'_>,
                duckdb::Arrow<'_>,
            >(arrow));
        }
    }

    Ok(true)
}

pub fn clear_arrow() {
    unsafe {
        *get_global_statement().get() = None;
        *get_global_arrow().get() = None;
    }
}

pub fn create_secret(
    secret_name: &str,
    user_mapping_options: HashMap<String, String>,
) -> Result<()> {
    if user_mapping_options.is_empty() {
        return Ok(());
    }

    let secret_type = SecretType::try_from(
        require_option(UserMappingOptions::Type.into(), &user_mapping_options)
            .map_err(|_| anyhow!("USER MAPPING OPTION requires TYPE option"))?,
    )?;

    let type_str = Some(format!("TYPE {}", <&str>::from(secret_type)));
    let provider = user_mapping_options
        .get(UserMappingOptions::Provider.into())
        .map(|provider| format!("PROVIDER {}", provider));
    let scope = user_mapping_options
        .get(UserMappingOptions::Scope.into())
        .map(|scope| format!("SCOPE {}", scope));
    let chain = user_mapping_options
        .get(UserMappingOptions::Chain.into())
        .map(|chain| format!("CHAIN {}", chain));
    let key_id = user_mapping_options
        .get(UserMappingOptions::KeyId.into())
        .map(|key_id| format!("KEY_ID '{}'", key_id));
    let secret = user_mapping_options
        .get(UserMappingOptions::Secret.into())
        .map(|secret| format!("SECRET '{}'", secret));
    let region = user_mapping_options
        .get(UserMappingOptions::Region.into())
        .map(|region| format!("REGION '{}'", region));
    let session_token = user_mapping_options
        .get(UserMappingOptions::SessionToken.into())
        .map(|session_token| format!("SESSION_TOKEN '{}'", session_token));
    let endpoint = user_mapping_options
        .get(UserMappingOptions::Endpoint.into())
        .map(|endpoint| format!("ENDPOINT '{}'", endpoint));
    let url_style = user_mapping_options
        .get(UserMappingOptions::UrlStyle.into())
        .map(|url_style| format!("URL_STYLE '{}'", url_style));
    let use_ssl = user_mapping_options
        .get(UserMappingOptions::UseSsl.into())
        .map(|use_ssl| format!("USE_SSL {}", use_ssl));
    let url_compatibility_mode = user_mapping_options
        .get(UserMappingOptions::UrlCompatibilityMode.into())
        .map(|url_compatibility_mode| {
            format!("URL_COMPATIBILITY_MODE '{}'", url_compatibility_mode)
        });
    let connection_string = user_mapping_options
        .get(UserMappingOptions::ConnectionString.into())
        .map(|connection_string| format!("CONNECTION_STRING '{}'", connection_string));
    let account_name = user_mapping_options
        .get(UserMappingOptions::AccountName.into())
        .map(|account_name| format!("ACCOUNT_NAME '{}'", account_name));
    let tenant_id = user_mapping_options
        .get(UserMappingOptions::TenantId.into())
        .map(|tenant_id| format!("TENANT_ID '{}'", tenant_id));
    let client_id = user_mapping_options
        .get(UserMappingOptions::ClientId.into())
        .map(|client_id| format!("CLIENT_ID '{}'", client_id));
    let client_secret = user_mapping_options
        .get(UserMappingOptions::ClientSecret.into())
        .map(|client_secret| format!("CLIENT_SECRET '{}'", client_secret));
    let client_certificate_path = user_mapping_options
        .get(UserMappingOptions::ClientCertificatePath.into())
        .map(|client_certificate_path| {
            format!("CLIENT_CERTIFICATE_PATH '{}'", client_certificate_path)
        });
    let http_proxy = user_mapping_options
        .get(UserMappingOptions::HttpProxy.into())
        .map(|http_proxy| format!("HTTP_PROXY '{}'", http_proxy));
    let http_user_name = user_mapping_options
        .get(UserMappingOptions::HttpUserName.into())
        .map(|http_user_name| format!("HTTP_USER_NAME '{}'", http_user_name));
    let http_password = user_mapping_options
        .get(UserMappingOptions::HttpPassword.into())
        .map(|http_password| format!("HTTP_PASSWORD '{}'", http_password));

    let secret_string = vec![
        type_str,
        provider,
        scope,
        chain,
        key_id,
        secret,
        region,
        session_token,
        endpoint,
        url_style,
        use_ssl,
        url_compatibility_mode,
        connection_string,
        account_name,
        tenant_id,
        client_id,
        client_secret,
        client_certificate_path,
        http_proxy,
        http_user_name,
        http_password,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>()
    .join(", ");

    execute(
        format!("CREATE OR REPLACE SECRET {secret_name} ({secret_string})").as_str(),
        [],
    )?;

    Ok(())
}

pub fn get_next_batch() -> Result<Option<RecordBatch>> {
    unsafe {
        if let Some(arrow) = get_global_arrow().get().as_mut().unwrap() {
            Ok(arrow.next())
        } else {
            Err(anyhow!("No Arrow batches found in GLOBAL_ARROW"))
        }
    }
}

pub fn get_batches() -> Result<Vec<RecordBatch>> {
    unsafe {
        if let Some(arrow) = get_global_arrow().get().as_mut().unwrap() {
            Ok(arrow.collect())
        } else {
            Err(anyhow!("No Arrow batches found in GLOBAL_ARROW"))
        }
    }
}

pub fn has_results() -> bool {
    unsafe {
        get_global_arrow()
            .get()
            .as_ref()
            .map_or(false, |arrow| arrow.is_some())
    }
}

pub fn execute<P: Params>(sql: &str, params: P) -> Result<usize> {
    unsafe {
        let conn = &*get_global_connection().get();
        conn.execute(sql, params).map_err(|err| anyhow!("{err}"))
    }
}

pub fn view_exists(table_name: &str, schema_name: &str) -> Result<bool> {
    unsafe {
        let conn = &mut *get_global_connection().get();
        let mut statement = conn.prepare(format!("SELECT * from information_schema.tables WHERE table_schema = '{schema_name}' AND table_name = '{table_name}' AND table_type = 'VIEW'").as_str())?;
        match statement.query([])?.next() {
            Ok(Some(_)) => Ok(true),
            _ => Ok(false),
        }
    }
}
