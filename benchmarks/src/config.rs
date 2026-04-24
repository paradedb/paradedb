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

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
pub struct DatasetConfig {
    pub root_table: RootTableConfig,
    pub sampling_seed: u64,
    pub tables: Vec<TableConfig>,
    #[serde(default)]
    pub s3_base_path: Option<String>,
}

#[derive(Deserialize)]
pub struct RootTableConfig {
    pub name: String,
    pub _primary_key: String,
}

#[derive(Deserialize, Clone)]
pub struct TableConfig {
    pub name: String,
    pub parent: String,
    pub parent_join_col: String,
    pub join_col: String,
}

pub fn load_dataset_config(path: &str) -> Result<DatasetConfig> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read config '{path}'"))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse config '{path}'"))
}

/// Returns table indices in topological order (children only, excludes root).
pub fn topological_order(config: &DatasetConfig) -> Result<Vec<usize>> {
    let mut order = Vec::with_capacity(config.tables.len() + 1);
    let mut processed: HashSet<&str> = HashSet::new();

    // Start with the root table.
    processed.insert(&config.root_table.name);

    // Iteratively add tables whose parent has been processed. Repeat until no progress is made
    let mut progress = true;
    while progress {
        progress = false;
        for (i, table) in config.tables.iter().enumerate() {
            if processed.contains(table.name.as_str()) {
                continue;
            }
            if processed.contains(table.parent.as_str()) {
                // Validate that child tables have the required keys.
                order.push(i);
                processed.insert(&table.name);
                progress = true;
            }
        }
    }

    // Check for unprocessed tables (cycle or missing parent).
    for table in &config.tables {
        if !processed.contains(table.name.as_str()) {
            bail!(
                "Table '{}' could not be processed. Its parent '{}' is not in the config or there is a cycle.",
                table.name,
                table.parent,
            );
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(name: &str, parent: &str) -> TableConfig {
        TableConfig {
            name: name.to_string(),
            parent: parent.to_string(),
            parent_join_col: "parent_id".to_string(),
            join_col: "id".to_string(),
        }
    }

    fn make_config(root_table: &str, tables: Vec<TableConfig>) -> DatasetConfig {
        DatasetConfig {
            root_table: RootTableConfig {
                name: root_table.to_string(),
                _primary_key: format!("{root_table}_pk"),
            },
            sampling_seed: 42,
            tables,
            s3_base_path: None,
        }
    }

    #[test]
    fn single_root_table() {
        let config = make_config("orders", vec![]);
        let order = topological_order(&config).unwrap();
        assert_eq!(order, Vec::<usize>::new());
    }

    #[test]
    fn root_with_one_child() {
        let config = make_config("orders", vec![make_table("line_items", "orders")]);
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn root_with_multiple_children() {
        let config = make_config(
            "orders",
            vec![
                make_table("line_items", "orders"),
                make_table("payments", "orders"),
            ],
        );
        let order = topological_order(&config).unwrap();
        let rest: HashSet<usize> = order[..].iter().copied().collect();
        assert_eq!(rest, HashSet::from([0, 1]));
    }

    #[test]
    fn multi_level_hierarchy() {
        // orders -> line_items -> shipments
        let config = make_config(
            "orders",
            vec![
                make_table("line_items", "orders"),
                make_table("shipments", "line_items"),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn tables_listed_in_reverse_dependency_order() {
        // Config lists child before parent — should still resolve correctly.
        let config = make_config(
            "orders",
            vec![
                make_table("shipments", "line_items"),
                make_table("line_items", "orders"),
            ],
        );
        let order = topological_order(&config).unwrap();
        // Root (orders=idx2) must come first, then line_items(idx1), then shipments(idx0).
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn error_missing_parent_reference() {
        let config = make_config("orders", vec![make_table("line_items", "nonexistent")]);
        assert!(topological_order(&config).is_err());
    }
}
