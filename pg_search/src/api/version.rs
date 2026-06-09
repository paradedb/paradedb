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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    #[allow(dead_code)]
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
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
