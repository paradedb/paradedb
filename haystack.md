# Haystack Integration for ParadeDB: Strategic Overview

## What is Haystack?

Haystack is an **open-source AI orchestration framework** for building production-ready:

- **Retrieval-Augmented Generation (RAG) systems**
- **AI Agents** with function calling and tool use
- **Multimodal search systems** (text, image, audio)
- **Conversational AI** applications

**Key difference from LangChain**: Haystack uses explicit **DAG (Directed Acyclic Graph) pipelines** where component connections are declared upfront. This makes it better for production systems that need debugging, serialization, and cloud deployment.

---

## Haystack Architecture Primer

Haystack has four core concepts:

### 1. Components

Building blocks (Embedders, Retrievers, Generators, Rankers, Writers) that:

- Have explicit `run()` methods with typed inputs/outputs
- Can be used standalone or in pipelines
- Support lazy-loading via `warm_up()` for heavy resources (models, LLMs)

### 2. Pipelines

DAGs connecting components where:

- Each connection is explicit (output → input)
- Serializable (YAML/JSON)
- Cloud-agnostic, Kubernetes-ready
- Run with `pipeline.run()`

### 3. Document Stores

Database abstraction layer (NOT a component) that:

- Stores documents and metadata
- Provides standard interface: `write_documents()`, `filter_documents()`, `count_documents()`, `delete_documents()`
- Each has a corresponding Retriever component

### 4. Retrievers

Components that:

- Fetch documents from Document Stores
- Implement the `BaseRetriever` interface
- Have explicit outputs (list of Documents)
- Often database-specific (e.g., `ElasticsearchEmbeddingRetriever`)

**Simple RAG Pipeline in Haystack**:

```
User Query
    ↓
TextEmbedder (query → embedding)
    ↓
Retriever (embedding → documents from DocumentStore)
    ↓
PromptBuilder (question + documents → prompt)
    ↓
Generator/LLM (prompt → answer)
    ↓
Answer
```

---

## ParadeDB's Position in Haystack

### Current State

**Haystack supports 13+ databases**:

- **Full-text search**: Elasticsearch, OpenSearch
- **Pure vectors**: Chroma, Pinecone, Qdrant, Weaviate, Marqo, Milvus
- **SQL + vectors**: **pgvector only** ← This is where ParadeDB comes in
- **NoSQL + vectors**: MongoDB, Astra, Neo4j
- **In-memory**: InMemoryDocumentStore

**The gap**: pgvector is the ONLY SQL database option, and it's **vector-first, keyword-second**.

### ParadeDB's Unique Angle

ParadeDB is **search-first, SQL-second**:

- **Native BM25**: Production-grade full-text search (like Elasticsearch)
- **Transactional**: ACID updates, zero indexing lag (unlike Elasticsearch)
- **SQL-native**: Full Postgres in one system (unlike separate Elasticsearch cluster)
- **Real-time**: Changes visible immediately to readers

### Why Not Just Use Elasticsearch?

| Dimension                | ParadeDB                         | Elasticsearch           |
| ------------------------ | -------------------------------- | ----------------------- |
| **Setup**                | Docker 1 min                     | Cluster setup 5-10 min  |
| **Operational overhead** | Single database                  | Separate system + ETL   |
| **Data consistency**     | Transactional (write-after-read) | Eventual (1-2s lag)     |
| **Real-time indexing**   | Sub-millisecond                  | 1-2 second delay        |
| **SQL access**           | Full Postgres SQL                | Query DSL only          |
| **Multi-tenancy**        | Row-level security               | Complex to implement    |
| **Hybrid search**        | BM25 + pgvector native           | BM25 + vectors + RRF    |
| **Cost**                 | Postgres price                   | Elasticsearch licensing |

**Winner**: ParadeDB for teams who **already use Postgres**, need **transactional consistency**, or want **operational simplicity**.

---

## Three Types of ParadeDB Retrievers for Haystack

