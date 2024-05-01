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
    SkipSignature,
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
            Self::SkipSignature => "skip_signature",
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
            Self::SkipSignature => false,
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
            Self::SkipSignature,
        ]
        .into_iter()
    }
}

pub enum TableOption {
    Path,
    Extension,
    Format,
}

impl TableOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Path => "path",
            Self::Extension => "extension",
            Self::Format => "format",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Path => true,
            Self::Extension => true,
            Self::Format => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::Path, Self::Extension, Self::Format].into_iter()
    }
}

/// These functions are pulled from supabase-wrappers.
/// We pulled them out because supabase-wrappers depends on pgrx 0.11.3
/// and pg_analytics uses 0.12.0-alpha.1. Once this is no longer the case we
/// can remove these functions and use the ones from supabase-wrappers
#[inline]
pub fn require_option<'map>(
    opt_name: &str,
    options: &'map HashMap<String, String>,
) -> Result<&'map str, OptionsError> {
    options
        .get(opt_name)
        .map(|t| t.as_ref())
        .ok_or_else(|| OptionsError::OptionNameNotFound(opt_name.to_string()))
}

/// Taken from supabase-wrappers
#[inline]
pub fn require_option_or<'a>(
    opt_name: &str,
    options: &'a HashMap<String, String>,
    default: &'a str,
) -> &'a str {
    options.get(opt_name).map(|t| t.as_ref()).unwrap_or(default)
}

#[derive(Clone, Debug)]
pub struct ServerOptions(pub HashMap<String, String>);

#[derive(Error, Debug)]
pub enum OptionsError {
    #[error("Option name not found: {0}")]
    OptionNameNotFound(String),
}
