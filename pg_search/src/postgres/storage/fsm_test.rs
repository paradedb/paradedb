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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::fsm::FreeSpaceManager;
use super::buffer::BufferManager;
use crate::Spi;
use pgrx::pg_test;
use pgrx::pg_sys;
use pgrx::pg_sys::BlockNumber;
use std::ops::Range;

#[pgrx::pg_schema]
mod tests {
    use super::*;
    
    #[pg_test]
    fn test_fsm_pushpop() {
        let relation_oid = init_bm25_index();
        let idxrel = PgSearchRelation::open(relation_oid);
        let root = unsafe { FreeSpaceManager::create(&idxrel) };
        let mut fsm = FreeSpaceManager::open(root);
        let mut bman = BufferManager::new(&idxrel);
        let r : Range<pg_sys::BlockNumber> = Range{start: 1, end: 1000};
        fsm.extend(&mut bman, r);
        let drained : Vec<pg_sys::BlockNumber> = fsm.drain(&mut bman, 2000).collect();
        let mut i = 1;
        for d in drained {
            if d != i {
                panic!("invalid block number {} != {}", d, i)
            }
            i += 1;
        }
        if i != 999 {
            panic!("wrong count of returned blocks: {}", i);
        }
    }
    
    fn init_bm25_index() -> pg_sys::Oid {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
            .expect("spi should succeed")
            .unwrap()
    }
}
