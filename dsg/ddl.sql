DROP TABLE IF EXISTS contacts_companies_combined_full CASCADE;
CREATE TABLE contacts_companies_combined_full (
    contact_id bigint NOT NULL,
    company_id bigint,
    contact_business_email character varying,
    contact_confirmed_connect_date date,
    contact_canonical_shorthand_name character varying,
    contact_logo_url character varying,
    contact_direct_number character varying,
    contact_mobile_phone character varying,
    contact_first_name character varying,
    contact_last_name character varying,
    contact_full_name character varying,
    contact_job_title character varying,
    contact_job_details jsonb,
    contact_locations_details jsonb,
    company_shorthand_name character varying,
    company_name character varying,
    company_industry character varying,
    company_sector character varying,
    company_sub_sectors jsonb,
    company_locations_details jsonb,
    company_domain character varying,
    company_phone character varying,
    company_emp_rev_details jsonb,
    employee_rank integer,
    revenue_rank integer,
    company_website character varying,
    company_linkedin_url character varying,
    company_logo_url character varying,
    company_type character varying,
    contact_hems character varying[],
    list_id integer[],
    contact_state_created_date date,
    CONSTRAINT contacts_companies_combined_full_pkey PRIMARY KEY (contact_id)
);


-- Use a transaction to ensure all or no data is inserted
BEGIN;

-- Create temporary arrays of random data for variety
DROP TABLE IF EXISTS first_names CASCADE;
CREATE TEMP TABLE first_names (name TEXT) ON COMMIT DROP;
INSERT INTO first_names VALUES ('jane'), ('alex'), ('sam'), ('charlie'), ('morgan'), ('casey'), ('jordan'), ('taylor'), ('riley');

DROP TABLE IF EXISTS last_names CASCADE;
CREATE TEMP TABLE last_names (name TEXT) ON COMMIT DROP;
INSERT INTO last_names VALUES ('smith'), ('jones'), ('williams'), ('brown'), ('davis'), ('miller'), ('wilson'), ('moore'), ('garcia');

DROP TABLE IF EXISTS topics CASCADE;
CREATE TEMP TABLE topics (name TEXT) ON COMMIT DROP;
INSERT INTO topics VALUES ('finance'), ('technology'), ('marketing'), ('logistics'), ('cybersecurity'), ('human resources'), ('saas'), ('cloud computing');

DROP TABLE IF EXISTS industries CASCADE;
CREATE TEMP TABLE industries (name TEXT) ON COMMIT DROP;
INSERT INTO industries VALUES ('Financial Services'), ('Information Technology'), ('Manufacturing'), ('Retail'), ('Consulting'), ('Transportation');

------------------------------------------------------------------
--
-- Tweakable Constants for Data Generation
--
------------------------------------------------------------------
DO $$
BEGIN
    -- Contacts Table Ratios
    PERFORM set_config('myvars.total_contacts', '10000000', false);
    PERFORM set_config('myvars.john_contact_count', '4000000', false); -- % of contacts that are "john"

    -- Intent Table Ratios
    PERFORM set_config('myvars.total_intents', '10000000', false);
    PERFORM set_config('myvars.healthcare_company_count', '500000', false); -- unique companies are in healthcare
    PERFORM set_config('myvars.duplicates_per_company', '100', false); -- Each healthcare company has N duplicate intent rows
END $$;


------------------------------------------------------------------
--
-- Populate `contacts_companies_combined_full`
--
------------------------------------------------------------------

