use object_store::aws::{AmazonS3, AmazonS3Builder};
use std::collections::HashMap;
use supabase_wrappers::prelude::require_option;
use thiserror::Error;

pub enum ServerOption {
    Endpoint,
    Url,
    Region,
    AccessKeyId,
    SecretAccessKey,
    SessionToken,
    AllowHttp,
}

pub enum TableOption {
    Url,
    Format,
}

impl ServerOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Endpoint => "endpoint",
            Self::Url => "url",
            Self::Region => "region",
            Self::AccessKeyId => "access_key_id",
            Self::SecretAccessKey => "secret_access_key",
            Self::SessionToken => "session_token",
            Self::AllowHttp => "allow_http",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Endpoint => false,
            Self::Url => true,
            Self::Region => true,
            Self::AccessKeyId => false,
            Self::SecretAccessKey => false,
            Self::SessionToken => false,
            Self::AllowHttp => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Endpoint,
            Self::Url,
            Self::Region,
            Self::AccessKeyId,
            Self::SecretAccessKey,
            Self::SessionToken,
            Self::AllowHttp,
        ]
        .into_iter()
    }
}

impl TableOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Url => "url",
            Self::Format => "format",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Url => true,
            Self::Format => true,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::Url, Self::Format].into_iter()
    }
}

#[derive(Clone, Debug)]
pub struct ServerOptions(pub HashMap<String, String>);

impl TryFrom<ServerOptions> for AmazonS3 {
    type Error = OptionsError;

    fn try_from(options: ServerOptions) -> Result<Self, Self::Error> {
        let ServerOptions(options) = options;

        let url = require_option(ServerOption::Url.as_str(), &options)?;
        let region = require_option(ServerOption::Region.as_str(), &options)?;

        let mut builder = AmazonS3Builder::new().with_url(url).with_region(region);

        if let Some(access_key_id) = options.get(ServerOption::AccessKeyId.as_str()) {
            builder = builder.clone().with_access_key_id(access_key_id.as_str());
        }

        if let Some(secret_access_key) = options.get(ServerOption::SecretAccessKey.as_str()) {
            builder = builder.with_secret_access_key(secret_access_key.as_str());
        }

        if let Some(session_token) = options.get(ServerOption::SessionToken.as_str()) {
            builder = builder.with_token(session_token.as_str());
        }

        if let Some(endpoint) = options.get(ServerOption::Endpoint.as_str()) {
            builder = builder.with_endpoint(endpoint.as_str());
        }

        if let Some(allow_http) = options.get(ServerOption::AllowHttp.as_str()) {
            if allow_http == "true" {
                builder = builder.with_allow_http(true);
            }
        }

        Ok(builder.build()?)
    }
}

#[derive(Error, Debug)]
pub enum OptionsError {
    #[error(transparent)]
    Option(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),
}
