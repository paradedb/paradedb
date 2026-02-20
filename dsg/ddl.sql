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
    contact_id, company_id, contact_first_name, contact_last_name, contact_full_name, company_name
)
SELECT
    series AS contact_id,
    -- Assign these "johns" to the "health care" companies to create overlap
    (series % current_setting('myvars.healthcare_company_count')::int) + 1 AS company_id,
    'john' AS contact_first_name,
    (SELECT name FROM last_names ORDER BY random() LIMIT 1) AS contact_last_name,
    'john ' || (SELECT name FROM last_names ORDER BY random() LIMIT 1) AS contact_full_name,
    'HealthCo ' || (series % current_setting('myvars.healthcare_company_count')::int) + 1 AS company_name
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
    list_id character varying,
    original_value jsonb,
    ldf_id bigint,
    entity_type character varying,
    created_at timestamp(3) without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp(3) without time zone DEFAULT timezone('utc'::text, now()) NOT NULL
);

-- Populate the table with 10,000,000 rows
INSERT INTO contact_list (list_id, ldf_id)
SELECT
    'tjy3slfS5wk',
    series
FROM
    generate_series(1, 10000000) AS series;


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
CREATE INDEX contacts_companies_combined_full_idx ON contacts_companies_combined_full
USING bm25 (contact_id, company_domain, company_industry, company_sector, company_sub_sectors, company_name, company_shorthand_name, contact_business_email, contact_canonical_shorthand_name, contact_first_name, contact_full_name, contact_job_title, contact_last_name, contact_mobile_phone, company_id, employee_rank, revenue_rank, company_emp_rev_details, company_locations_details, contact_job_details, contact_locations_details, contact_confirmed_connect_date)
WITH (
    key_field = contact_id,
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
        "company_id": {"indexed": true},
        "employee_rank": {"indexed": true},
        "revenue_rank": {"indexed": true}
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
