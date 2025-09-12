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

#[pgrx::pg_schema]
mod tests {
    use super::*;

    #[pg_test] fn fsm_pushpop_small() { pushpop_n(10, 123) }
    #[pg_test] fn fsm_pushpop_medium() { pushpop_n(500, 124) }
    #[pg_test] fn fsm_pushpop_large() { pushpop_n(1000, 125) }
    #[pg_test] fn fsm_pushpop_huge() { pushpop_n(10000, 126) }

    #[pg_test]
    fn fsm_big() {
        let (mut bman, mut fsm) = init();

        let extend_when = pg_sys::TransactionId::from(100);
        fsm.extend_with_when_recyclable(&mut bman, extend_when, 0..100_000);

        let drain_when = pg_sys::TransactionId::from(101);
        let drained : Vec<_> = fsm.drain_at(&mut bman, drain_when, 100_000).collect();
        assert_eq!(drained.len(), 100_000);
    }

    fn pushpop_n(n : usize, xid_n : u32) {
        let (mut bman, mut fsm) = init();
        let xid = pg_sys::TransactionId::from(xid_n);

        let blocks: Vec<BlockNumber> = (1..=n as BlockNumber).collect();
        fsm.extend_with_when_recyclable(&mut bman, xid, blocks.clone().into_iter());
        let mut drained: Vec<BlockNumber> = fsm.drain(&mut bman, n + 10).collect();
        drained.sort();

        assert_eq!(drained, blocks);
    }

    #[pg_test]
    fn fsm_empty() {
        let (mut bman, mut fsm) = init();

        let drained: Vec<BlockNumber> = fsm.drain(&mut bman, 10).collect();
        assert_eq!(drained.len(), 0);
    }

    #[pg_test]
    fn fsm_multi_xid() {
        let (mut bman, mut fsm) = init();

        let xid1 = pg_sys::TransactionId::from(100);
        let xid2 = pg_sys::TransactionId::from(101);

        fsm.extend_with_when_recyclable(&mut bman, xid1, vec![1, 2, 3].into_iter());
        fsm.extend_with_when_recyclable(&mut bman, xid2, vec![4, 5, 6].into_iter());

        let mut drained: Vec<BlockNumber> = fsm.drain(&mut bman, 10).collect();
        drained.sort();

        let expected = vec![1, 2, 3, 4, 5, 6];

        assert_eq!(drained, expected);
    }

    #[pg_test]
    fn fsm_partial_drain() {
        let (mut bman, mut fsm) = init();
        let xid = pg_sys::TransactionId::from(100);

        let blocks: Vec<BlockNumber> = (1..=10).collect();
        fsm.extend_with_when_recyclable(&mut bman, xid, blocks.into_iter());

        let drained1: Vec<BlockNumber> = fsm.drain(&mut bman, 5).collect();
        assert_eq!(drained1.len(), 5);

        let drained2: Vec<BlockNumber> = fsm.drain(&mut bman, 10).collect();
        assert_eq!(drained2.len(), 5);

        let mut all_drained = drained1;
        all_drained.extend(drained2);
        all_drained.sort();

        assert_eq!(all_drained, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[pg_test]
    fn fsm_lifecycle() {
        let (mut bman, mut fsm) = init();
        let xid = pg_sys::TransactionId::from(100);

        // we don't guarantee order of drains, so we can't check for vector
        // equality here.
        fsm.extend_with_when_recyclable(&mut bman, xid, vec![1, 2].into_iter());
        let first_drain: Vec<BlockNumber> = fsm.drain(&mut bman, 5).collect();
        assert_eq!(first_drain.len(), 2);

        fsm.extend_with_when_recyclable(&mut bman, xid, vec![3, 4, 5].into_iter());
        let second_drain: Vec<BlockNumber> = fsm.drain(&mut bman, 5).collect();
        assert_eq!(second_drain.len(), 3);

        let third_drain: Vec<BlockNumber> = fsm.drain(&mut bman, 5).collect();
        assert_eq!(third_drain.len(), 0);
    }

    #[pg_test]
    fn fsm_interleaved() {
        let (mut bman, mut fsm) = init();
        let xid = pg_sys::TransactionId::from(100);

        fsm.extend_with_when_recyclable(&mut bman, xid, vec![1, 2].into_iter());

        let pop1 = fsm.pop(&mut bman);
        assert!(pop1.is_some());

        fsm.extend_with_when_recyclable(&mut bman, xid, vec![3, 4, 5].into_iter());

        let remaining: Vec<BlockNumber> = fsm.drain(&mut bman, 10).collect();
        assert_eq!(remaining.len(), 4);
    }

    // Tests XID ordering with out-of-order inserts and varying horizons
    #[pg_test]
    fn fsm_xid_ordering() {
        let (mut bman, mut fsm) = init();

        let xid1 = pg_sys::TransactionId::from(105);
        let xid2 = pg_sys::TransactionId::from(102);
        let xid3 = pg_sys::TransactionId::from(108);
        let xid4 = pg_sys::TransactionId::from(103);

        fsm.extend_with_when_recyclable(&mut bman, xid1, vec![10, 11].into_iter());
        fsm.extend_with_when_recyclable(&mut bman, xid2, vec![20, 21].into_iter());
        fsm.extend_with_when_recyclable(&mut bman, xid3, vec![30, 31].into_iter());
        fsm.extend_with_when_recyclable(&mut bman, xid4, vec![40, 41].into_iter());

        let horizon1 = pg_sys::TransactionId::from(104);
        let drained1 : Vec<_> = fsm.drain_at(&mut bman, horizon1, 10).collect();
        assert_eq!(drained1.len(), 4);

        let horizon2 = pg_sys::TransactionId::from(106);
        let drained2 : Vec<_> = fsm.drain_at(&mut bman, horizon2, 10).collect();
        assert_eq!(drained2.len(), 2);

        let horizon3 = pg_sys::TransactionId::from(110);
        let drained3 : Vec<_> = fsm.drain_at(&mut bman, horizon3, 10).collect();
        assert_eq!(drained3.len(), 2);

        let mut all_drained = drained1;
        all_drained.extend(drained2);
        all_drained.extend(drained3);
        all_drained.sort();
        let expected = vec![10, 11, 20, 21, 30, 31, 40, 41];
        assert_eq!(all_drained, expected);
    }

    fn init() -> (BufferManager, FreeSpaceManager) {
        let relation_oid = init_bm25_index();
        let idxrel = PgSearchRelation::open(relation_oid);
        let root = unsafe { FreeSpaceManager::create(&idxrel) };
        let bman = BufferManager::new(&idxrel);
        let fsm = FreeSpaceManager::open(root);
        (bman, fsm)
    }

    fn init_bm25_index() -> pg_sys::Oid {
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id')").unwrap();
        Spi::get_one("SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';")
            .expect("spi should succeed")
            .unwrap()
    }
}
