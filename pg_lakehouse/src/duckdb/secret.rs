use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use supabase_wrappers::prelude::*;

use super::connection;

pub enum SecretType {
    Azure,
    S3,
    Gcs,
    R2,
}

pub enum Provider {
    Config,
    CredentialChain,
    ServicePrincipal,
}

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
    // Azure
    ConnectionString,
    AccountName,
    TenantId,
    ClientId,
    ClientSecret,
    ClientCertificatePath,
    HttpProxy,
    HttpUserName,
    HttpPassword,
}

impl TryFrom<&str> for UserMappingOptions {
    type Error = anyhow::Error;

    fn try_from(option: &str) -> Result<Self> {
        match option.to_uppercase().as_str() {
            "TYPE" => Ok(Self::Type),
            "PROVIDER" => Ok(Self::Provider),
            "SCOPE" => Ok(Self::Scope),
            "CHAIN" => Ok(Self::Chain),
            "KEY_ID" => Ok(Self::KeyId),
            "SECRET" => Ok(Self::Secret),
            "REGION" => Ok(Self::Region),
            "SESSION_TOKEN" => Ok(Self::SessionToken),
            "ENDPOINT" => Ok(Self::Endpoint),
            "URL_STYLE" => Ok(Self::UrlStyle),
            "USE_SSL" => Ok(Self::UseSsl),
            "URL_COMPATIBILITY_MODE" => Ok(Self::UrlCompatibilityMode),
            "CONNECTION_STRING" => Ok(Self::ConnectionString),
            "ACCOUNT_NAME" => Ok(Self::AccountName),
            "TENANT_ID" => Ok(Self::TenantId),
            "CLIENT_ID" => Ok(Self::ClientId),
            "CLIENT_SECRET" => Ok(Self::ClientSecret),
            "CLIENT_CERTIFICATE_PATH" => Ok(Self::ClientCertificatePath),
            "HTTP_PROXY" => Ok(Self::HttpProxy),
            "HTTP_USER_NAME" => Ok(Self::HttpUserName),
            "HTTP_PASSWORD" => Ok(Self::HttpPassword),
            _ => bail!("Invalid user mapping option: {}", option),
        }
    }
}

impl From<UserMappingOptions> for &str {
    fn from(option: UserMappingOptions) -> &'static str {
        match option {
            UserMappingOptions::Type => "TYPE",
            UserMappingOptions::Provider => "PROVIDER",
            UserMappingOptions::Scope => "SCOPE",
            UserMappingOptions::Chain => "CHAIN",
            UserMappingOptions::KeyId => "KEY_ID",
            UserMappingOptions::Secret => "SECRET",
            UserMappingOptions::Region => "REGION",
            UserMappingOptions::SessionToken => "SESSION_TOKEN",
            UserMappingOptions::Endpoint => "ENDPOINT",
            UserMappingOptions::UrlStyle => "URL_STYLE",
            UserMappingOptions::UseSsl => "USE_SSL",
            UserMappingOptions::UrlCompatibilityMode => "URL_COMPATIBILITY_MODE",
            UserMappingOptions::ConnectionString => "CONNECTION_STRING",
            UserMappingOptions::AccountName => "ACCOUNT_NAME",
            UserMappingOptions::TenantId => "TENANT_ID",
            UserMappingOptions::ClientId => "CLIENT_ID",
            UserMappingOptions::ClientSecret => "CLIENT_SECRET",
            UserMappingOptions::ClientCertificatePath => "CLIENT_CERTIFICATE_PATH",
            UserMappingOptions::HttpProxy => "HTTP_PROXY",
            UserMappingOptions::HttpUserName => "HTTP_USER_NAME",
            UserMappingOptions::HttpPassword => "HTTP_PASSWORD",
        }
    }
}

impl TryFrom<&str> for SecretType {
    type Error = anyhow::Error;

    fn try_from(secret_type: &str) -> Result<Self> {
        match secret_type.to_uppercase().as_str() {
            "AZURE" => Ok(Self::Azure),
            "S3" => Ok(Self::S3),
            "GCS" => Ok(Self::Gcs),
            "R2" => Ok(Self::R2),
            _ => bail!("Invalid secret type: {}", secret_type),
        }
    }
}

impl From<SecretType> for &str {
    fn from(secret_type: SecretType) -> &'static str {
        match secret_type {
            SecretType::Azure => "AZURE",
            SecretType::S3 => "S3",
            SecretType::Gcs => "GCS",
            SecretType::R2 => "R2",
        }
    }
}

impl TryFrom<&str> for Provider {
    type Error = anyhow::Error;

    fn try_from(provider: &str) -> Result<Self> {
        match provider.to_uppercase().as_str() {
            "CONFIG" => Ok(Self::Config),
            "CREDENTIAL_CHAIN" => Ok(Self::CredentialChain),
            "SERVICE_PRINCIPAL" => Ok(Self::ServicePrincipal),
            _ => bail!("Invalid provider: {}", provider),
        }
    }
}

impl From<Provider> for &str {
    fn from(provider: Provider) -> &'static str {
        match provider {
            Provider::Config => "CONFIG",
            Provider::CredentialChain => "CREDENTIAL_CHAIN",
            Provider::ServicePrincipal => "SERVICE_PRINCIPAL",
        }
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

    connection::execute(
        format!("CREATE OR REPLACE SECRET {secret_name} ({secret_string})").as_str(),
        [],
    )?;

    Ok(())
}
