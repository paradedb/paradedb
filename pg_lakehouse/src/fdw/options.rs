use crate::fdw::base::BaseFdwError;
use std::collections::HashMap;

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

pub fn validate_options(
    opt_list: Vec<Option<String>>,
    valid_options: Vec<String>,
) -> Result<(), BaseFdwError> {
    for opt in opt_list
        .iter()
        .flatten()
        .map(|opt| opt.split("=").next().unwrap_or(""))
    {
        if !valid_options.contains(&opt.to_string()) {
            return Err(BaseFdwError::InvalidOption(opt.to_string(), valid_options));
        }
    }

    Ok(())
}
