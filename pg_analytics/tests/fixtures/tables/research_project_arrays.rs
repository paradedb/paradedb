use pgrx::pg_sys::Oid;
use soa_derive::StructOfArray;
use sqlx::{
    types::{BigDecimal, Uuid},
    FromRow,
};
use time::{Date, PrimitiveDateTime};

use super::Table;

#[derive(Debug, PartialEq, FromRow, StructOfArray)]
pub struct ResearchProjectArraysTable {
    #[sqlx(skip)]
    pub project_id: Uuid,
    pub experiment_flags: Vec<bool>,
    #[sqlx(skip)]
    pub binary_data: Vec<Vec<u8>>,
    pub notes: Vec<String>,
    pub keywords: Vec<String>,
    pub short_descriptions: Vec<String>,
    pub participant_ages: Vec<i16>,
    pub participant_ids: Vec<i32>,
    pub observation_counts: Vec<i64>,
    #[sqlx(skip)]
    pub related_project_o_ids: Vec<Oid>,
    pub measurement_errors: Vec<f32>,
    pub precise_measurements: Vec<f64>,
    #[sqlx(skip)]
    pub observation_timestamps: Vec<PrimitiveDateTime>,
    #[sqlx(skip)]
    pub observation_dates: Vec<Date>,
    #[sqlx(skip)]
    pub budget_allocations: Vec<BigDecimal>,
    #[sqlx(skip)]
    pub participant_uuids: Vec<Uuid>,
}

impl Table for ResearchProjectArraysTable {
    fn setup_with() -> &'static str {
        RESEARCH_PROJECT_ARRAYS_TABLE_SETUP
    }
}

static RESEARCH_PROJECT_ARRAYS_TABLE_SETUP: &str = r#"
CREATE TABLE research_project_arrays (
    -- project_id UUID PRIMARY KEY,
    experiment_flags BOOLEAN[],
    -- binary_data BYTEA[],
    notes TEXT[],
    keywords VARCHAR[],
    short_descriptions BPCHAR[],
    participant_ages INT2[],
    participant_ids INT4[],
    observation_counts INT8[],
    -- related_project_o_ids OID[],
    measurement_errors FLOAT4[],
    precise_measurements FLOAT8[]
    -- observation_timestamps TIMESTAMP[],
    -- observation_dates DATE[],
    -- budget_allocations NUMERIC[],
    -- participant_uuids UUID[]
) USING deltalake;

INSERT INTO research_project_arrays (
    -- project_id,
    experiment_flags,
    -- binary_data,
    notes,
    keywords,
    short_descriptions,
    participant_ages,
    participant_ids,
    observation_counts,
    -- related_project_o_ids,
    measurement_errors,
    precise_measurements
    -- observation_timestamps,
    -- observation_dates
    -- budget_allocations,
    -- participant_uuids
)
VALUES
(
 -- 'a0ec8c90-9032-4f8f-87d3-6b76b4fadb02',
 ARRAY[true, false, true],
 --  ARRAY['\\xDEADBEEF'::bytea], 
 ARRAY['Initial setup complete', 'Preliminary results promising'],
 ARRAY['climate change', 'coral reefs'],
 ARRAY['CRLRST    ', 'OCEAN1    '],
 ARRAY[28, 34, 29], 
 ARRAY[101, 102, 103], 
 ARRAY[150, 120, 130], 
 -- ARRAY[1643, 1644, 1645]
 ARRAY[0.02, 0.03, 0.015], 
 ARRAY[1.5, 1.6, 1.7]
 -- ARRAY['2023-01-01 10:00:00', '2023-01-02 11:00:00', '2023-01-03 09:30:00']::timestamp[], 
 -- ARRAY['2023-01-01', '2023-01-02', '2023-01-03']::date[]
 -- ARRAY[10000.00, 5000.00, 7500.00], 
 -- ARRAY['d1ec8c90-9032-4f8f-87d3-6b76b4fa0001', 'd1ec8c90-9032-4f8f-87d3-6b76b4fa0002']::uuid[]
 ),

(
 -- 'b1fd9d22-2e5c-4af2-bf09-88f567abc123',
 ARRAY[false, true, false],
 -- ARRAY['\\xCAFEF00D'::bytea], 
 ARRAY['Need to re-evaluate methodology', 'Unexpected results in phase 2'],
 ARRAY['sustainable farming', 'soil health'],
 ARRAY['FARMEX    ', 'SOILQ2    '], 
 ARRAY[22, 27, 32], 
 ARRAY[201, 202, 203], 
 ARRAY[160, 140, 135], 
 -- ARRAY[2643, 2644, 2645],
 ARRAY[0.025, 0.02, 0.01], 
 ARRAY[2.0, 2.1, 2.2]
 -- ARRAY['2023-02-15 14:00:00', '2023-02-16 15:30:00', '2023-02-17 14:45:00']::timestamp[],
 -- ARRAY['2023-02-15', '2023-02-16', '2023-02-17']::date[],
 -- ARRAY[20000.00, 25000.00, 20000.00], 
 -- ARRAY['c2fd9d22-2e5c-4af2-bf09-88f567ab0003', 'c2fd9d22-2e5c-4af2-bf09-88f567ab0004']::uuid[]
);
"#;
