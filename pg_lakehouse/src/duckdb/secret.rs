use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;

pub enum Provider {
    Config,
    CredentialChain,
    ServicePrincipal,
}

pub enum SecretType {
    Azure,
    S3,
    Gcs,
    R2,
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
        match option.to_lowercase().as_str() {
            "type" => Ok(Self::Type),
            "provider" => Ok(Self::Provider),
            "scope" => Ok(Self::Scope),
            "chain" => Ok(Self::Chain),
            "key_id" => Ok(Self::KeyId),
            "secret" => Ok(Self::Secret),
            "region" => Ok(Self::Region),
            "session_token" => Ok(Self::SessionToken),
            "endpoint" => Ok(Self::Endpoint),
            "url_style" => Ok(Self::UrlStyle),
            "use_ssl" => Ok(Self::UseSsl),
            "url_compatibility_mode" => Ok(Self::UrlCompatibilityMode),
            "connection_string" => Ok(Self::ConnectionString),
            "account_name" => Ok(Self::AccountName),
            "tenant_id" => Ok(Self::TenantId),
            "client_id" => Ok(Self::ClientId),
            "client_secret" => Ok(Self::ClientSecret),
            "client_certificate_path" => Ok(Self::ClientCertificatePath),
            "http_proxy" => Ok(Self::HttpProxy),
            "http_user_name" => Ok(Self::HttpUserName),
            "http_password" => Ok(Self::HttpPassword),
            _ => bail!("Invalid user mapping option: {}", option),
        }
    }
}

impl From<UserMappingOptions> for &str {
    fn from(option: UserMappingOptions) -> &'static str {
        match option {
            UserMappingOptions::Type => "type",
            UserMappingOptions::Provider => "provider",
            UserMappingOptions::Scope => "scope",
            UserMappingOptions::Chain => "chain",
            UserMappingOptions::KeyId => "key_id",
            UserMappingOptions::Secret => "secret",
            UserMappingOptions::Region => "region",
            UserMappingOptions::SessionToken => "session_token",
            UserMappingOptions::Endpoint => "endpoint",
            UserMappingOptions::UrlStyle => "url_style",
            UserMappingOptions::UseSsl => "use_ssl",
            UserMappingOptions::UrlCompatibilityMode => "url_compatibility_mode",
            UserMappingOptions::ConnectionString => "connection_string",
            UserMappingOptions::AccountName => "account_name",
            UserMappingOptions::TenantId => "tenant_id",
            UserMappingOptions::ClientId => "client_id",
            UserMappingOptions::ClientSecret => "client_secret",
            UserMappingOptions::ClientCertificatePath => "client_certificate_path",
            UserMappingOptions::HttpProxy => "http_proxy",
            UserMappingOptions::HttpUserName => "http_user_name",
            UserMappingOptions::HttpPassword => "http_password",
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
) -> Result<String> {
    let secret_type = SecretType::try_from(
        user_mapping_options
            .get(UserMappingOptions::Type.into())
            .ok_or_else(|| anyhow!("type option required for USER MAPPING"))?
            .as_str(),
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

    Ok(format!(
        "CREATE OR REPLACE SECRET {secret_name} ({secret_string})"
    ))
}