### 1. SimpleParadeDBRetriever

**What it does**: BM25 keyword search only

**Haystack interface**:

```
Input: query (string)
Output: documents (List[Document])
```

**When to use**:

- Exact terminology matters (legal docs, technical specs, RFCs)
- Metadata filtering needed
- Simplicity preferred over semantic matching
- Getting started with ParadeDB

**Tradeoff**: Misses semantic relationships (e.g., "machine learning frameworks" ≠ "neural network libraries" for BM25)

**Implementation**:

- SQL: `SELECT * FROM documents WHERE content @@ to_tsquery(query) ORDER BY ts_rank(...) LIMIT k`
- Maps Haystack's filter syntax to Postgres WHERE clauses

---

### 2. HybridParadeDBRetriever

**What it does**: Combines BM25 + vector search

**How it works** (three stages):

1. **Stage 1**: BM25 search on ParadeDB (fast, cheap, broad matches)
2. **Stage 2**: Vector search on pgvector (slow, expensive, precise semantic matches)
3. **Stage 3**: Merge using RRF (Reciprocal Rank Fusion) algorithm

**Why hybrid?**

- Query: "machine learning frameworks"
- BM25 finds: exact term matches (frameworks, algorithms, libraries)
- Vector search finds: semantic similarity (neural networks, deep learning tools)
- **Together**: Better relevance than either alone

**When to use**: Production RAG where retrieval quality matters

**Tradeoff**: Requires pgvector extension + embedding model (more complexity, better results)

**Implementation**:

- BM25 stage: `SELECT * FROM documents WHERE content @@ to_tsquery(query) ... LIMIT top_k`
- Vector stage: `SELECT * FROM documents ORDER BY embedding <-> query_embedding LIMIT top_k`
- RRF merge: Convert both scores to ranks (1st place = rank 1), combine with formula: `RRF_score = 1/(k + rank)`

---

### 3. MetadataAwareParadeDBRetriever

**What it does**: BM25 search + SQL metadata filtering in one query

**Example**:

```
WHERE bm25_match(content, 'kubernetes')
  AND created_date > '2024-01-01'
  AND author_team = 'platform'
  AND status = 'published'
```

**When to use**:

- Multi-tenant systems
- Time-sensitive data (legal, financial)
- Complex access control (different users see different docs)
- Document categories or tags matter

**Haystack advantage**: Can use Haystack's `Filter` objects for SQL generation

**Implementation**:

- Translate Haystack filter syntax to Postgres WHERE clauses
- Support date ranges, equals, in, comparisons
- Combine with BM25 in single query (no post-filtering needed)

---

## ParadeDB's Position: Full-Text Search Database Category

Haystack recognizes **full-text search databases** as a distinct category (alongside pure vectors, SQL+vectors, etc.):

### Full-Text Search Databases in Haystack

**Elasticsearch** (existing integration):

- Mature, battle-tested
- Large ecosystem
- Established LangChain support
- Requires separate cluster

**OpenSearch** (existing integration):

- Fork of Elasticsearch
- Same capabilities

**ParadeDB** (proposed integration):

- **Transactional** alternative to Elasticsearch
- **SQL-native** (single Postgres instance)
- **Newer** (smaller community)
- **Better for**: Teams already on Postgres, need ACID, want operational simplicity

### Haystack's Categorization

```
Vector Databases          SQL + Vector              Full-Text Search
├─ Pinecone             ├─ pgvector        ← Only option    ├─ Elasticsearch
├─ Weaviate             │                                    ├─ OpenSearch
├─ Qdrant               │                                    └─ ParadeDB
├─ Chroma               │                                          ↑ New
└─ Others               └─ (ParadeDB alternative)            Opportunity
```

ParadeDB bridges the gap: **Transactional full-text search with SQL, not just vectors.**

---

## Haystack's DocumentStore Interface

Any database integrating with Haystack must implement **four methods**:

