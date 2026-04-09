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
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
pub struct DatasetConfig {
    pub root_table: String,
    pub sampling_seed: u64,
    pub tables: Vec<TableConfig>,
    #[serde(default)]
    pub s3_base_path: Option<String>,
}

#[derive(Deserialize)]
pub struct TableConfig {
    pub name: String,
    pub parent: Option<String>,
    pub parent_join_col: Option<String>,
    pub join_col: Option<String>,
}

pub fn load_dataset_config(path: &str) -> Result<DatasetConfig> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read config '{path}'"))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse config '{path}'"))
}

/// Returns table indices in topological order (root first, then children).
pub fn topological_order(config: &DatasetConfig) -> Result<Vec<usize>> {
    let name_to_idx: HashMap<&str, usize> = config
        .tables
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();

    let mut order = Vec::with_capacity(config.tables.len());
    let mut processed: HashSet<&str> = HashSet::new();

    // Start with the root table.
    let root_idx = *name_to_idx
        .get(config.root_table.as_str())
        .with_context(|| {
            format!(
                "Root table '{}' not found in tables list",
                config.root_table
            )
        })?;
    if config.tables[root_idx].parent.is_some() {
        bail!(
            "Root table '{}' cannot have a parent table.",
            config.root_table
        )
    }
    order.push(root_idx);
    processed.insert(&config.root_table);

    // Iteratively add tables whose parent has been processed. Repeat until no progress is made
    let mut progress = true;
    while progress {
        progress = false;
        for (i, table) in config.tables.iter().enumerate() {
            if processed.contains(table.name.as_str()) {
                continue;
            }
            if let Some(parent) = &table.parent {
                if processed.contains(parent.as_str()) {
                    // Validate that child tables have the required keys.
                    if table.parent_join_col.is_none() || table.join_col.is_none() {
                        bail!(
                            "Table '{}' has a parent '{}' but is missing parent_join_col or join_col",
                            table.name,
                            parent
                        );
                    }
                    order.push(i);
                    processed.insert(&table.name);
                    progress = true;
                }
            } else if table.name != config.root_table {
                bail!(
                    "Table '{}' has no parent and is not the root table '{}'",
                    table.name,
                    config.root_table
                );
            }
        }
    }

    // Check for unprocessed tables (cycle or missing parent).
    for table in &config.tables {
        if !processed.contains(table.name.as_str()) {
            bail!(
                "Table '{}' could not be processed. Its parent '{}' is not in the config or there is a cycle.",
                table.name,
                table.parent.as_deref().unwrap_or("(none)")
            );
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(name: &str, parent: Option<&str>) -> TableConfig {
        TableConfig {
            name: name.to_string(),
            parent: parent.map(|s| s.to_string()),
            parent_join_col: parent.map(|_| "parent_id".to_string()),
            join_col: parent.map(|_| "id".to_string()),
        }
    }

    fn make_config(root_table: &str, tables: Vec<TableConfig>) -> DatasetConfig {
        DatasetConfig {
            root_table: root_table.to_string(),
            sampling_seed: 42,
            tables,
            s3_base_path: None,
        }
    }

    #[test]
    fn single_root_table() {
        let config = make_config("orders", vec![make_table("orders", None)]);
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn root_with_one_child() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("orders")),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn root_with_multiple_children() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("orders")),
                make_table("payments", Some("orders")),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order[0], 0); // root first
        let rest: HashSet<usize> = order[1..].iter().copied().collect();
        assert_eq!(rest, HashSet::from([1, 2]));
    }

    #[test]
    fn multi_level_hierarchy() {
        // orders -> line_items -> shipments
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("orders")),
                make_table("shipments", Some("line_items")),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn tables_listed_in_reverse_dependency_order() {
        // Config lists child before parent — should still resolve correctly.
        let config = make_config(
            "orders",
            vec![
                make_table("shipments", Some("line_items")),
                make_table("line_items", Some("orders")),
                make_table("orders", None),
            ],
        );
        let order = topological_order(&config).unwrap();
        // Root (orders=idx2) must come first, then line_items(idx1), then shipments(idx0).
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn error_root_table_not_in_list() {
        let config = make_config("missing", vec![make_table("orders", None)]);
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_root_table_has_parent() {
        let config = make_config("orders", vec![make_table("orders", Some("something"))]);
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_non_root_without_parent() {
        let config = make_config(
            "orders",
            vec![make_table("orders", None), make_table("orphan", None)],
        );
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_missing_parent_reference() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("nonexistent")),
            ],
        );
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_child_missing_join_cols() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                TableConfig {
                    name: "line_items".to_string(),
                    parent: Some("orders".to_string()),
                    parent_join_col: None,
                    join_col: None,
                },
            ],
        );
        assert!(topological_order(&config).is_err());
    }
}
