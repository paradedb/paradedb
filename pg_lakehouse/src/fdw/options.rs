// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::fdw::base::BaseFdwError;
use anyhow::{anyhow, Result};
use pgrx::*;
use std::collections::HashMap;
use supabase_wrappers::prelude::*;
use url::Url;

use crate::datafusion::format::TableFormat;

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
pub struct ObjectStoreConfig {
    url: Url,
    format: TableFormat,
    server_options: HashMap<String, String>,
    user_mapping_options: HashMap<String, String>,
}

impl ObjectStoreConfig {
    pub fn new(
        url: &Url,
        format: TableFormat,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Self {
        Self {
            url: url.clone(),
            format,
            server_options,
            user_mapping_options,
        }
    }

    pub fn format(&self) -> &TableFormat {
        &self.format
    }

    pub fn server_options(&self) -> &HashMap<String, String> {
        &self.server_options
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn user_mapping_options(&self) -> &HashMap<String, String> {
        &self.user_mapping_options
    }
}

pub trait ForeignOptions {
    fn table_options(&self) -> Result<HashMap<String, String>>;
    fn server_options(&self) -> Result<HashMap<String, String>>;
    fn user_mapping_options(&self) -> Result<HashMap<String, String>>;
}

impl ForeignOptions for PgRelation {
    fn table_options(&self) -> Result<HashMap<String, String>> {
        if !self.is_foreign_table() {
            return Ok(HashMap::new());
        }

        let foreign_table = unsafe { pg_sys::GetForeignTable(self.oid()) };
        let table_options = unsafe { options_to_hashmap((*foreign_table).options)? };

        Ok(table_options)
    }

    fn server_options(&self) -> Result<HashMap<String, String>> {
        if !self.is_foreign_table() {
            return Ok(HashMap::new());
        }

        let foreign_table = unsafe { pg_sys::GetForeignTable(self.oid()) };
        let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
        let server_options = unsafe { options_to_hashmap((*foreign_server).options)? };

        Ok(server_options)
    }

    fn user_mapping_options(&self) -> Result<HashMap<String, String>> {
        if !self.is_foreign_table() {
            return Ok(HashMap::new());
        }

        let foreign_table = unsafe { pg_sys::GetForeignTable(self.oid()) };
        let foreign_server = unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
        let user_mapping_options = unsafe { user_mapping_options(foreign_server) };

        Ok(user_mapping_options)
    }
}

pub fn validate_options(opt_list: Vec<Option<String>>, valid_options: Vec<String>) -> Result<()> {
    for opt in opt_list
        .iter()
        .flatten()
        .map(|opt| opt.split('=').next().unwrap_or(""))
    {
        if !valid_options.contains(&opt.to_string()) {
            return Err(anyhow!(
                "invalid option: {}. valid options are: {}",
                opt,
                valid_options.join(", ")
            ));
        }
    }

    Ok(())
}
