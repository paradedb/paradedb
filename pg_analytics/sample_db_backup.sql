--
-- PostgreSQL database dump
--

-- Dumped from database version 16.1 (Homebrew)
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
-- Name: paradedb; Type: SCHEMA; Schema: -; Owner: mingying
--

CREATE SCHEMA paradedb;


ALTER SCHEMA paradedb OWNER TO mingying;

--
-- Name: pg_analytics; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_analytics WITH SCHEMA paradedb;


--
-- Name: EXTENSION pg_analytics; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pg_analytics IS 'Real-time analytics for PostgreSQL using columnar storage and vectorized execution';


--
-- Name: add(double precision, double precision); Type: FUNCTION; Schema: public; Owner: mingying
--

CREATE FUNCTION public.add(double precision, double precision) RETURNS double precision
    LANGUAGE sql IMMUTABLE STRICT
    AS $_$select $1 + $2;$_$;


ALTER FUNCTION public.add(double precision, double precision) OWNER TO mingying;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: s; Type: TABLE; Schema: public; Owner: mingying
--

CREATE TABLE public.s (
    a integer,
    b text
);


ALTER TABLE public.s OWNER TO mingying;

SET default_table_access_method = deltalake;

--
-- Name: t; Type: TABLE; Schema: public; Owner: mingying
--

CREATE TABLE public.t (
    a integer,
    b text
);


ALTER TABLE public.t OWNER TO mingying;

--
-- Name: u; Type: TABLE; Schema: public; Owner: mingying
--

CREATE TABLE public.u (
    a integer
);


ALTER TABLE public.u OWNER TO mingying;

--
-- Data for Name: s; Type: TABLE DATA; Schema: public; Owner: mingying
--

COPY public.s (a, b) FROM stdin;
1	test
2	another
\.


--
-- Data for Name: t; Type: TABLE DATA; Schema: public; Owner: mingying
--

COPY public.t (a, b) FROM stdin;
2	another
1	test
\.


--
-- Data for Name: u; Type: TABLE DATA; Schema: public; Owner: mingying
--

COPY public.u (a) FROM stdin;
2
1
\.


--
-- PostgreSQL database dump complete
--

