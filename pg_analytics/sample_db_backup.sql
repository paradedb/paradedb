--
-- PostgreSQL database dump
--

-- Dumped from database version 16.1 (Debian 16.1-1.pgdg120+1)
-- Dumped by pg_dump version 16.1 (Homebrew)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: paradedb; Type: SCHEMA; Schema: -; Owner: myuser
--

CREATE SCHEMA paradedb;


ALTER SCHEMA paradedb OWNER TO myuser;

--
-- Name: search_idx; Type: SCHEMA; Schema: -; Owner: myuser
--

CREATE SCHEMA search_idx;


ALTER SCHEMA search_idx OWNER TO myuser;

--
-- Name: pg_analytics; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_analytics WITH SCHEMA paradedb;


--
-- Name: EXTENSION pg_analytics; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pg_analytics IS 'Real-time analytics for PostgreSQL using columnar storage and vectorized execution';


--
-- Name: pg_bm25; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_bm25 WITH SCHEMA paradedb;


--
-- Name: EXTENSION pg_bm25; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pg_bm25 IS 'pg_bm25: Full text search for PostgreSQL using BM25';


--
-- Name: svector; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS svector WITH SCHEMA public;


--
-- Name: EXTENSION svector; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION svector IS 'pg_sparse: Sparse vector data type and sparse HNSW access methods';


--
-- Name: vector; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS vector WITH SCHEMA public;


--
-- Name: EXTENSION vector; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION vector IS 'vector data type and ivfflat and hnsw access methods';


--
-- Name: highlight(text, integer, integer, text, integer, boolean, boolean, text, integer, text); Type: FUNCTION; Schema: search_idx; Owner: myuser
--

CREATE FUNCTION search_idx.highlight(query text, offset_rows integer DEFAULT NULL::integer, limit_rows integer DEFAULT NULL::integer, fuzzy_fields text DEFAULT NULL::text, distance integer DEFAULT NULL::integer, transpose_cost_one boolean DEFAULT NULL::boolean, prefix boolean DEFAULT NULL::boolean, regex_fields text DEFAULT NULL::text, max_num_chars integer DEFAULT NULL::integer, highlight_field text DEFAULT NULL::text) RETURNS TABLE(id bigint, highlight_bm25 text)
    LANGUAGE plpgsql
    AS $$
        DECLARE
            __paradedb_search_config__ JSONB;
        BEGIN
           -- Merge the outer 'index_json' object into the parameters passed to the dynamic function.
           __paradedb_search_config__ := jsonb_strip_nulls(
        		'{"key_field": "id", "index_name": "search_idx_bm25_index", "table_name": "mock_items", "schema_name": "public"}'::jsonb || jsonb_build_object(
            		'query', query,
                	'offset_rows', offset_rows,
                	'limit_rows', limit_rows,
                	'fuzzy_fields', fuzzy_fields,
                	'distance', distance,
                	'transpose_cost_one', transpose_cost_one,
                	'prefix', prefix,
                	'regex_fields', regex_fields,
                	'max_num_chars', max_num_chars,
                    'highlight_field', highlight_field
            	)
        	);
            RETURN QUERY SELECT * FROM paradedb.highlight_bm25(__paradedb_search_config__);
        END;
        $$;


ALTER FUNCTION search_idx.highlight(query text, offset_rows integer, limit_rows integer, fuzzy_fields text, distance integer, transpose_cost_one boolean, prefix boolean, regex_fields text, max_num_chars integer, highlight_field text) OWNER TO myuser;

--
-- Name: rank(text, integer, integer, text, integer, boolean, boolean, text, integer, text); Type: FUNCTION; Schema: search_idx; Owner: myuser
--

