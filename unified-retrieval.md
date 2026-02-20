# The Era of the Unified Retrieval Engine: Why Betting on Postgres Wins

**By Ankit Mittal**

I recently sat down with Benjamin and Eldad on the _Data Engineering Show_ to discuss a migration that raised a lot of eyebrows: moving search at Instacart from a legacy search engine to Postgres. Since then, the feedback has ranged from "You're crazy" to "Finally, someone said it."

The industry has spent the last decade unbundling the database. We were told we needed a specialized tool for everything: vector databases for embeddings, graph databases for relationships, keyword engines for search, and Postgres just for the "boring" relational stuff.

But the game has changed. Retrieval isn't just about finding records for humans anymore. It's the backbone of modern AI and RAG systems. Trying to build this intelligent infrastructure on a fractured stack has created an impossible-to-debug nightmare of syncing multiple data systems.

**The era of the fragmented search stack is over.** This is the age of the **Unified Retrieval Engine**, and it lives entirely inside the database.

## What is a Unified Retrieval Engine?

The idea is simple: PostgreSQL's extensible architecture lets it serve as a single, ACID-compliant substrate for multimodal retrieval.

With extensions like **ParadeDB** for BM25 lexical scoring, **pgvector** for semantic search, and **Apache AGE** for graph traversal, you can collapse the entire retrieval stack into one system.

## The Secret Weapon: Rust in the Database

The common critique I hear is: _"Postgres can't handle this much math!"_

They're wrong. Pushing compute down to the data layer is a known, high-efficiency computing pattern. PostGIS was the OG extension that proved this, handling complex geo-spatial math for over a decade long before pgvector or ParadeDB existed.

Yes, PL/pgSQL is too slow for complex vector math. But the landscape has shifted with **pgrx**. We can now write high-performance Rust that compiles to native machine code and runs safely inside the Postgres process, executing complex retrieval algorithms right where the data lives. No more moving data to external engines.

## The Scale Elephant in the Room

The immediate counter-argument is always: _"But Postgres can't scale to billions of vectors like specialized engines."_

This overlooks reality. PostgreSQL has some of the most mature scaling mechanisms in the database world. We aren't reinventing the wheel. We're using partitioning, sharding, and read replicas.

- **Vertical scaling:** Hardware is cheaper than engineering time. You can go a long way just running Postgres on larger instances. Often the most pragmatic path.
- **Horizontal scaling:** For read-heavy retrieval workloads, adding read replicas is trivial and linearly increases throughput.
- **Data distribution:** For massive datasets, declarative partitioning and sharding let you distribute the index across nodes transparently.

Here's the thing: **you already have to scale your primary Postgres database.** As your company grows, your operational expertise in scaling Postgres grows with it. Scaling a Unified Retrieval Engine is far easier than trying to become an expert in scaling three different distributed systems (Postgres + Vector DB + Graph DB) simultaneously. Consolidating your scaling strategy into one proven technology is the path of least resistance.

## The Operational Maturity Dividend

By betting on Postgres, you aren't just getting search. You're inheriting 30 years of battle-tested operational maturity.

- Point-in-Time Recovery (PITR) and Write-Ahead Logging (WAL) come out of the box.
- You get robust Role-Based Access Control (RBAC) that security teams already trust, rather than fighting to secure a new, niche vector database.
- It works with every BI tool, ORM, and backup solution in existence.

## Latency & The Distributed Systems Tax

Critics often point to raw query speed, arguing that a dedicated engine might be 5ms faster per query. But that misses the point entirely. **End-to-end latency is what users feel.**

In a fragmented stack, you pay a "Distributed Systems Tax." You fetch IDs from a vector DB, send them to your app, fetch metadata from your primary DB, and then perform a join in your application layer.

Two things kill you:

1.  **Network jitter.** You're adding multiple network hops and serialization/deserialization costs.
2.  **Tail latency compounding.** Your request is held hostage by the slowest service. The P99 of your slowest component effectively becomes the P95 of your entire user request.

By moving the join to the data, inside the Unified Retrieval Engine, you eliminate these hops. The raw search might be theoretically slower, but the final response to the user is significantly faster because the data never leaves the database engine until the final result is ready.

## The Three Pillars of Unified Retrieval

Three retrieval methods coexist on the same data:

### 1. Semantic Retrieval

Semantic retrieval has become the workhorse of RAG workloads. **pgvector** handles this natively, but its true power unlocks when combined with standard SQL. Unlike specialized vector databases that struggle with "pre-filtering vs. post-filtering," Postgres allows you to combine vector search with complex metadata filters (dates, user IDs, categories) in a single, optimized query plan.

### 2. Lexical Retrieval

BM25 lexical retrieval, powered by extensions like **ParadeDB**, is back in vogue. Engineers are recognizing that while vectors provide high recall (intelligence), they often suffer from "vibes-based" inaccuracies. Lexical search offers high precision (exactness). A good mental model is that vectors provide the "intelligence," while keywords provide the "memory." You need both for a robust system.

### 3. Graph Retrieval

The third pillar is the graph (often called "GraphRAG"). In high-quality datasets, explicit relationships like citations, social connections, and product taxonomies provide the strongest signal of relevance.

PostgreSQL is effectively a relational graph engine. It can perform sophisticated traversals using native Recursive CTEs or extensions like Apache AGE. That's "multi-hop" reasoning without moving data to a separate graph store.

## The Power of In-Database Re-ranking

The killer feature of the Unified Retrieval Engine is the ability to combine these three signals (vector, lexical, and graph) and re-rank them instantly without moving data over the wire.

- **Reciprocal Rank Fusion (RRF)** to normalize scores from multiple retrieval methods into a single, high-quality result set.
- **Score boosting** with "hot + decay" algorithms, decaying scores over time while boosting based on voting patterns or popularity.
- **Personalization** using materialized views of historical click-through rates to boost results for specific user segments.

## Pre-Retrieval: Intelligent Query Transformation

Because the retrieval engine sits inside the database, you can do "zero-ETL" query transformations before the search even runs:

- **Query expansion:** A quick scan over corpus tokens can expand "Car" into "Auto OR Vehicle OR Sedan."
- **Knowledge graph grounding:** Use an LLM to generate a 2-hop thinking path, then verify those hops against the relational data in Postgres before executing the final retrieval.

## Conclusion

The "Best of Breed" era gave us fragmented, fragile architectures where data was constantly being copied between systems. A unified approach reduces data movement, guarantees ACID consistency, and enables retrieval workflows that were previously impossible due to sync latency.

The database stops being a passive storage container and becomes an active, intelligent retrieval system. You move faster, debug easier, and sleep better.

---

_Relevant Talks & Resources:_

- _Postgres vs. Elasticsearch: Instacart's Unexpected Winner in High-Stakes Search with Ankit Mittal_
- _Building Intelligent Applications with Graph-Based RAG on PostgreSQL | POSETTE 2025_
