// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use tantivy::aggregation::bucket::SerializableQuery;
use tantivy::query::{EnableScoring, Query, Weight};

#[derive(Debug)]
pub struct FilterQuery(pub Box<dyn Query>);

impl Clone for FilterQuery {
    fn clone(&self) -> Self {
        Self(self.0.box_clone())
    }
}

impl Query for FilterQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        self.0.weight(enable_scoring)
    }
}

impl SerializableQuery for FilterQuery {
    fn clone_box(&self) -> Box<dyn SerializableQuery> {
        Box::new(self.clone())
    }
}

impl serde::Serialize for FilterQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        unimplemented!("serializing FilterQuery is not supported in 0.19.x")
    }
}
