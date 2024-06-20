use anyhow::{bail, Result};

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
