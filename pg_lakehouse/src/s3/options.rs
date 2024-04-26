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
    // Tech Debt: Pass Postgres table and schema name directly into begin_scan()
    Table,
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
            Self::Table => "table",
            Self::Format => "format",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Url => true,
            Self::Table => true,
            Self::Format => true,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::Url, Self::Table, Self::Format].into_iter()
    }
}