```
DocumentStore Protocol:
├─ count_documents() → int
├─ write_documents(documents: List[Document], policy: DuplicatePolicy) → int
├─ filter_documents(filters: Dict, top_k: int) → List[Document]
└─ delete_documents(document_ids: List[str]) → None
```

**Why this matters for ParadeDB**:

- `count_documents()`: `SELECT COUNT(*) FROM documents`
- `write_documents()`: `INSERT INTO documents (...) ON CONFLICT (...) DO UPDATE/SKIP`
- `filter_documents()`: `SELECT * FROM documents WHERE content @@ to_tsquery(...) AND ... LIMIT k`
- `delete_documents()`: `DELETE FROM documents WHERE id IN (...)`

All four map cleanly to SQL.

---

## How ParadeDB Fits in Haystack Ecosystem

### Integration Architecture

```
ParadeDB Integration
├─ Document Store Layer
│  └─ ParadeDBDocumentStore (implements DocumentStore protocol)
│     ├─ Postgres connection
│     ├─ Document table (id, content, metadata, embeddings)
│     ├─ BM25 index (via ParadeDB)
│     └─ Vector index (via pgvector)
│
└─ Retriever Layer (three implementations)
   ├─ SimpleParadeDBRetriever (BM25 only)
   ├─ HybridParadeDBRetriever (BM25 + pgvector)
   └─ MetadataAwareParadeDBRetriever (BM25 + SQL filters)
```

### How It Connects to Haystack Pipelines

```
Haystack Pipeline
├─ Input: User question
├─ TextEmbedder: question → embedding (for hybrid search)
├─ Retriever (choice of three)
│  └─ ParadeDBDocumentStore (fetches docs)
├─ PromptBuilder: docs + question → prompt
├─ Generator: prompt → answer
└─ Output: Answer with sources
```

**Key point**: ParadeDBDocumentStore is accessed BY retrievers, not a component itself.

---

## Testing Strategy for Haystack Integration

### Layer 1: Unit Tests (No Database)

**Purpose**: Verify retriever logic is correct

**How**: Mock ParadeDBDocumentStore

- Verify `run()` method accepts query
- Verify outputs are Document objects
- Verify metadata is preserved
- Test edge cases (empty results, special characters)
- Test component inputs/outputs match Haystack protocol

**Speed**: Fast (seconds)

---

### Layer 2: Integration Tests (Real ParadeDB)

**Purpose**: Verify integration with actual database

**How**:

- Spin up ParadeDB in Docker
- Load test documents (Kaggle helpdesk, blog posts)
- Run real queries
- Verify retrieval quality

**Questions**:

- Does BM25 find relevant documents?
- Are scores reasonable?
- Does metadata filtering work?
- Do results appear in expected order?

**Speed**: Medium (minutes)

---

### Layer 3: Performance Tests (Real ParadeDB)

**Purpose**: Verify production readiness

**Targets**:

- Single query latency: **<100ms**
- Throughput: **>10 queries/sec**
- Consistency: Same query always returns same results

**How**:

- Load 10,000+ documents
- Run 100+ varied queries
- Measure tail latencies (p95, p99)
- Compare BM25 vs vector vs hybrid
- Benchmark vs Elasticsearch

**Speed**: Slow (hours)

---

## Datasets for Testing

### Option 1: Kaggle IT Helpdesk (Easiest)

- 10 real IT support documents
- Q&A pairs for ground truth
- CSV format, ready to load
- Real-world relevance judgments
- **Use for**: Initial integration tests

### Option 2: Blog Posts (Realistic)