CREATE FUNCTION search_idx.rank(query text, offset_rows integer DEFAULT NULL::integer, limit_rows integer DEFAULT NULL::integer, fuzzy_fields text DEFAULT NULL::text, distance integer DEFAULT NULL::integer, transpose_cost_one boolean DEFAULT NULL::boolean, prefix boolean DEFAULT NULL::boolean, regex_fields text DEFAULT NULL::text, max_num_chars integer DEFAULT NULL::integer, highlight_field text DEFAULT NULL::text) RETURNS TABLE(id bigint, rank_bm25 real)
    LANGUAGE plpgsql
    AS $$
        DECLARE
            __paradedb_search_config__ JSONB;
        BEGIN
           -- Merge the outer 'index_json' object into the parameters passed to the dynamic function.
           __paradedb_search_config__ := jsonb_strip_nulls(
        		'{"key_field": "id", "index_name": "search_idx_bm25_index", "table_name": "mock_items", "schema_name": "public"}'::jsonb || jsonb_build_object(
            		'query', query,
                	'offset_rows', offset_rows,
                	'limit_rows', limit_rows,
                	'fuzzy_fields', fuzzy_fields,
                	'distance', distance,
                	'transpose_cost_one', transpose_cost_one,
                	'prefix', prefix,
                	'regex_fields', regex_fields,
                	'max_num_chars', max_num_chars,
                    'highlight_field', highlight_field
            	)
        	);
            RETURN QUERY SELECT * FROM paradedb.rank_bm25(__paradedb_search_config__);
        END;
        $$;


ALTER FUNCTION search_idx.rank(query text, offset_rows integer, limit_rows integer, fuzzy_fields text, distance integer, transpose_cost_one boolean, prefix boolean, regex_fields text, max_num_chars integer, highlight_field text) OWNER TO myuser;

--
-- Name: rank_hybrid(text, text, integer, integer, real, real); Type: FUNCTION; Schema: search_idx; Owner: myuser
--

