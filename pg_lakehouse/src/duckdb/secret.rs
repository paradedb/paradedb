// Copyright (c) 2023-2024 Retake, Inc.
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

use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;

pub enum UserMappingOptions {
    // Universal
    Type,
    Provider,
    Scope,
    Chain,
    // S3/GCS/R2
    KeyId,
    Secret,
    Region,
    SessionToken,
    Endpoint,
    UrlStyle,
    UseSsl,
    UrlCompatibilityMode,
    AccountId,
    // Azure
    ConnectionString,
    AccountName,
    TenantId,
    ClientId,
    ClientSecret,
    ClientCertificatePath,
    HttpProxy,
    ProxyUserName,
    ProxyPassword,
}

impl UserMappingOptions {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Type => "type",
            Self::Provider => "provider",
            Self::Scope => "scope",
            Self::Chain => "chain",
            Self::KeyId => "key_id",
            Self::Secret => "secret",
            Self::Region => "region",
            Self::SessionToken => "session_token",
            Self::Endpoint => "endpoint",
            Self::UrlStyle => "url_style",
            Self::UseSsl => "use_ssl",
            Self::UrlCompatibilityMode => "url_compatibility_mode",
            Self::AccountId => "account_id",
            Self::ConnectionString => "connection_string",
            Self::AccountName => "account_name",
            Self::TenantId => "tenant_id",
            Self::ClientId => "client_id",
            Self::ClientSecret => "client_secret",
            Self::ClientCertificatePath => "client_certificate_path",
            Self::HttpProxy => "http_proxy",
            Self::ProxyUserName => "proxy_user_name",
            Self::ProxyPassword => "proxy_password",
        }
    }

    #[allow(unused)]
    pub fn is_required(&self) -> bool {
        match self {
            Self::Type => true,
            Self::Provider => false,
            Self::Scope => false,
            Self::Chain => false,
            Self::KeyId => false,
            Self::Secret => false,
            Self::Region => false,
            Self::SessionToken => false,
            Self::Endpoint => false,
            Self::UrlStyle => false,
            Self::UseSsl => false,
            Self::UrlCompatibilityMode => false,
            Self::AccountId => false,
            Self::ConnectionString => false,
            Self::AccountName => false,
            Self::TenantId => false,
            Self::ClientId => false,
            Self::ClientSecret => false,
            Self::ClientCertificatePath => false,
            Self::HttpProxy => false,
            Self::ProxyUserName => false,
            Self::ProxyPassword => false,
        }
    }

    #[allow(unused)]
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Type,
            Self::Provider,
            Self::Scope,
            Self::Chain,
            Self::KeyId,
            Self::Secret,
            Self::Region,
            Self::SessionToken,
            Self::Endpoint,
            Self::UrlStyle,
            Self::UseSsl,
            Self::UrlCompatibilityMode,
            Self::AccountId,
            Self::ConnectionString,
            Self::AccountName,
            Self::TenantId,
            Self::ClientId,
            Self::ClientSecret,
            Self::ClientCertificatePath,
            Self::HttpProxy,
            Self::ProxyUserName,
            Self::ProxyPassword,
        ]
        .into_iter()
    }
}

