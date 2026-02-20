# The Era of the Unified Retrieval Engine: Why Betting on Postgres Wins

**By Ankit Mittal**

I recently sat down with Benjamin and Eldad on the _Data Engineering Show_ to discuss a migration that raised a lot of eyebrows: moving search at Instacart from Elasticsearch to Postgres. Since then, the feedback has ranged from "You're crazy" to "Finally, someone said it."

The industry has spent the last decade unbundling the database. We were told we needed a specialized tool for everything—vector databases for embeddings, graph databases for relationships, keyword engines for search, and Postgres just for the "boring" relational stuff.

But here's what that "best of breed" stack actually gave my team at Instacart: a 3am page because our vector index was 10 minutes behind our primary database, causing phantom products to show up in search results. A two-week debugging saga tracking down why user preferences weren't applying to recommendations—turned out to be a sync failure between three different data stores. When retrieval is the backbone of your AI systems, a fractured stack becomes an impossible-to-debug nightmare.

**The era of the fragmented search stack is over.** PostgreSQL's extensible architecture makes it a single, ACID (Atomicity, Consistency, Isolation, Durability)-compliant substrate for multimodal retrieval. With extensions like ParadeDB for BM25 (Best Matching 25) lexical scoring, pgvector for semantic search, and Apache AGE for graph traversal, you can collapse the entire retrieval stack into one system. This is the age of the **Unified Retrieval Engine**, and it lives entirely inside the database.

## "Postgres Can't Handle This Much Math"

A friend at a major SaaS company told me over coffee: "Postgres can't handle this much math." I hear this constantly at conferences. It's the wrong mental model.

Pushing compute down to the data layer is a proven, high-efficiency pattern. PostGIS proved this over a decade ago, handling complex geo-spatial math long before pgvector or ParadeDB existed.

Yes, PL/pgSQL (Procedural Language/PostgreSQL) is too slow for complex vector operations. But the landscape shifted with **pgrx**. My team could now write high-performance Rust that compiles to native machine code and runs safely inside the Postgres process, executing complex retrieval algorithms right where the data lives. No more serializing data to external engines.

Here's a real query from our production system—the kind that would require orchestrating three separate systems:

```sql
WITH semantic_results AS (
  SELECT id, embedding <=> query_vector AS semantic_score
  FROM products
  WHERE category_id = $1 AND in_stock = true
  ORDER BY embedding <=> query_vector
  LIMIT 100
),
lexical_results AS (
  SELECT id, paradedb.rank_bm25(id) AS bm25_score
  FROM products
  WHERE product_text @@@ paradedb.term($2)
    AND category_id = $1 AND in_stock = true
  LIMIT 100
)
SELECT p.*,
  1.0 / (60 + semantic_results.rank) +
  1.0 / (60 + lexical_results.rank) AS rrf_score
FROM products p
JOIN semantic_results ON p.id = semantic_results.id
JOIN lexical_results ON p.id = lexical_results.id
ORDER BY rrf_score DESC
LIMIT 20;
```

That's Reciprocal Rank Fusion (RRF) combining semantic and lexical search, filtering by live inventory, all in one atomic query. No microservice orchestration. No data serialization. No sync delays.

## The Scale Elephant in the Room

The immediate counter-argument: _"But Postgres can't scale to billions of vectors like specialized engines."_

This overlooks reality. **You already have to scale your primary Postgres database.** As your company grows, your operational expertise scales with it. The question isn't "Can Postgres scale?"—it's "Do you want to become an expert in scaling three different distributed systems simultaneously?"

At Instacart, my colleagues and I scaled Postgres using the same playbook every mature company uses:

- **Vertical scaling:** Hardware is cheaper than engineering time. Running on larger instances took us far before hitting limits.
- **Read replicas:** For read-heavy retrieval workloads, adding replicas linearly increases throughput.
- **Declarative partitioning:** For massive datasets, we distributed indexes across nodes transparently.

Consolidating your scaling strategy into one proven technology is the path of least resistance. We already had runbooks for Postgres failovers, backup procedures that security approved, and monitoring that actually worked.

## The Distributed Systems Tax

Critics point to raw query speed—"a dedicated engine might be 5ms faster per query." That misses the point entirely. **End-to-end latency is what users feel.**

In our old fragmented stack, every search request paid the Distributed Systems Tax:

1. Fetch IDs from Pinecone (semantic search)
2. Serialize and send over the network
3. Fetch metadata from Postgres
4. Join in application memory
5. Fetch additional signals from a third service

Two things killed our tail latency:

- **Network jitter:** Each hop added unpredictable milliseconds of serialization and TCP (Transmission Control Protocol) overhead
- **Tail latency compounding:** Your P99 becomes the sum of every service's P95. The slowest component held every request hostage.

After we moved to unified retrieval in Postgres, our P95 latency dropped from 180ms to 45ms. Not because the search was faster—because the data never left the database until the final result was ready.

## The Three Pillars in Practice

At Instacart, my team combined three retrieval methods on the same data:

**Semantic retrieval** became the workhorse of our recommendation system. pgvector handles this natively, and its real power emerges when combining vector search with complex filters—user dietary restrictions, delivery time windows, past purchase history—in a single optimized query plan using standard SQL (Structured Query Language).

**Lexical retrieval** came back into fashion when our data scientists recognized that vector search, while excellent for semantic understanding and high recall, can be fuzzy on exact matches. BM25 via ParadeDB provides the keyword precision that pure vector search lacks—capturing literal terms rather than just meaning. You need both: semantic depth and lexical precision.

**Graph traversal** unlocked our "customers also bought" feature without a separate graph database. Using recursive CTEs (Common Table Expressions), we could do multi-hop reasoning: "Find products purchased by customers who bought this item, then filter by seasonal trends." That's "GraphRAG" (Graph-based Retrieval Augmented Generation) without moving data to Neo4j.

The killer feature? **In-database re-ranking.** We could combine all three signals—semantic, lexical, graph—and apply custom scoring algorithms (time decay, popularity boost, personalization from click-through rates stored in materialized views) without ever moving data over the wire.

## What I'd Tell Someone Starting Today

If you're building a new project in 2026, here's the one thing you need to know: the database isn't a passive storage container anymore.

Every startup pitch deck from 2019 sold the same dream—unbundle everything, let each system do one thing well. That era gave us fragmented, fragile architectures where data was constantly being copied between systems.

The game changed when retrieval became the backbone of AI systems. RAG (Retrieval Augmented Generation) workflows demand ACID consistency, sub-100ms latency, and the ability to combine multiple search modalities in a single request. A unified approach reduces data movement, eliminates sync latency, and enables retrieval workflows that were previously impossible.

Most teams already know how to scale Postgres. They already have backup procedures. Security teams already trust Postgres's RBAC (Role-Based Access Control). Why add three more distributed systems to that stack when extensions can do the job?

At Instacart, consolidating our retrieval stack into Postgres meant fewer 3am pages for the on-call team, faster debugging, and search results that actually stayed in sync with reality. That's the dividend of unified retrieval.
