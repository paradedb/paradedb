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

pub mod github;
pub mod gucs;
pub mod trace;

#[cfg(feature = "fixtures")]
pub mod fixtures;

// We need to re-export the dependencies below, because they're used by our public macros.
// This lets consumers of the macros use them without needing to also install these dependencies.
pub use pgrx;
pub use serde_json;
pub use trace::init_ereport_logger;
pub use tracing;
pub use tracing_subscriber;
