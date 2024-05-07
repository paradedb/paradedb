use std::collections::HashMap;

pub enum AmazonServerOption {
    Endpoint,
    Bucket,
    Root,
    Region,
    AllowAnonymous,
}

impl AmazonServerOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Endpoint => "endpoint",
            Self::Bucket => "bucket",
            Self::Root => "root",
            Self::Region => "region",
            Self::AllowAnonymous => "allow_anonymous",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Endpoint => false,
            Self::Bucket => true,
            Self::Root => false,
            Self::Region => false,
            Self::AllowAnonymous => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Endpoint,
            Self::Bucket,
            Self::Root,
            Self::Region,
            Self::AllowAnonymous,
        ]
        .into_iter()
    }
}

pub enum AmazonUserMappingOption {
    AccessKeyId,
    SecretAccessKey,
    SecurityToken,
}

impl AmazonUserMappingOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AccessKeyId => "access_key_id",
            Self::SecretAccessKey => "secret_access_key",
            Self::SecurityToken => "security_token",
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
