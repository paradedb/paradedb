use datafusion::common::DataFusionError;
use pgrx::*;
use std::collections::HashMap;
use thiserror::Error;

pub enum AmazonServerOption {
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

impl AmazonServerOption {
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

#[derive(Error, Debug)]
pub enum OptionsError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    Option(#[from] supabase_wrappers::options::OptionsError),

    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),
}
