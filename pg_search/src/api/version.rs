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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[allow(dead_code)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

/// The first pg_search version in which datetime fields are stored as i64 microseconds from the
/// PG epoch (rather than tantivy `DateTime` nanoseconds from the Unix epoch). Used at read/write
/// time to gate which storage representation an index uses.
pub const DATETIME_I64_STORAGE_VERSION: Version = Version::new(0, 24, 1);

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Capability queries against an index's `created_by_version`. Implemented for both `Version`
/// and `Option<Version>` so callers can dispatch on `MetaPage::created_by_version()`'s return
/// type directly without an unwrap. `None` represents indexes built before version stamping
/// existed; for capability checks that gate new behavior, treat `None` as "does not have it".
pub trait VersionInfo {
    fn stores_datetimes_in_i64(&self) -> bool;
}
impl VersionInfo for Version {
    fn stores_datetimes_in_i64(&self) -> bool {
        self >= &DATETIME_I64_STORAGE_VERSION
    }
}
impl VersionInfo for Option<Version> {
    fn stores_datetimes_in_i64(&self) -> bool {
        self.filter(|v| v.stores_datetimes_in_i64()).is_some()
    }
}

/// Parses a decimal version component at compile time. Overflow or non-digit bytes
/// produce a compile error via the const-eval rules.
pub const fn parse_version_component(s: &str) -> u16 {
    let bytes = s.as_bytes();
    let mut result: u16 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        assert!(b >= b'0' && b <= b'9', "version component must be decimal");
        result = result * 10 + (b - b'0') as u16;
        i += 1;
    }
    result
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;

    #[test]
    fn version_ordering_is_lexicographic_on_components() {
        assert!(Version::new(0, 18, 0) < Version::new(0, 18, 1));
        assert!(Version::new(0, 18, 9) < Version::new(0, 19, 0));
        assert!(Version::new(0, 99, 99) < Version::new(1, 0, 0));
        assert_eq!(Version::new(1, 2, 3), Version::new(1, 2, 3));
    }
}
