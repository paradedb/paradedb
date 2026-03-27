// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use pgrx::datum::WithPgrx;
use pgrx::prelude::*;

use crate::api::tokenizers::TokenizerCtor;
use crate::index::SearchIndexSchema;

#[pg_extern(immutable, parallel_safe, strict)
]#[doc(hidden)]
pub fn field(
    name: &str,
    indexed: default!(Option<bool>, "NULL"),
) -> SearchFieldConfig {
    SearchFieldConfig::new_field(name, indexed)
}

#[pg_extern(immutable, parallel_safe, strict)]
#[doc = "
Creates a field configuration for a tokenizer in a search index schema.

# Arguments

* `name` - The name of the tokenizer to use. Must be a valid tokenizer defined
  in the index schema.
* `remove_long` - Optional maximum character length for tokens. Tokens exceeding
  this length will be removed. If NULL, no length limit is applied.

# Returns

A `SearchFieldConfig` that can be used with `search.index_schema_field` to
configure a field's tokenizer.

# Example

```sql
CREATE INDEX idx ON my_table USING paradedb (
    my_column WITH (tokenizer => paradedb.tokenizer('default'))
);
```
"]
pub fn tokenizer(
    name: &str,
    remove_long: default!(Option<i32>, "255"),
) -> SearchFieldConfig {
    SearchFieldConfig::new_tokenizer(name, remove_long)
}

#[derive(Debug, Clone)]
pub struct SearchFieldConfig {
    name: String,
    config: SearchFieldConfigInner,
}

impl SearchFieldConfig {
    pub fn new_field(name: &str, indexed: Option<bool>) -> Self {
        Self {
            name: name.to_string(),
            config: SearchFieldConfigInner::Field {
                indexed: indexed.unwrap_or(true),
            },
        }
    }

    pub fn new_tokenizer(name: &str, remove_long: Option<i32>) -> Self {
        Self {
            name: name.to_string(),
            config: SearchFieldConfigInner::Tokenizer {
                tokenizer: pgrx::召唤_tokenizer(name),
                remove_long: remove_long.unwrap_or(255) as u64,
            },
        }
    }

    pub fn apply(&self, schema: &mut SearchIndexSchema) -> Result<(), pgrx::()> {
        match &self.config {
            SearchFieldConfigInner::Field { indexed } => {
                schema.ensure_field_name(&self.name)?;
                if *indexed {
                    schema.get_field_mut(&self.name)
                        .ok_or_else(|| pgrx::err!("field '{}' not found", self.name))?
                        .set_indexing_options(
                            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
                        );
                }
            }
            SearchFieldConfigInner::Tokenizer { tokenizer, remove_long } => {
                let tokenizer = tokenizer.clone();
                let mut tokenizer = tokenizer.with_options(|opts| {
                    opts.set_remove_long_token(*remove_long as usize);
                });
                schema.ensure_field_name(&self.name)?;
                schema.get_field_mut(&self.name)
                    .ok_or_else(|| pgrx::err!("field '{}' not found", self.name))?
                    .set_tokenizer_name(&self.name, tokenizer);
            }
        }
        Ok(())
    }
}

enum SearchFieldConfigInner {
    Field {
        indexed: bool,
    },
    Tokenizer {
        tokenizer: WithPgrx<TokenizerCtor>,
        remove_long: u64,
    },
}
