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

// Collect fixture rows into columns for concise query-result assertions.
macro_rules! fixture_table {
    ($(#[$attr:meta])* pub struct $row:ident => $columns:ident {
        pub $first:ident: $first_ty:ty,
        $(pub $field:ident: $ty:ty,)*
    }) => {
        $(#[$attr])*
        pub struct $row {
            pub $first: $first_ty,
            $(pub $field: $ty,)*
        }

        #[derive(Debug, Default, PartialEq)]
        pub struct $columns {
            pub $first: Vec<$first_ty>,
            $(pub $field: Vec<$ty>,)*
        }

        impl FromIterator<$row> for $columns {
            fn from_iter<I: IntoIterator<Item = $row>>(rows: I) -> Self {
                let mut columns = Self::default();
                for row in rows {
                    columns.$first.push(row.$first);
                    $(columns.$field.push(row.$field);)*
                }
                columns
            }
        }

        impl $columns {
            pub fn len(&self) -> usize {
                self.$first.len()
            }

            pub fn is_empty(&self) -> bool {
                self.$first.is_empty()
            }
        }
    };
}

mod deliveries;
mod icu_amharic_posts;
mod icu_arabic_posts;
mod icu_czech_posts;
mod icu_greek_posts;
mod partitioned;
mod simple_products;

pub use deliveries::*;
pub use icu_amharic_posts::*;
pub use icu_arabic_posts::*;
pub use icu_czech_posts::*;
pub use icu_greek_posts::*;
pub use partitioned::*;
pub use simple_products::*;

#[cfg(test)]
mod tests {
    use super::{SimpleProductsTable, SimpleProductsTableVec};

    #[test]
    fn fixture_columns_preserve_row_order() {
        let columns: SimpleProductsTableVec = [
            SimpleProductsTable {
                id: 2,
                description: "second".into(),
                ..Default::default()
            },
            SimpleProductsTable {
                id: 1,
                description: "first".into(),
                ..Default::default()
            },
        ]
        .into_iter()
        .collect();

        assert_eq!(columns.id, vec![2, 1]);
        assert_eq!(columns.description, vec!["second", "first"]);
        assert_eq!(columns.category.len(), 2);
        assert_eq!(columns.len(), 2);
        assert!(!columns.is_empty());

        let empty: SimpleProductsTableVec = std::iter::empty().collect();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert!(empty.description.is_empty());
    }
}
