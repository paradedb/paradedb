# Data Generation and Query Benchmark Suite

This directory (`dsg/`) contains a synthetic dataset generator and a set of SQL queries designed to benchmark and validate `paradedb`'s performance, particularly focusing on full-text search combined with complex joins and filtering.

## Dataset Generation (`ddl.sql`)

The `ddl.sql` script sets up the schema and populates tables with synthetic data.

### Key Tables

- **`contacts_companies_combined_full`**: The primary table with ~10 million rows, representing contacts denormalized with their company information.
- **`contact_list`**: A mapping table linking contacts to specific lists (e.g., `list_id = 'tjy3slfS5wk'`).
- **`company_list`**: A mapping table linking companies to specific lists.
- **Autocomplete Tables**: `company_intent_autocomplete`, `company_tech_install_autocomplete`, and `company_specialties_autocomplete`. These are used for "semi-join" style filtering (e.g., "Find contacts at companies that use Salesforce").

### Data Characteristics

- **Skew**: The data generation introduces intentional skew. A large number of contacts named "John" are assigned to a subset of "HealthCo" companies. This is designed to stress test join performance and intermediate result set handling.
- **Targeted Matches**: Specific rows are generated with exact strings (e.g., job titles like "Senior Programmer", specialties like "salesforce") to ensuring that the provided benchmark queries return non-empty results.

### Index Configuration & Optimization

The `contacts_companies_combined_full` table has a BM25 index configured with:

- `sort_by = 'company_id ASC NULLS FIRST'`
- `numeric_fields` including `company_id`, `revenue_rank`, and `employee_rank` marked as `fast: true`.

**The Tradeoff:**

- **Optimized:** Queries joining on `company_id` (Intent, Tech Install, Specialties, Company List) benefit from the physical index sort order, allowing the Postgres planner to utilize efficient **Merge Joins**.
- **Tradeoff:** Queries filtering primarily on `contact_id` (Contact List, Aggregate) cannot leverage this sort order for their joins and will likely default to Hash Joins or explicit Sort steps. This was a conscious decision to optimize the majority of the workload.

## Queries

There are 6 benchmark queries, each converted to a "Top-K" format (LIMIT 10/25) and wrapped in `PREPARE`/`EXECUTE` blocks to facilitate easy benchmarking with `EXPLAIN (ANALYZE, BUFFERS)`.

1.  **`query-contact-list.sql`**: Filters contacts by a specific `contact_list` ID. Ordering by `revenue_rank` (Top-K).
2.  **`query-company-list.sql`**: Filters contacts whose companies appear in a specific `company_list`. Ordering by `revenue_rank` (Top-K).
3.  **`query-intent.sql`**: Filters contacts at companies with specific intent signals (scored text search).
4.  **`query-tech-install.sql`**: Filters contacts at companies with specific technology installed (e.g., "salesforce").
5.  **`query-specialties.sql`**: Filters contacts at companies that _have_ specialties but _do not_ have "salesforce". This query uses an optimized `EXISTS` / `NOT EXISTS` structure to avoid inefficient nested `NOT IN` plans.
6.  **`query-aggregate.sql`**: A complex filter on job titles and contact lists, flattened from an aggregation query into a Top-K document retrieval query.

## Usage

To run a benchmark:

1.  Load the schema and data: `psql -f dsg/ddl.sql`
2.  Execute a query (which runs `EXPLAIN ANALYZE` followed by the actual execution): `psql -f dsg/query-intent.sql`