pub fn create_secret(
    secret_name: &str,
    user_mapping_options: HashMap<String, String>,
) -> Result<String> {
    if user_mapping_options.is_empty() {
        bail!("create_secret requires user mapping options")
    }

    let secret_type = Some(format!(
        "TYPE {}",
        user_mapping_options
            .get(UserMappingOptions::Type.as_str())
            .ok_or_else(|| anyhow!("type option required for USER MAPPING"))?
            .as_str()
    ));

    let provider = user_mapping_options
        .get(UserMappingOptions::Provider.as_str())
        .map(|provider| format!("PROVIDER {}", provider));

    let scope = user_mapping_options
        .get(UserMappingOptions::Scope.as_str())
        .map(|scope| format!("SCOPE {}", scope));

    let chain = user_mapping_options
        .get(UserMappingOptions::Chain.as_str())
        .map(|chain| format!("CHAIN '{}'", chain));

    let key_id = user_mapping_options
        .get(UserMappingOptions::KeyId.as_str())
        .map(|key_id| format!("KEY_ID '{}'", key_id));

    let secret = user_mapping_options
        .get(UserMappingOptions::Secret.as_str())
        .map(|secret| format!("SECRET '{}'", secret));

    let region = user_mapping_options
        .get(UserMappingOptions::Region.as_str())
        .map(|region| format!("REGION '{}'", region));

    let session_token = user_mapping_options
        .get(UserMappingOptions::SessionToken.as_str())
        .map(|session_token| format!("SESSION_TOKEN '{}'", session_token));

    let endpoint = user_mapping_options
        .get(UserMappingOptions::Endpoint.as_str())
        .map(|endpoint| format!("ENDPOINT '{}'", endpoint));

    let url_style = user_mapping_options
        .get(UserMappingOptions::UrlStyle.as_str())
        .map(|url_style| format!("URL_STYLE '{}'", url_style));

    let use_ssl = user_mapping_options
        .get(UserMappingOptions::UseSsl.as_str())
        .map(|use_ssl| format!("USE_SSL {}", use_ssl));

    let url_compatibility_mode = user_mapping_options
        .get(UserMappingOptions::UrlCompatibilityMode.as_str())
        .map(|url_compatibility_mode| format!("URL_COMPATIBILITY_MODE {}", url_compatibility_mode));

    let account_id = user_mapping_options
        .get(UserMappingOptions::AccountId.as_str())
        .map(|account_id| format!("ACCOUNT_ID '{}'", account_id));

    let connection_string = user_mapping_options
        .get(UserMappingOptions::ConnectionString.as_str())
        .map(|connection_string| format!("CONNECTION_STRING '{}'", connection_string));

    let account_name = user_mapping_options
        .get(UserMappingOptions::AccountName.as_str())
        .map(|account_name| format!("ACCOUNT_NAME '{}'", account_name));

    let tenant_id = user_mapping_options
        .get(UserMappingOptions::TenantId.as_str())
        .map(|tenant_id| format!("TENANT_ID '{}'", tenant_id));

    let client_id = user_mapping_options
        .get(UserMappingOptions::ClientId.as_str())
        .map(|client_id| format!("CLIENT_ID '{}'", client_id));

    let client_secret = user_mapping_options
        .get(UserMappingOptions::ClientSecret.as_str())
        .map(|client_secret| format!("CLIENT_SECRET '{}'", client_secret));

    let client_certificate_path = user_mapping_options
        .get(UserMappingOptions::ClientCertificatePath.as_str())
        .map(|client_certificate_path| {
            format!("CLIENT_CERTIFICATE_PATH '{}'", client_certificate_path)
        });

    let http_proxy = user_mapping_options
        .get(UserMappingOptions::HttpProxy.as_str())
        .map(|http_proxy| format!("HTTP_PROXY '{}'", http_proxy));

    let proxy_user_name = user_mapping_options
        .get(UserMappingOptions::ProxyUserName.as_str())
        .map(|proxy_user_name| format!("PROXY_USER_NAME '{}'", proxy_user_name));

    let proxy_password = user_mapping_options
        .get(UserMappingOptions::ProxyPassword.as_str())
        .map(|proxy_password| format!("PROXY_PASSWORD '{}'", proxy_password));

    let secret_string = vec![
        secret_type,
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
        account_id,
        connection_string,
        account_name,
        tenant_id,
        client_id,
        client_secret,
        client_certificate_path,
        http_proxy,
        proxy_user_name,
        proxy_password,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>()
    .join(", ");

    Ok(format!(
        "CREATE OR REPLACE SECRET {secret_name} ({secret_string})"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    #[test]
    fn test_create_s3_secret_config_valid() {
        let secret_name = "s3_secret";
        let user_mapping_options = HashMap::from([
            (
                UserMappingOptions::Type.as_str().to_string(),
                "S3".to_string(),
            ),
            (
                UserMappingOptions::Provider.as_str().to_string(),
                "CONFIG".to_string(),
            ),
            (
                UserMappingOptions::KeyId.as_str().to_string(),
                "key_id".to_string(),
            ),
            (
                UserMappingOptions::Secret.as_str().to_string(),
                "secret".to_string(),
            ),
            (
                UserMappingOptions::Region.as_str().to_string(),
                "us-west-2".to_string(),
            ),
            (
                UserMappingOptions::SessionToken.as_str().to_string(),
                "session_token".to_string(),
            ),
            (
                UserMappingOptions::Endpoint.as_str().to_string(),
                "s3.amazonaws.com".to_string(),
            ),
            (
                UserMappingOptions::UrlStyle.as_str().to_string(),
                "vhost".to_string(),
            ),
            (
                UserMappingOptions::UseSsl.as_str().to_string(),
                "true".to_string(),
            ),
            (
                UserMappingOptions::UrlCompatibilityMode
                    .as_str()
                    .to_string(),
                "true".to_string(),
            ),
        ]);

        let expected = "CREATE OR REPLACE SECRET s3_secret (TYPE S3, PROVIDER CONFIG, KEY_ID 'key_id', SECRET 'secret', REGION 'us-west-2', SESSION_TOKEN 'session_token', ENDPOINT 's3.amazonaws.com', URL_STYLE 'vhost', USE_SSL true, URL_COMPATIBILITY_MODE true)";
        let actual = create_secret(secret_name, user_mapping_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        let mut statement = conn.prepare(&actual).unwrap();
        statement.execute([]).unwrap();
    }

    #[test]
    fn test_create_s3_secret_config_invalid() {
        let secret_name = "s3_secret";
        let user_mapping_options = HashMap::from([
            (
                UserMappingOptions::Type.as_str().to_string(),
                "S3".to_string(),
            ),
            (
                UserMappingOptions::Provider.as_str().to_string(),
                "TENANT_ID".to_string(),
            ),
        ]);

        let actual = create_secret(secret_name, user_mapping_options).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid s3 secret should throw an error"),
            Err(e) => assert!(e.to_string().contains("tenant_id")),
        }
    }

    #[test]
    fn test_create_azure_secret_valid() {
        let secret_name = "azure_secret";
        let user_mapping_options = HashMap::from([
            (
                UserMappingOptions::Type.as_str().to_string(),
                "AZURE".to_string(),
            ),
            (
                UserMappingOptions::Provider.as_str().to_string(),
                "CONFIG".to_string(),
            ),
            (
                UserMappingOptions::ConnectionString.as_str().to_string(),
                "connection_string".to_string(),
            ),
            (
                UserMappingOptions::HttpProxy.as_str().to_string(),
                "http_proxy".to_string(),
            ),
            (
                UserMappingOptions::ProxyUserName.as_str().to_string(),
                "proxy_user_name".to_string(),
            ),
            (
                UserMappingOptions::ProxyPassword.as_str().to_string(),
                "proxy_password".to_string(),
            ),
        ]);

        let expected = "CREATE OR REPLACE SECRET azure_secret (TYPE AZURE, PROVIDER CONFIG, CONNECTION_STRING 'connection_string', HTTP_PROXY 'http_proxy', PROXY_USER_NAME 'proxy_user_name', PROXY_PASSWORD 'proxy_password')";
        let actual = create_secret(secret_name, user_mapping_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        let mut statement = conn.prepare(&actual).unwrap();
        statement.execute([]).unwrap();
    }

    #[test]
    fn test_create_type_invalid() {
        let secret_name = "invalid_secret";
        let user_mapping_options = HashMap::from([(
            UserMappingOptions::Type.as_str().to_string(),
            "INVALID".to_string(),
        )]);

        let actual = create_secret(secret_name, user_mapping_options).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid secret type should throw an error"),
            Err(e) => assert!(e.to_string().contains("invalid")),
        }
    }
}
