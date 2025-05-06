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

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

/// `sqlsmith` generated a query that crashed due to us dereferencing a null pointer
///
/// The query itself it completely nonsensical but we shouldn't crash no matter what
/// the user (or sqlsmith) throws at us.
#[rstest]
fn crash_in_subquery(mut conn: PgConnection) {
    let result = r#"
        select
          pg_catalog.jsonb_build_array() as c0,
          (select high from paradedb.index_layer_info limit 1 offset 36)
             as c1
        from
          (select
                subq_1.c1 as c0,
                subq_1.c1 as c1
              from
                (select
                      ref_0.high as c0,
                      (select relname from paradedb.index_layer_info limit 1 offset 1)
                         as c1,
                      ref_0.byte_size as c2,
                      ref_0.layer_size as c3,
                      ref_0.byte_size as c4,
                      ref_0.segments as c5,
                      ref_0.byte_size as c6,
                      ref_0.relname as c7,
                      3 as c8
                    from
                      paradedb.index_layer_info as ref_0
                    where cast(null as int2) >= cast(null as int2)
                    limit 44) as subq_0,
                lateral (select
                      subq_0.c4 as c0,
                      (select segments from paradedb.index_layer_info limit 1 offset 4)
                         as c1
                    from
                      paradedb.index_layer_info as ref_1
                    where (cast(null as float8) >= cast(null as float8))
                      and (subq_0.c3 @@@ ref_1.relname)
                    limit 117) as subq_1
              where case when (select count from paradedb.index_layer_info limit 1 offset 3)
                       > cast(null as int2) then cast(nullif(cast(null as "time"),
                    cast(null as "time")) as "time") else cast(nullif(cast(null as "time"),
                    cast(null as "time")) as "time") end
                   <= pg_catalog.make_time(
                  cast(subq_0.c8 as int4),
                  cast(40 as int4),
                  cast(pg_catalog.pg_notification_queue_usage() as float8))
              limit 115) as subq_2
        where (select high from paradedb.index_layer_info limit 1 offset 3)
             is not NULL
        limit 53;
    "#
    .execute_result(&mut conn);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{err}")
        .contains("unable to determine Var relation as it belongs to a NULL subquery"))
}
