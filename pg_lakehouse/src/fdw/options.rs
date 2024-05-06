use std::collections::HashMap;

pub enum AmazonServerOption {
    Endpoint,
    Url,
    Region,
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
            Self::SessionToken,
            Self::AllowHttp,
            Self::SkipSignature,
        ]
        .into_iter()
    }
}

pub enum AmazonUserMappingOption {
    AccessKeyId,
    SecretAccessKey,
    SessionToken,
}

impl AmazonUserMappingOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AccessKeyId => "access_key_id",
            Self::SecretAccessKey => "secret_access_key",
            Self::SessionToken => "session_token",
        }
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

#[derive(Clone, Debug)]
pub struct ServerOptions {
    server_options: HashMap<String, String>,
    user_mapping_options: HashMap<String, String>,
}

impl ServerOptions {
    pub fn new(
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Self {
        Self {
            server_options,
            user_mapping_options,
        }
    }

    pub fn server_options(&self) -> &HashMap<String, String> {
        &self.server_options
    }

    pub fn user_mapping_options(&self) -> &HashMap<String, String> {
        &self.user_mapping_options
    }
}