CREATE FUNCTION search_idx.rank_hybrid(bm25_query text, similarity_query text, similarity_limit_n integer DEFAULT 100, bm25_limit_n integer DEFAULT 100, similarity_weight real DEFAULT 0.5, bm25_weight real DEFAULT 0.5) RETURNS TABLE(id bigint, rank_hybrid real)
    LANGUAGE plpgsql
    AS $_$
            DECLARE
                __paradedb_search_config__ JSONB;
                query text;
            BEGIN
            -- Merge the outer 'index_json' object into the parameters passed to the dynamic function.
                __paradedb_search_config__ := jsonb_strip_nulls(
                    '{"key_field": "id", "index_name": "search_idx_bm25_index", "table_name": "mock_items", "schema_name": "public"}'::jsonb || jsonb_build_object(
                        'query', bm25_query,
                        'limit_rows', bm25_limit_n
                    )
                );

                query := replace('
            WITH similarity AS (
                SELECT
                    __key_field__ as key_field,
                    1 - ((__similarity_query__) - MIN(__similarity_query__) OVER ()) / 
                    (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ()) AS score
                FROM mock_items
                ORDER BY __similarity_query__
                LIMIT $2
            ),
            bm25 AS (
                SELECT 
                    __key_field__ as key_field, 
                    rank_bm25 as score 
                FROM paradedb.minmax_bm25($1)
            )
            SELECT
                COALESCE(similarity.key_field, bm25.key_field) AS __key_field__,
                (COALESCE(similarity.score, 0.0) * $3 + COALESCE(bm25.score, 0.0) * $4)::real AS score_hybrid
            FROM similarity
            FULL OUTER JOIN bm25 ON similarity.key_field = bm25.key_field
            ORDER BY score_hybrid DESC;
        ', '__similarity_query__', similarity_query);
                query := replace(query, '__key_field__', __paradedb_search_config__ ->>'key_field');

                RETURN QUERY EXECUTE query
                USING __paradedb_search_config__, similarity_limit_n, similarity_weight, bm25_weight;
            END;
            $_$;


ALTER FUNCTION search_idx.rank_hybrid(bm25_query text, similarity_query text, similarity_limit_n integer, bm25_limit_n integer, similarity_weight real, bm25_weight real) OWNER TO myuser;

--
-- Name: schema(); Type: FUNCTION; Schema: search_idx; Owner: myuser
--

CREATE FUNCTION search_idx.schema() RETURNS TABLE(name text, field_type text, stored boolean, indexed boolean, fast boolean, fieldnorms boolean, expand_dots boolean, tokenizer text, record text, normalizer text)
    LANGUAGE plpgsql
    AS $$
        BEGIN
            RETURN QUERY SELECT * FROM paradedb.schema_bm25('search_idx');
        END;
        $$;


ALTER FUNCTION search_idx.schema() OWNER TO myuser;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: mock_items; Type: TABLE; Schema: public; Owner: myuser
--

CREATE TABLE public.mock_items (
    id integer NOT NULL,
    description text,
    rating integer,
    category character varying(255),
    in_stock boolean,
    metadata jsonb,
    CONSTRAINT mock_items_rating_check CHECK (((rating >= 1) AND (rating <= 5)))
);


ALTER TABLE public.mock_items OWNER TO myuser;

--
-- Name: search(text, integer, integer, text, integer, boolean, boolean, text, integer, text); Type: FUNCTION; Schema: search_idx; Owner: myuser
--

CREATE FUNCTION search_idx.search(query text, offset_rows integer DEFAULT NULL::integer, limit_rows integer DEFAULT NULL::integer, fuzzy_fields text DEFAULT NULL::text, distance integer DEFAULT NULL::integer, transpose_cost_one boolean DEFAULT NULL::boolean, prefix boolean DEFAULT NULL::boolean, regex_fields text DEFAULT NULL::text, max_num_chars integer DEFAULT NULL::integer, highlight_field text DEFAULT NULL::text) RETURNS SETOF public.mock_items
    LANGUAGE plpgsql
    AS $$
        DECLARE
            __paradedb_search_config__ JSONB;
        BEGIN
           -- Merge the outer 'index_json' object into the parameters passed to the dynamic function.
           __paradedb_search_config__ := jsonb_strip_nulls(
        		'{"key_field": "id", "index_name": "search_idx_bm25_index", "table_name": "mock_items", "schema_name": "public"}'::jsonb || jsonb_build_object(
            		'query', query,
                	'offset_rows', offset_rows,
                	'limit_rows', limit_rows,
                	'fuzzy_fields', fuzzy_fields,
                	'distance', distance,
                	'transpose_cost_one', transpose_cost_one,
                	'prefix', prefix,
                	'regex_fields', regex_fields,
                	'max_num_chars', max_num_chars,
                    'highlight_field', highlight_field
            	)
        	);
            RETURN QUERY SELECT * FROM public.mock_items WHERE mock_items @@@ __paradedb_search_config__;
        END;
        $$;


ALTER FUNCTION search_idx.search(query text, offset_rows integer, limit_rows integer, fuzzy_fields text, distance integer, transpose_cost_one boolean, prefix boolean, regex_fields text, max_num_chars integer, highlight_field text) OWNER TO myuser;

--
-- Name: mock_items_id_seq; Type: SEQUENCE; Schema: public; Owner: myuser
--

CREATE SEQUENCE public.mock_items_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.mock_items_id_seq OWNER TO myuser;

--
-- Name: mock_items_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: myuser
--

ALTER SEQUENCE public.mock_items_id_seq OWNED BY public.mock_items.id;


SET default_table_access_method = deltalake;

--
-- Name: test_table; Type: TABLE; Schema: public; Owner: myuser
--

CREATE TABLE public.test_table (
    id integer NOT NULL,
    data text
);


ALTER TABLE public.test_table OWNER TO myuser;

--
-- Name: test_table_id_seq; Type: SEQUENCE; Schema: public; Owner: myuser
--

CREATE SEQUENCE public.test_table_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.test_table_id_seq OWNER TO myuser;

--
-- Name: test_table_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: myuser
--

ALTER SEQUENCE public.test_table_id_seq OWNED BY public.test_table.id;


--
-- Name: mock_items id; Type: DEFAULT; Schema: public; Owner: myuser
--

ALTER TABLE ONLY public.mock_items ALTER COLUMN id SET DEFAULT nextval('public.mock_items_id_seq'::regclass);


--
-- Data for Name: mock_items; Type: TABLE DATA; Schema: public; Owner: myuser
--

COPY public.mock_items (id, description, rating, category, in_stock, metadata) FROM stdin;
1	Ergonomic metal keyboard	4	Electronics	t	{"color": "Silver", "location": "United States"}
2	Plastic Keyboard	4	Electronics	f	{"color": "Black", "location": "Canada"}
3	Sleek running shoes	5	Footwear	t	{"color": "Blue", "location": "China"}
4	White jogging shoes	3	Footwear	f	{"color": "White", "location": "United States"}
5	Generic shoes	4	Footwear	t	{"color": "Brown", "location": "Canada"}
6	Compact digital camera	5	Photography	f	{"color": "Black", "location": "China"}
7	Hardcover book on history	2	Books	t	{"color": "Brown", "location": "United States"}
8	Organic green tea	3	Groceries	t	{"color": "Green", "location": "Canada"}
9	Modern wall clock	4	Home Decor	f	{"color": "Silver", "location": "China"}
10	Colorful kids toy	1	Toys	t	{"color": "Multicolor", "location": "United States"}
11	Soft cotton shirt	5	Apparel	t	{"color": "Blue", "location": "Canada"}
12	Innovative wireless earbuds	5	Electronics	t	{"color": "Black", "location": "China"}
13	Sturdy hiking boots	4	Footwear	t	{"color": "Brown", "location": "United States"}
14	Elegant glass table	3	Furniture	t	{"color": "Clear", "location": "Canada"}
15	Refreshing face wash	2	Beauty	f	{"color": "White", "location": "China"}
16	High-resolution DSLR	4	Photography	t	{"color": "Black", "location": "United States"}
17	Paperback romantic novel	3	Books	t	{"color": "Multicolor", "location": "Canada"}
18	Freshly ground coffee beans	5	Groceries	t	{"color": "Brown", "location": "China"}
19	Artistic ceramic vase	4	Home Decor	f	{"color": "Multicolor", "location": "United States"}
20	Interactive board game	3	Toys	t	{"color": "Multicolor", "location": "Canada"}
21	Slim-fit denim jeans	5	Apparel	f	{"color": "Blue", "location": "China"}
22	Fast charging power bank	4	Electronics	t	{"color": "Black", "location": "United States"}
23	Comfortable slippers	3	Footwear	t	{"color": "Brown", "location": "Canada"}
24	Classic leather sofa	5	Furniture	f	{"color": "Brown", "location": "China"}
25	Anti-aging serum	4	Beauty	t	{"color": "White", "location": "United States"}
26	Portable tripod stand	4	Photography	t	{"color": "Black", "location": "Canada"}
27	Mystery detective novel	2	Books	f	{"color": "Multicolor", "location": "China"}
28	Organic breakfast cereal	5	Groceries	t	{"color": "Brown", "location": "United States"}
29	Designer wall paintings	5	Home Decor	t	{"color": "Multicolor", "location": "Canada"}
30	Robot building kit	4	Toys	t	{"color": "Multicolor", "location": "China"}
31	Sporty tank top	4	Apparel	t	{"color": "Blue", "location": "United States"}
32	Bluetooth-enabled speaker	3	Electronics	t	{"color": "Black", "location": "Canada"}
33	Winter woolen socks	5	Footwear	f	{"color": "Gray", "location": "China"}
34	Rustic bookshelf	4	Furniture	t	{"color": "Brown", "location": "United States"}
35	Moisturizing lip balm	4	Beauty	t	{"color": "Pink", "location": "Canada"}
36	Lightweight camera bag	5	Photography	f	{"color": "Black", "location": "China"}
37	Historical fiction book	3	Books	t	{"color": "Multicolor", "location": "United States"}
38	Pure honey jar	4	Groceries	t	{"color": "Yellow", "location": "Canada"}
39	Handcrafted wooden frame	5	Home Decor	f	{"color": "Brown", "location": "China"}
40	Plush teddy bear	4	Toys	t	{"color": "Brown", "location": "United States"}
41	Warm woolen sweater	3	Apparel	f	{"color": "Red", "location": "Canada"}
\.


--
-- Data for Name: test_table; Type: TABLE DATA; Schema: public; Owner: myuser
--

COPY public.test_table (id, data) FROM stdin;
\.


--
-- Name: mock_items_id_seq; Type: SEQUENCE SET; Schema: public; Owner: myuser
--

SELECT pg_catalog.setval('public.mock_items_id_seq', 41, true);


--
-- Name: test_table_id_seq; Type: SEQUENCE SET; Schema: public; Owner: myuser
--

SELECT pg_catalog.setval('public.test_table_id_seq', 2, true);


--
-- Name: mock_items mock_items_pkey; Type: CONSTRAINT; Schema: public; Owner: myuser
--

ALTER TABLE ONLY public.mock_items
    ADD CONSTRAINT mock_items_pkey PRIMARY KEY (id);


--
-- PostgreSQL database dump complete
--