-- GOAL: Create many contacts named "john" and ensure they work at the "health care" companies.
-- This overlap will force the JOIN to produce a large intermediate result set.
INSERT INTO contacts_companies_combined_full (
    contact_id, company_id, contact_first_name, contact_last_name, contact_full_name, company_name,
    contact_job_title, contact_job_details
)
SELECT
    series AS contact_id,
    -- Assign these "johns" to the "health care" companies to create overlap
    (series % current_setting('myvars.healthcare_company_count')::int) + 1 AS company_id,
    'john' AS contact_first_name,
    (SELECT name FROM last_names ORDER BY random() LIMIT 1) AS contact_last_name,
    'john ' || (SELECT name FROM last_names ORDER BY random() LIMIT 1) AS contact_full_name,
    'HealthCo ' || (series % current_setting('myvars.healthcare_company_count')::int) + 1 AS company_name,
    -- Populate job details for `dsg/query-aggregate.sql` for every 20th row (ensuring it's in list 'tjy3slfS5wk')
    CASE
        WHEN series % 20 = 0 THEN 'Senior Programmer'
        ELSE 'Other Job'
    END AS contact_job_title,
    CASE
        WHEN series % 20 = 0 THEN '{"job_function": "product management, research, & innovation", "job_area": "software development & engineering"}'::jsonb
        ELSE '{}'::jsonb
    END AS contact_job_details
FROM generate_series(1, current_setting('myvars.john_contact_count')::int) AS series;

-- Insert the remaining random contacts for other companies
INSERT INTO contacts_companies_combined_full (
    contact_id, company_id, contact_first_name, contact_last_name, contact_full_name, company_name
)
SELECT
    series + current_setting('myvars.john_contact_count')::int AS contact_id,
    -- Assign to non-healthcare companies
    series + current_setting('myvars.healthcare_company_count')::int AS company_id,
    (SELECT name FROM first_names ORDER BY random() LIMIT 1) AS contact_first_name,
    (SELECT name FROM last_names ORDER BY random() LIMIT 1) AS contact_last_name,
    (SELECT name FROM first_names ORDER BY random() LIMIT 1) || ' ' || (SELECT name FROM last_names ORDER BY random() LIMIT 1) AS contact_full_name,
    'RandomCorp ' || (series + current_setting('myvars.healthcare_company_count')::int) AS company_name
FROM generate_series(1, current_setting('myvars.total_contacts')::int - current_setting('myvars.john_contact_count')::int) AS series;



------------------------------------------------------------------
--
-- Populate `contact_list`
--
------------------------------------------------------------------

DROP TABLE IF EXISTS contact_list;
CREATE TABLE contact_list (
    id serial primary key,
    list_id character varying,
    original_value jsonb,
    ldf_id bigint,
    entity_type character varying,
    created_at timestamp(3) without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp(3) without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);

-- Populate the table with 10,000,000 rows
-- Split into two lists for querying: 'tjy3slfS5wk' and '21430'
INSERT INTO contact_list (list_id, ldf_id)
SELECT
    CASE WHEN series % 2 = 0 THEN 'tjy3slfS5wk' ELSE '21430' END,
    series
FROM
    generate_series(1, 10000000) AS series;


------------------------------------------------------------------
--
-- Populate `company_list`
--
------------------------------------------------------------------

DROP TABLE IF EXISTS company_list;
CREATE TABLE company_list (
    id serial primary key,
    list_id character varying,
    original_value jsonb,
    ldf_id bigint, -- Maps to company_id
    entity_type character varying,
    created_at timestamp(3) without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp(3) without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);

-- Populate with some companies in list '2543'
INSERT INTO company_list (list_id, ldf_id)
SELECT
    '2543',
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1
FROM
    generate_series(1, 5000000) AS series;


------------------------------------------------------------------
--
-- Populate `company_intent_autocomplete`
--
------------------------------------------------------------------

DROP TABLE IF EXISTS company_intent_autocomplete;
CREATE TABLE company_intent_autocomplete (
    unique_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id bigint,
    intent_topic text,
    score integer
);

-- Insert relevant data for query: "pre-employment & employee testing" with score 1-100
INSERT INTO company_intent_autocomplete (company_id, intent_topic, score)
SELECT
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1,
    'pre-employment & employee testing',
    (random() * 99)::int + 1
FROM
    generate_series(1, 500000) AS series;

-- Insert noise
INSERT INTO company_intent_autocomplete (company_id, intent_topic, score)
SELECT
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1,
    'other topic',
    (random() * 100)::int
FROM
    generate_series(1, 500000) AS series;


------------------------------------------------------------------
--
-- Populate `company_tech_install_autocomplete`
--
------------------------------------------------------------------

DROP TABLE IF EXISTS company_tech_install_autocomplete;
CREATE TABLE company_tech_install_autocomplete (
    unique_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id bigint,
    technology_name text
);

-- Insert relevant data for query: "salesforce"
INSERT INTO company_tech_install_autocomplete (company_id, technology_name)
SELECT
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1,
    'salesforce'
FROM
    generate_series(1, 500000) AS series;

-- Insert noise
INSERT INTO company_tech_install_autocomplete (company_id, technology_name)
SELECT
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1,
    'other tech'
FROM
    generate_series(1, 500000) AS series;


------------------------------------------------------------------
--
-- Populate `company_specialties_autocomplete`
--
------------------------------------------------------------------

DROP TABLE IF EXISTS company_specialties_autocomplete;
CREATE TABLE company_specialties_autocomplete (
    unique_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id bigint,
    speciality text
);

-- Insert relevant data for query: "salesforce"
INSERT INTO company_specialties_autocomplete (company_id, speciality)
SELECT
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1,
    'salesforce'
FROM
    generate_series(1, 500000) AS series;

-- Insert noise
INSERT INTO company_specialties_autocomplete (company_id, speciality)
SELECT
    (random() * current_setting('myvars.healthcare_company_count')::int)::int + 1,
    'other speciality'
FROM
    generate_series(1, 500000) AS series;


COMMIT;

------------------------------------------------------------------
--
-- Add Constraints and Indexes from Production
--
------------------------------------------------------------------

--
-- Name: cccf_company_id_contact_id_uix; Type: INDEX; Schema: public;
--
CREATE UNIQUE INDEX cccf_company_id_contact_id_uix ON contacts_companies_combined_full USING btree (company_id, contact_id);

--
-- pg_search BM25 index for advanced full-text search capabilities across multiple fields
--
-- NOTE: `sort_by` Configuration Tradeoff
-- We have configured this index with `sort_by = 'company_id ASC NULLS FIRST'`.
--
-- Rationale:
-- Four out of the six main query patterns (Company List, Intent, Tech Install, Specialties)
-- filter or join based on `company_id`. By physically sorting the index on `company_id`,
-- we enable the PostgreSQL planner to utilize efficient Merge Joins without an explicit
-- Sort step for these queries.
--
-- Tradeoff:
-- The `Contact List` query and the `Aggregate` query (dsg/query-aggregate.sql) filter
-- primarily on `contact_id`. Since the index is sorted by `company_id`, these specific
-- queries cannot leverage the pre-sorted order for Merge Joins on `contact_id`.
-- The planner will likely default to Hash Joins or require explicit Sort steps for these.
-- We prioritized optimizing the majority of the workload (company-based filters).
--
CREATE INDEX contacts_companies_combined_full_idx ON contacts_companies_combined_full
USING bm25 (contact_id, company_domain, company_industry, company_sector, company_sub_sectors, company_name, company_shorthand_name, contact_business_email, contact_canonical_shorthand_name, contact_first_name, contact_full_name, contact_job_title, contact_last_name, contact_mobile_phone, company_id, employee_rank, revenue_rank, company_emp_rev_details, company_locations_details, contact_job_details, contact_locations_details, contact_confirmed_connect_date)
WITH (
    key_field = contact_id,
    sort_by = 'company_id ASC NULLS FIRST',
    text_fields = '{
        "company_domain": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "company_industry": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}},
        "company_sector": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}},
        "company_name": {"fast": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "company_shorthand_name": {"fast": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_business_email": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_canonical_shorthand_name": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_first_name": {"fast": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_full_name": {"fast": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_job_title": {"fast": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_last_name": {"fast": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"},
        "contact_mobile_phone": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}, "normalizer": "lowercase"}
    }',
    numeric_fields = '{
        "company_id": {"fast": true, "indexed": true},
        "employee_rank": {"fast": true, "indexed": true},
        "revenue_rank": {"fast": true, "indexed": true}
    }',
    json_fields = '{
        "company_emp_rev_details": {"fast": true, "indexed": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}},
        "company_locations_details": {"fast": true, "indexed": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}},
        "company_sub_sectors": {"fast": true, "indexed": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}},
        "contact_job_details": {"fast": true, "indexed": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}},
        "contact_locations_details": { "fast": true, "indexed": true, "tokenizer": {"ascii_folding": true, "lowercase": true, "remove_long": 255, "type": "raw"}}
    }',
    datetime_fields = '{
        "contact_confirmed_connect_date": {"indexed": true}
    }'
);