- Fetch from URLs (e.g., Lilian Weng's AI blog)
- Split into chunks (like real RAG)
- Moderate size (100-500 documents)
- Varied topics and writing styles
- **Use for**: Quality testing

### Option 3: Synthetic Data (Fastest)

- Generate in code
- No external dependencies
- Perfect for unit tests
- **Use for**: Fast iteration, edge cases

### Option 4: Wikipedia/Docs (Large-scale)

- 10,000+ documents
- Diverse topics
- **Use for**: Performance benchmarks

---

## Comparison: ParadeDB vs Alternatives in Haystack

### ParadeDB vs Elasticsearch (for Haystack)

| Dimension                  | ParadeDB           | Elasticsearch    |
| -------------------------- | ------------------ | ---------------- |
| **Setup time**             | 1 minute           | 5-10 minutes     |
| **Operational complexity** | Single system      | Separate cluster |
| **Real-time indexing**     | Sub-millisecond    | 1-2 second lag   |
| **ACID transactions**      | Yes                | No (eventual)    |
| **Full SQL support**       | Yes                | DSL only         |
| **Multi-tenancy**          | Row-level security | Complex          |
| **Native BM25**            | Yes                | Yes              |
| **Native vectors**         | Via pgvector       | Built-in         |
| **Hybrid search quality**  | Excellent          | Excellent        |
| **Maturity**               | Growing            | Established      |
| **Community size**         | Small              | Large            |

**Winner by use case**:

- **ParadeDB**: Teams on Postgres, need ACID, want simplicity
- **Elasticsearch**: Need scale, have ops resources, want mature ecosystem

---

### ParadeDB vs pgvector (for Haystack)

| Dimension                | ParadeDB       | pgvector               |
| ------------------------ | -------------- | ---------------------- |
| **Full-text search**     | Native BM25    | Not native (raw ILIKE) |
| **Search relevance**     | TF-IDF scoring | No scoring             |
| **Vector search**        | Via pgvector   | Native                 |
| **Transactional**        | Yes            | Yes                    |
| **SQL**                  | Full           | Full                   |
| **Setup**                | Optimized      | Bare Postgres          |
| **Operational overhead** | Low            | Low                    |
| **Best for text**        | Search-first   | Vector-first           |

**Use together**:

- **ParadeDB + pgvector** = BM25 + vector search in one database
- **HybridParadeDBRetriever** = Leverage both

---

## Implementation Roadmap for Haystack Integration

### Week 1: SimpleParadeDBRetriever

**Deliverables**:

- `ParadeDBDocumentStore` class (implements DocumentStore protocol)
- `SimpleParadeDBRetriever` class (BM25-only)
- Unit tests (mocked database)
- Integration tests (real ParadeDB)
- README and basic documentation

**Effort**: 1 week

---

### Week 2: HybridParadeDBRetriever

**Deliverables**:

- `HybridParadeDBRetriever` class (BM25 + pgvector)
- RRF ranking algorithm
- Async support (`run()` and `run_async()`)
- Performance tests
- Benchmark vs Elasticsearch

**Effort**: 1 week

---

### Week 3: MetadataAwareRetriever + Polish

**Deliverables**:

- `MetadataAwareParadeDBRetriever` class
- Filter syntax translation (Haystack filters → SQL WHERE)
- Advanced integration tests
- Documentation with examples

**Effort**: 1 week

---

### Week 4: Publishing + Ecosystem

**Deliverables**:

- Package for PyPI (`haystack-paradedb` or `paradedb-haystack`)
- Submit PR to `haystack-core-integrations`
- Integration page on Haystack docs
- Tutorial and cookbook entries
- Co-marketing with deepset

**Effort**: 1 week

**Total**: 4 weeks, similar to LangChain approach

---

## Key Concepts to Understand

### BM25 Scoring

- Industry-standard relevance algorithm for full-text search
- Considers term frequency and document frequency
- Works well for keyword matching
- Similar to how Elasticsearch scores results
- ParadeDB uses the same BM25 as Elasticsearch (same math)

### Vector Embeddings

- Text → high-dimensional vectors
- Similar meanings = similar vectors
- Enable semantic search
- Necessary for hybrid search in Haystack

### RRF (Reciprocal Rank Fusion)

- Problem: BM25 scores (e.g., 15.3) vs vector similarity (e.g., 0.87) incomparable
- Solution: Convert both to ranks (1st = rank 1), then merge
- Formula: `RRF_score = 1 / (k + rank)` where k=60 (Haystack standard)
- Both Elasticsearch and Haystack use this for hybrid search

### Haystack's Document Protocol

- `Document` objects have `content` (text), `metadata` (dict), `id`, `embedding`
- Metadata = filters, sources, creation dates, etc.
- Compatible with any retriever implementation

### DocumentStore vs Component

- **DocumentStore**: Database interface (not in pipeline)
- **Retriever**: Component in pipeline that reads from DocumentStore
- Distinction matters: DocumentStore handles persistence, Retriever handles retrieval logic

---

## Why This Matters

Most RAG systems in Haystack treat retrieval as **"find semantically similar documents"** (pure vectors).

**ParadeDB opens a different model: structured, transactional, text-first retrieval.**

For teams who:

- Use Postgres already (eliminates new infrastructure)
- Need real-time consistency (financial, legal data)
- Value operational simplicity (one database, not two)
- Want SQL control (metadata, filtering, analytics)
- Prefer keyword search for technical/structured content

**ParadeDB is the natural choice for Haystack.**

---

## How This Differs from LangChain Integration

| Aspect                | LangChain              | Haystack                  |
| --------------------- | ---------------------- | ------------------------- |
| **Integration point** | Custom `BaseRetriever` | DocumentStore + Retriever |
| **Architecture**      | Implicit chaining      | Explicit DAG              |
| **Serialization**     | Python code            | YAML/JSON pipelines       |
| **Community size**    | 100k+                  | 10k+                      |
| **Best for**          | Quick prototypes       | Production systems        |
| **Complexity**        | Lower entry, flexible  | Higher entry, explicit    |

**ParadeDB benefits**:

- **In LangChain**: Focused on RAG, reaches large community
- **In Haystack**: Fills database gap, appeals to Postgres users who need production RAG

Both integrations valuable, **different audiences**.

---

## Next Steps

1. **Understand Haystack architecture** - Read this document
2. **Understand ParadeDB capabilities** - Full-text search + SQL
3. **Understand Haystack's DocumentStore interface** - Four methods to implement
4. **Study Elasticsearch integration** - Reference implementation in Haystack
5. **Design ParadeDBDocumentStore** - Map database methods to protocol
6. **Implement SimpleParadeDBRetriever** - BM25-only, minimal viable product
7. **Build HybridParadeDBRetriever** - BM25 + pgvector combination
8. **Test thoroughly** - Unit, integration, performance
9. **Package and publish** - PyPI, submit to haystack-core-integrations
10. **Co-market with deepset** - Reach Haystack community

---

## Strategic Positioning

**ParadeDB in Haystack** is positioned as:

> **The transactional, Postgres-native full-text search database for production RAG systems.**

**Not trying to beat**: Elasticsearch at scale or pure vector search. (Elasticsearch for scale, pgvector for vectors.)

**Trying to own**: Teams who want **ACID + BM25 + SQL in one system** for **operational simplicity**.

---

## Architecture Diagram

```
Haystack Pipeline with ParadeDB
┌─────────────────────────────────────────┐
│  User Query: "How do I set up email?"   │
└──────────────┬──────────────────────────┘
               │
               ▼
        ┌──────────────────┐
        │  TextEmbedder    │ (query → embedding, for hybrid)
        └────────┬─────────┘
                 │
                 ▼
    ┌────────────────────────────┐
    │  SimpleParadeDBRetriever   │ (BM25 only) OR
    │  HybridParadeDBRetriever   │ (BM25 + pgvector) OR
    │  MetadataAwareRetriever    │ (BM25 + SQL filters)
    └────────────┬───────────────┘
                 │
                 ▼
    ┌────────────────────────────────┐
    │  ParadeDBDocumentStore         │
    │  (Postgres + ParadeDB indexes) │
    │  (Returns List[Document])      │
    └────────────┬───────────────────┘
                 │
                 ▼
    ┌────────────────────────────────┐
    │  PromptBuilder                 │ (docs + query → prompt)
    └────────────┬───────────────────┘
                 │
                 ▼
    ┌────────────────────────────────┐
    │  Generator (OpenAI, Anthropic) │ (prompt → answer)
    └────────────┬───────────────────┘
                 │
                 ▼
    ┌────────────────────────────────┐
    │  Final Answer with sources     │
    └────────────────────────────────┘
```

---

## Key Takeaways

**On Haystack**: It's more structured than LangChain, better for production systems with explicit pipelines and clear component contracts.

**On DocumentStore interface**: Four simple methods map cleanly to SQL—ParadeDB is a natural fit.

**On full-text search databases**: Elasticsearch dominates, but ParadeDB offers a transactional alternative for Postgres teams.

**On hybrid search**: Both Elasticsearch and Haystack expect BM25 + vectors to be standard, which plays to ParadeDB's strengths.

**On positioning**: "Transactional, Postgres-native full-text search" is a defensible market position in RAG ecosystem.

**On implementation**: 4 weeks total, following a clear LangChain-inspired roadmap.

**On community**: Smaller than LangChain (10k vs 100k), but more cohesive and production-focused.

---

## Reading Guide

**5 minutes**: Sections "What is Haystack?" + "ParadeDB's Position in Haystack"

**15 minutes**: Add "Three Types of ParadeDB Retrievers"

**30 minutes**: Add "Testing Strategy" + "Implementation Roadmap"

**1 hour**: Full document (everything)

---

# Testing Strategy (Practical Details)

## Unit Test Approach

**Goal**: Verify retriever logic without database

**Structure**:

```python
# test_simple_retriever.py

class MockParadeDBDocumentStore:
    def filter_documents(self, filters, top_k):
        # Return hard-coded test documents

def test_simple_retriever_returns_documents():
    mock_store = MockParadeDBDocumentStore()
    retriever = SimpleParadeDBRetriever(mock_store)

    results = retriever.run("test query")

    assert len(results["documents"]) > 0
    assert all(isinstance(d, Document) for d in results["documents"])
```

**Questions to answer**:

- Does retriever return `Document` objects?
- Is metadata preserved?
- Does `top_k` parameter work?
- Are edge cases handled (empty results, special chars)?
- Do outputs match Haystack component protocol?

---

## Integration Test Approach

**Goal**: Verify with real ParadeDB

**Setup** (docker-compose):

```yaml
services:
  paradedb:
    image: paradedb/paradedb:latest
    ports:
      - "5432:5432"
    environment:
      POSTGRES_PASSWORD: password
```

**Workflow**:

1. Start ParadeDB container
2. Load test documents (Kaggle IT helpdesk CSV)
3. Create BM25 index
4. Run test queries
5. Verify results are relevant

**Test queries** (Kaggle helpdesk):

- "How do I set up email?" → Should find mobile device setup doc
- "PIN reset" → Should find password reset doc
- "VPN remote work" → Should find VPN configuration doc

**Assertions**:

- Top result is relevant
- Scores are monotonically decreasing
- No errors on weird inputs

---

## Performance Test Approach

**Goal**: Verify production readiness

**Dataset**: 10,000 documents from Wikipedia or blog posts

**Benchmarks**:

- Load 10k docs: Time it
- Single query latency: Measure p50, p95, p99
- Throughput: Run 100 queries, measure QPS
- Consistency: Same query 10 times, verify same results

**Comparison**:

- SimpleParadeDBRetriever (BM25): vs Elasticsearch BM25
- HybridParadeDBRetriever (BM25 + vector): vs Elasticsearch hybrid

**Success criteria**:

- Single query: <100ms p95
- Throughput: >10 queries/sec
- Consistency: 100% same results on repeats

---

## Test Harness Example

```python
class ParadeDBRetrieverHarness:
    def __init__(self, retriever, document_store):
        self.retriever = retriever
        self.document_store = document_store
        self.tests = []

    def add_test(self, query, expected_keywords, retriever_type="simple"):
        self.tests.append({
            "query": query,
            "expected": expected_keywords,
            "type": retriever_type
        })

    def run_tests(self):
        for test in self.tests:
            results = self.retriever.run(test["query"])
            docs = results["documents"]

            # Check if any result contains expected keywords
            found = any(
                any(kw in doc.content.lower() for kw in test["expected"])
                for doc in docs
            )

            status = "✓" if found else "✗"
            print(f"{status} {test['query'][:40]} ({len(docs)} results)")

    def benchmark(self, queries, num_runs=100):
        import time
        times = []

        for _ in range(num_runs):
            for query in queries:
                start = time.time()
                self.retriever.run(query)
                times.append(time.time() - start)

        print(f"Mean: {np.mean(times)*1000:.1f}ms")
        print(f"p95: {np.percentile(times, 95)*1000:.1f}ms")
        print(f"p99: {np.percentile(times, 99)*1000:.1f}ms")
        print(f"Throughput: {len(queries) * num_runs / sum(times):.1f} QPS")
```

---

## Comparison Test Setup

Compare ParadeDB vs Elasticsearch on same queries:

```python
class ComparisonBenchmark:
    def __init__(self, paradedb_retriever, elasticsearch_retriever):
        self.pdb = paradedb_retriever
        self.es = elasticsearch_retriever

    def compare_query(self, query):
        import time

        # ParadeDB
        start = time.time()
        pdb_results = self.pdb.run(query)
        pdb_time = time.time() - start

        # Elasticsearch
        start = time.time()
        es_results = self.es.run(query)
        es_time = time.time() - start

        return {
            "query": query,
            "paradedb": {
                "num_results": len(pdb_results["documents"]),
                "latency_ms": pdb_time * 1000,
                "top_doc": pdb_results["documents"][0].content[:50]
            },
            "elasticsearch": {
                "num_results": len(es_results["documents"]),
                "latency_ms": es_time * 1000,
                "top_doc": es_results["documents"][0].content[:50]
            }
        }

    def report(self, queries):
        results = [self.compare_query(q) for q in queries]

        print("\n=== Comparison Report ===")
        print(f"{'Query':<40} {'ParadeDB (ms)':<15} {'Elasticsearch (ms)':<15}")
        print("-" * 70)

        for r in results:
            pdb_time = r["paradedb"]["latency_ms"]
            es_time = r["elasticsearch"]["latency_ms"]
            faster = "ParadeDB" if pdb_time < es_time else "Elasticsearch"
            print(f"{r['query']:<40} {pdb_time:<15.1f} {es_time:<15.1f}")
```

---

## Success Criteria

Before considering integration production-ready:

- [x] Returns `Document` objects with content, metadata, id
- [x] Respects `top_k` parameter (limit)
- [x] Handles empty results gracefully
- [x] Scores are monotonically decreasing
- [x] Metadata filtering works correctly
- [x] Single query latency <100ms p95
- [x] 100 queries complete in <10 seconds
- [x] Same query repeated returns same docs in same order
- [x] Works with Haystack pipeline
- [x] Integrates with PromptBuilder and Generator
- [x] Handles special characters, Unicode, edge cases
- [x] Error handling for bad queries, missing docs

---

## Database-Specific Tests

**For ParadeDB BM25**:

- Test query syntax (operators like `|||` for OR, `&` for AND)
- Verify TF-IDF scoring behavior
- Test phrase queries
- Test boolean operators
- Verify stemming/tokenization

**For pgvector (hybrid)**:

- Verify vector dimensions match embedding model
- Test cosine distance ranking
- Test filtering with WHERE clauses
- Verify RRF merge formula

**For metadata filtering**:

- Test date range filters
- Test string equality filters
- Test IN filters (array matching)
- Test complex AND/OR combinations
