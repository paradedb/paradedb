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