--
-- Name: cccf_contact_canonical_shorthand_name_idx; Type: INDEX; Schema: public;
--
CREATE INDEX cccf_contact_canonical_shorthand_name_idx ON contacts_companies_combined_full USING btree (lower((contact_canonical_shorthand_name)::text));


--
-- Name: contact_list_list_id_idx; Type: INDEX; Schema: public;
--
CREATE INDEX contact_list_list_id_idx ON contact_list USING btree (list_id);


--
-- Name: contact_list_list_id_ldf_id_idx; Type: INDEX; Schema: public;
--
CREATE INDEX contact_list_list_id_ldf_id_idx ON contact_list USING btree (list_id, ldf_id);


--
-- Name: contacts_companies_combined_full_company_id; Type: INDEX; Schema: public;
--
CREATE INDEX contacts_companies_combined_full_company_id ON contacts_companies_combined_full USING btree (company_id);

CREATE INDEX contact_list_search_idx ON contact_list USING bm25 (id, list_id, ldf_id) WITH (key_field = 'id');
CREATE INDEX company_list_search_idx ON company_list USING bm25 (id, list_id, ldf_id) WITH (key_field = 'id');

--
-- Additional BM25 Indexes for autocomplete tables
--

CREATE INDEX company_intent_autocomplete_idx ON company_intent_autocomplete
USING bm25 (unique_id, intent_topic, score, company_id)
WITH (
    key_field = unique_id,
    sort_by = 'company_id ASC NULLS FIRST',
    text_fields = '{
        "intent_topic": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}}
    }',
    numeric_fields = '{
        "score": {"indexed": true},
        "company_id": {"fast": true}
    }'
);

CREATE INDEX company_tech_install_autocomplete_idx ON company_tech_install_autocomplete
USING bm25 (unique_id, technology_name, company_id)
WITH (
    key_field = unique_id,
    sort_by = 'company_id ASC NULLS FIRST',
    text_fields = '{
        "technology_name": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}}
    }',
    numeric_fields = '{
        "company_id": {"fast": true}
    }'
);

CREATE INDEX company_specialties_autocomplete_idx ON company_specialties_autocomplete
USING bm25 (unique_id, speciality, company_id)
WITH (
    key_field = unique_id,
    sort_by = 'company_id ASC NULLS FIRST',
    text_fields = '{
        "speciality": {"fast": true, "tokenizer": {"lowercase": true, "remove_long": 255, "type": "raw"}}
    }',
    numeric_fields = '{
        "company_id": {"fast": true}
    }'
);
