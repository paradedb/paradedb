# ParadeDB RAG Integrations Guide

A consolidated guide for building RAG (Retrieval-Augmented Generation) integrations across LangChain, LlamaIndex, Haystack, and DSPy frameworks.

---

## What is RAG?

RAG = Retrieval-Augmented Generation:

1. **Retrieve**: Given a user question, fetch relevant documents from a knowledge base
2. **Augment**: Add those documents as context to the user's question
3. **Generate**: Ask an LLM to answer using that context

Solves two LLM problems: **stale knowledge** (training data is outdated) and **private data** (LLMs don't know your internal documents).

---

## ParadeDB's Unique Position

ParadeDB is **search-first, SQL-second** (not a vector database):

- **Native BM25**: Production-grade full-text search (like Elasticsearch)
- **Transactional**: ACID updates, zero indexing lag
- **SQL-native**: Full Postgres in one system
- **Real-time**: Changes visible immediately

### ParadeDB vs Elasticsearch

| Dimension                | ParadeDB                         | Elasticsearch         |
| ------------------------ | -------------------------------- | --------------------- |
| **Setup**                | Docker 1 min                     | Cluster 5-10 min      |
| **Operational overhead** | Single database                  | Separate system + ETL |
| **Data consistency**     | Transactional (write-after-read) | Eventual (1-2s lag)   |
| **Real-time indexing**   | Sub-millisecond                  | 1-2 second delay      |
| **SQL access**           | Full Postgres SQL                | Query DSL only        |
| **Multi-tenancy**        | Row-level security               | Complex to implement  |
| **Hybrid search**        | BM25 + pgvector native           | BM25 + vectors + RRF  |
| **Native vector search** | Via pgvector                     | Native                |
| **Ecosystem maturity**   | New                              | Established           |

**Winner**: ParadeDB for teams who already use Postgres, need transactional consistency, or want operational simplicity.

---

## Three Retriever Types (All Frameworks)

### 1. SimpleRetriever (BM25-only)

**What**: Pure BM25 keyword search without vectors

**When to use**:

- Exact terminology matters (legal docs, technical specs, RFCs)
- Cost-sensitive (BM25 is free, no embedding model)
- Metadata filtering needed
- Getting started

**Trade-off**: Misses semantic relationships ("machine learning frameworks" ≠ "neural network libraries")

### 2. HybridRetriever (BM25 + vectors)

**What**: Combines ParadeDB's BM25 with pgvector embeddings

**How it works** (three stages):

1. **Stage 1**: BM25 search on ParadeDB (fast, broad matches)
2. **Stage 2**: Vector search on pgvector (precise, semantic)
3. **Stage 3**: RRF merge (combine rankings fairly)

**Why hybrid?**

- Query: "machine learning frameworks"
- BM25 finds: exact term matches
- Vector search finds: semantic similarity (neural networks, deep learning tools)
- **Together**: Better relevance than either alone

**When to use**: Production RAG where retrieval quality matters

### 3. MetadataAwareRetriever (BM25 + SQL filters)

**What**: BM25 search combined with SQL metadata filtering in one query

**Example**:

```sql
WHERE bm25_match(content, 'kubernetes')
  AND created_date > '2024-01-01'
  AND author_team = 'platform'
  AND status = 'published'
```

**When to use**:

- Multi-tenant systems
- Time-sensitive data (legal, financial, compliance)
- Access control (row-level security)
- Complex metadata schemas

---

## Framework Comparison

| Dimension             | LangChain        | LlamaIndex                         | Haystack                      | DSPy                   |
| --------------------- | ---------------- | ---------------------------------- | ----------------------------- | ---------------------- |
| **Philosophy**        | Tool chaining    | Data layer (ingestion + retrieval) | Pipeline orchestration        | AI program compilation |
| **Focus**             | Composition      | RAG pipelines                      | Production DAGs               | Prompt optimization    |
| **Integration point** | `BaseRetriever`  | `VectorStore`                      | `DocumentStore` + `Retriever` | `dspy.Retrieve`        |
| **Community size**    | ~100k            | ~50k                               | ~10k                          | ~5k                    |
| **Best for**          | Quick prototypes | Data-focused RAG                   | Production systems            | Optimization           |
| **Learning curve**    | Gentler          | Moderate                           | Moderate                      | Steeper                |

---

## Framework Deep Dives

### LangChain

**What it is**: The most popular framework for building LLM applications. Provides abstractions for chaining together LLM calls, tools, and data sources into complex workflows.

**Who uses it**:

- Startups building AI products quickly
- Developers prototyping chatbots, agents, and RAG systems
- Teams wanting lots of pre-built integrations (100+ vector stores, tools)

**Core concepts**:

- **Chains**: Sequences of LLM calls and transformations
- **Agents**: LLMs that decide which tools to use
- **Retrievers**: Fetch relevant documents for context
- **Runnables**: Composable units with `.invoke()` interface

**Interface**: Implement `BaseRetriever` class

```python
Retriever(query: str) → List[Document]
```

**ParadeDB implementations**:

- `SimpleParadeDBRetriever` - keyword search
- `HybridParadeDBRetriever` - keyword + semantic
- `MetadataParadeDBRetriever` - search with SQL filtering

**Reach**: Largest community (100k+), best for broad adoption

---

### LlamaIndex

**What it is**: A data framework for connecting custom data sources to LLMs. Emphasizes the "data layer" - ingestion, indexing, and retrieval - rather than orchestration.

**Who uses it**:

- Data engineers building RAG pipelines
- Teams with complex data sources (PDFs, databases, APIs)
- Developers who want simple RAG with minimal code
- Companies needing 160+ data loaders out of the box

**Core concepts**:

- **Data Loaders**: Ingest from 160+ sources (PDF, Notion, Slack, etc.)
- **Indexes**: Transform documents into queryable structures
- **Query Engines**: Combine retrieval + LLM generation
- **VectorStores**: Database abstraction for embeddings

**Interface**: Implement `VectorStore` protocol

```python
class VectorStore:
    def add(self, nodes: List[BaseNode]) -> List[str]
    def delete(self, node_id: str) -> None
    def query(self, query_embedding: List[float], similarity_top_k: int) -> VectorStoreQueryResult
```

**Gap in ecosystem**: No full-text search (BM25) native implementation

**ParadeDB angle**: "The full-text search VectorStore for LlamaIndex"

---

### Haystack

**What it is**: An open-source AI orchestration framework by deepset (Germany) for building production-ready RAG systems. Uses explicit DAG (Directed Acyclic Graph) pipelines where component connections are declared upfront.

**Who uses it**:

- Enterprise teams needing production-grade, debuggable AI systems
- DevOps engineers who want serializable, Kubernetes-ready pipelines
- Companies requiring YAML/JSON pipeline definitions
- Teams migrating from Elasticsearch to AI-augmented search

**Core concepts**:

- **Components**: Building blocks with explicit `run()` methods and typed I/O
- **Pipelines**: DAGs connecting components (serializable to YAML)
- **Document Stores**: Database abstraction layer
- **Retrievers**: Components that fetch from Document Stores

**Interface**: Implement `DocumentStore` protocol (4 methods)

```python
DocumentStore:
├─ count_documents() → int
├─ write_documents(documents, policy) → int
├─ filter_documents(filters, top_k) → List[Document]
└─ delete_documents(document_ids) → None
```

**Current state**:

- pgvector is the ONLY SQL database option
- Elasticsearch is the main full-text search option

**ParadeDB positioning**: "Transactional, Postgres-native full-text search" - fills gap between pgvector (vectors-only) and Elasticsearch (separate system)

---

### DSPy

**What it is**: A framework from Stanford NLP for "programming AI systems, not prompting them." Instead of writing brittle prompts, you declare what you want (Signatures) and DSPy automatically optimizes prompts and examples.

**Who uses it**:

- ML researchers optimizing LLM pipelines
- Teams wanting automatic prompt tuning
- Developers building complex multi-step AI reasoning
- Anyone tired of manual prompt engineering

**Core concepts**:

- **Signatures**: Declarative task specs (e.g., `"context, question -> answer"`)
- **Modules**: Reusable building blocks (Predict, ChainOfThought, ReAct)
- **Optimizers**: Automatically improve prompts/examples (BootstrapFewShot, MIPROv2)
- **Compilation**: Transform unoptimized → optimized module

**Interface**: Extend `dspy.Retrieve`

```python
dspy.Retrieve(k=5)  # Returns top-k passages
```

**Gap in ecosystem**:

- No keyword search (BM25) retrieval
- No hybrid retrieval
- No metadata-aware retrieval

**ParadeDB advantage**: Optimization potential - DSPy can automatically choose best retrieval strategy

---

## Unified Sample App: IT Helpdesk Assistant

All framework integrations use the **same sample application** for consistent end-to-end testing.

### Dataset: Kaggle IT Helpdesk Knowledge Base

**Source**: https://www.kaggle.com/datasets/dkhundley/sample-rag-knowledge-item-dataset

**Contents**: 10-100 IT support articles typical of Fortune 500 helpdesk

- Mobile device email setup
- PIN/password reset procedures
- VPN configuration for remote work
- Microsoft Office troubleshooting
- Cisco Webex conference calls
- File backup procedures
- Tablet troubleshooting
- Wireless network setup
- Printer jam resolution
- Android email configuration

**Schema** (CSV columns):
| Column | Description |
|--------|-------------|
| `ki_topic` | Article title (e.g., "Setting Up a Mobile Device for Company Email") |
| `ki_text` | Full article content (markdown formatted) |
| `sample_question` | Example user question |
| `sample_ground_truth` | Expected correct answer |

**Why this dataset?**

- ✅ Ground truth included → easy accuracy measurement
- ✅ Clear terminology → shows BM25 keyword matching value
- ✅ Practical enterprise use case → relatable to developers
- ✅ Metadata available → demonstrates SQL filtering
- ✅ Small size → fast iteration during development
- ✅ Scalable version available (100 articles) for benchmarking

---

### Database Schema

```sql
CREATE TABLE helpdesk_docs (
    id SERIAL PRIMARY KEY,
    topic TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT,                    -- e.g., 'email', 'vpn', 'hardware'
    created_at TIMESTAMP DEFAULT NOW(),
    embedding vector(1536)            -- pgvector for hybrid search
);

-- BM25 full-text search index (ParadeDB)
CREATE INDEX idx_content_bm25 ON helpdesk_docs
USING bm25 (content) WITH (key_field='id');

-- Vector similarity index (pgvector)
CREATE INDEX idx_embedding ON helpdesk_docs
USING ivfflat (embedding vector_cosine_ops);
```

---

### Test Queries (Same Across All Frameworks)

| Query                                                   | Expected Article                          | Test Type           |
| ------------------------------------------------------- | ----------------------------------------- | ------------------- |
| "How do I set up my company email on my mobile device?" | Mobile Device Email Setup                 | Exact match         |
| "I forgot my PIN, how can I reset it?"                  | Resetting a Forgotten PIN                 | Keyword match       |
| "VPN remote work laptop"                                | Configuring VPN Access                    | Multi-keyword       |
| "Microsoft Word keeps freezing"                         | Troubleshooting Microsoft Office          | Semantic + keyword  |
| "set up video call Webex"                               | Setting Up Conference Call on Cisco Webex | Synonym handling    |
| "backup important files"                                | Creating a Backup of Important Files      | Exact phrase        |
| "printer paper stuck"                                   | Resetting a Jammed Printer                | Semantic similarity |
| "Android email configuration"                           | Configuring Email on an Android Device    | Exact match         |

---

### Architecture Per Framework

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     IT Helpdesk Assistant                                    │
│                     (Same App, Different Frameworks)                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User: "How do I reset my VPN password?"                                    │
│                              ↓                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │                    Framework Integration Layer                         ││
│  │                                                                        ││
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐  ││
│  │  │  LangChain   │ │  LlamaIndex  │ │   Haystack   │ │    DSPy      │  ││
│  │  │              │ │              │ │              │ │              │  ││
│  │  │ Retrieval-   │ │ VectorStore- │ │ Pipeline +   │ │ Module +     │  ││
│  │  │ QA Chain     │ │ Index +      │ │ DocumentStore│ │ Signature +  │  ││
│  │  │              │ │ QueryEngine  │ │ + Retriever  │ │ Optimizer    │  ││
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘  ││
│  │         │                │                │                │          ││
│  │         └────────────────┴────────────────┴────────────────┘          ││
│  │                                   │                                    ││
│  │                    ┌──────────────▼──────────────┐                     ││
│  │                    │   ParadeDB Retriever        │                     ││
│  │                    │                             │                     ││
│  │                    │  SimpleMode:  BM25 only     │                     ││
│  │                    │  HybridMode:  BM25 + vector │                     ││
│  │                    │  MetadataMode: + SQL filter │                     ││
│  │                    └──────────────┬──────────────┘                     ││
│  │                                   │                                    ││
│  └───────────────────────────────────┼────────────────────────────────────┘│
│                                      ↓                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │  ParadeDB (PostgreSQL)                                               │  │
│  │  ┌─────────────────────────────────────────────────────────────────┐│  │
│  │  │ helpdesk_docs                                                   ││  │
│  │  │ ├─ id, topic, content, category, created_at, embedding         ││  │
│  │  │ ├─ BM25 index on content                                       ││  │
│  │  │ └─ pgvector index on embedding                                 ││  │
│  │  └─────────────────────────────────────────────────────────────────┘│  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                      ↓                                     │
│  Answer: "To reset your VPN password: 1) Go to vpn.company.com..."        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Framework-Specific Integration Points

| Framework      | Entry Point                         | ParadeDB Component                            | Key Method                   |
| -------------- | ----------------------------------- | --------------------------------------------- | ---------------------------- |
| **LangChain**  | `RetrievalQA.from_chain_type()`     | `ParadeDBRetriever`                           | `.invoke(query)`             |
| **LlamaIndex** | `VectorStoreIndex.from_documents()` | `ParadeDBVectorStore`                         | `.as_query_engine().query()` |
| **Haystack**   | `Pipeline().add_component()`        | `ParadeDBDocumentStore` + `ParadeDBRetriever` | `pipeline.run()`             |
| **DSPy**       | `dspy.Module` subclass              | `ParadeDBRetrieval`                           | `.forward(question)`         |

---

### End-to-End Test Scenarios

**1. Retrieval Quality Test**

```
Input:  "How do I set up email on my phone?"
Assert: Top result contains "Mobile Device" or "Company Email"
Assert: Ground truth answer is semantically similar to generated answer
```

**2. BM25 vs Hybrid Comparison**

```
Input:  "printer paper stuck" (semantic: "jammed printer")
BM25:   May miss if exact words don't match
Hybrid: Should find "Resetting a Jammed Printer" via semantic similarity
Assert: Hybrid retrieval improves recall for semantic queries
```

**3. Metadata Filtering Test**

```
Input:  "VPN setup" with filter category='networking'
Assert: Only networking-related docs returned
Assert: Email setup docs excluded even if they mention "VPN"
```

**4. Latency Benchmark**

```
Run:    100 queries from test set
Assert: p95 latency < 100ms
Assert: Throughput > 10 queries/sec
```

**5. Accuracy Evaluation**

```
For each (question, ground_truth) pair:
  generated = rag_pipeline(question)
  score = semantic_similarity(generated, ground_truth)
Assert: Average score > 0.8
```

---

## Evaluation with RAGAS

RAGAS is the standard open-source framework for evaluating RAG systems. It works with all four frameworks (LangChain, LlamaIndex, Haystack, DSPy) using a unified data format.

### Installation

```bash
pip install ragas
```

### Framework Support

| Framework      | Official Integration | How to Use                        |
| -------------- | -------------------- | --------------------------------- |
| **LangChain**  | ✅ Native            | `RagasEvaluatorChain` + LangSmith |
| **LlamaIndex** | ✅ Native            | Official cookbook                 |
| **Haystack**   | ✅ Native            | Listed in integrations            |
| **DSPy**       | ⚪ Manual            | Extract outputs → standard format |

### RAGAS Metrics

| Metric                 | What It Measures                                    | Needs Ground Truth? |
| ---------------------- | --------------------------------------------------- | ------------------- |
| **Context Precision**  | % of retrieved chunks that are relevant             | No                  |
| **Context Recall**     | % of relevant info captured in retrieval            | Yes                 |
| **Faithfulness**       | Is answer grounded in context? (anti-hallucination) | No                  |
| **Answer Relevancy**   | Does answer address the question?                   | No                  |
| **Answer Correctness** | Does answer match expected answer?                  | Yes                 |

### Data Format (Framework-Agnostic)

RAGAS evaluates a standard schema - extract these 4 fields from any framework:

```python
# Works with LangChain, LlamaIndex, Haystack, DSPy
evaluation_data = {
    "question": [
        "How do I set up my company email on my mobile device?",
        "I forgot my PIN, how can I reset it?",
        ...
    ],
    "answer": [
        # Generated answers from your RAG pipeline
        "To set up your company email on your mobile device...",
        "To reset your PIN, use the self-service PIN Reset Tool...",
        ...
    ],
    "contexts": [
        # Retrieved documents (list of strings per question)
        ["Setting Up a Mobile Device for Company Email: Prerequisites..."],
        ["Resetting a Forgotten PIN: If you have forgotten your PIN..."],
        ...
    ],
    "ground_truth": [
        # From IT Helpdesk dataset's sample_ground_truth column
        "To set up your company email on your mobile device, please follow...",
        "Don't worry, I'm here to help. To reset your forgotten PIN...",
        ...
    ]
}
```

### Running Evaluation

```python
from ragas import evaluate
from ragas.metrics import (
    context_precision,
    context_recall,
    faithfulness,
    answer_relevancy,
    answer_correctness
)
from datasets import Dataset

# Convert to HuggingFace Dataset format
dataset = Dataset.from_dict(evaluation_data)

# Run evaluation
results = evaluate(
    dataset,
    metrics=[
        context_precision,    # Retrieval quality
        context_recall,       # Retrieval completeness
        faithfulness,         # Anti-hallucination
        answer_relevancy,     # Response quality
        answer_correctness    # Match to ground truth
    ]
)

print(results)
# {'context_precision': 0.92, 'context_recall': 0.87,
#  'faithfulness': 0.95, 'answer_relevancy': 0.89,
#  'answer_correctness': 0.83}
```

### Framework-Specific Collection

#### LangChain

```python
from langchain.chains import RetrievalQA

# Run your chain
chain = RetrievalQA.from_chain_type(llm=llm, retriever=paradedb_retriever)

questions = [row["sample_question"] for row in helpdesk_data]
ground_truths = [row["sample_ground_truth"] for row in helpdesk_data]

answers = []
contexts = []
for q in questions:
    result = chain.invoke({"query": q})
    answers.append(result["result"])
    contexts.append([doc.page_content for doc in result["source_documents"]])

# Now evaluate with RAGAS
evaluation_data = {
    "question": questions,
    "answer": answers,
    "contexts": contexts,
    "ground_truth": ground_truths
}
```

#### LlamaIndex

```python
from llama_index.core import VectorStoreIndex

index = VectorStoreIndex.from_vector_store(paradedb_store)
query_engine = index.as_query_engine()

answers = []
contexts = []
for q in questions:
    response = query_engine.query(q)
    answers.append(str(response))
    contexts.append([node.text for node in response.source_nodes])
```

#### Haystack

```python
# After pipeline.run()
answers = []
contexts = []
for q in questions:
    result = pipeline.run({"query": q})
    answers.append(result["generator"]["replies"][0])
    contexts.append([doc.content for doc in result["retriever"]["documents"]])
```

#### DSPy

```python
# After compiled_rag.forward()
answers = []
contexts = []
for q in questions:
    result = compiled_rag(question=q)
    answers.append(result.answer)
    contexts.append(result.context)  # If your module exposes context
```

### IT Helpdesk Evaluation Script

Complete script for evaluating the IT Helpdesk app:

```python
import pandas as pd
from ragas import evaluate
from ragas.metrics import (
    context_precision, context_recall,
    faithfulness, answer_relevancy, answer_correctness
)
from datasets import Dataset

# 1. Load IT Helpdesk dataset
df = pd.read_csv("rag_sample_qas_from_kis.csv")

# 2. Run your RAG pipeline (framework-agnostic function)
def run_rag_pipeline(question: str) -> tuple[str, list[str]]:
    """Returns (answer, list_of_context_chunks)"""
    # Your ParadeDB retriever + LLM here
    pass

# 3. Collect results
questions = df["sample_question"].tolist()
ground_truths = df["sample_ground_truth"].tolist()

answers = []
contexts = []
for q in questions:
    answer, ctx = run_rag_pipeline(q)
    answers.append(answer)
    contexts.append(ctx)

# 4. Evaluate with RAGAS
dataset = Dataset.from_dict({
    "question": questions,
    "answer": answers,
    "contexts": contexts,
    "ground_truth": ground_truths
})

results = evaluate(dataset, metrics=[
    context_precision,
    context_recall,
    faithfulness,
    answer_relevancy,
    answer_correctness
])

# 5. Report
print("\n=== RAGAS Evaluation Results ===")
print(f"Context Precision: {results['context_precision']:.2%}")
print(f"Context Recall:    {results['context_recall']:.2%}")
print(f"Faithfulness:      {results['faithfulness']:.2%}")
print(f"Answer Relevancy:  {results['answer_relevancy']:.2%}")
print(f"Answer Correctness:{results['answer_correctness']:.2%}")
```

### Target Scores

| Metric             | Minimum | Good | Excellent |
| ------------------ | ------- | ---- | --------- |
| Context Precision  | 0.70    | 0.85 | 0.95+     |
| Context Recall     | 0.70    | 0.85 | 0.95+     |
| Faithfulness       | 0.80    | 0.90 | 0.98+     |
| Answer Relevancy   | 0.75    | 0.85 | 0.95+     |
| Answer Correctness | 0.60    | 0.75 | 0.85+     |

### Comparing Retrieval Strategies

Use RAGAS to compare BM25-only vs Hybrid retrieval:

```python
# Test 1: BM25-only
paradedb_retriever.mode = "bm25"
bm25_results = collect_and_evaluate(questions, ground_truths)

# Test 2: Hybrid (BM25 + vector)
paradedb_retriever.mode = "hybrid"
hybrid_results = collect_and_evaluate(questions, ground_truths)

# Compare
print("\n=== BM25 vs Hybrid Comparison ===")
print(f"{'Metric':<20} {'BM25':>10} {'Hybrid':>10} {'Winner':>10}")
print("-" * 50)
for metric in ['context_precision', 'faithfulness', 'answer_correctness']:
    bm25 = bm25_results[metric]
    hybrid = hybrid_results[metric]
    winner = "Hybrid" if hybrid > bm25 else "BM25"
    print(f"{metric:<20} {bm25:>10.2%} {hybrid:>10.2%} {winner:>10}")
```

---

## Key Algorithms

### BM25 Scoring

- Industry standard relevance ranking for full-text search
- Considers term frequency and document frequency
- Works well for keyword matching

### RRF (Reciprocal Rank Fusion)

- Algorithm to combine rankings from different systems
- Problem: BM25 score (e.g., 15.3) vs vector similarity (e.g., 0.87) are incomparable
- Solution: Convert both to ranks, then merge with formula: `RRF_score = 1/(k + rank)`

### Vector Embeddings

- Convert text to numbers (high-dimensional vectors)
- Similar meanings = similar vectors
- Enable semantic search

---

## Testing Strategy

### Three Testing Layers

**Layer 1: Unit Tests (No Database)**

- Mock the database connection
- Verify retriever logic, SQL construction
- Fast, run locally (1 minute)

**Layer 2: Integration Tests (Real Database)**

- Use docker-compose to spin up ParadeDB
- Load test data, verify retrieval quality
- Slower (5 minutes)

**Layer 3: Performance Tests (Benchmarks)**

- Measure latency (target: <100ms p95)
- Measure throughput (target: >10 queries/sec)
- Compare against Elasticsearch

### Test Datasets

**Option 1: Kaggle IT Helpdesk (Easiest)**

- 10 sample Q&A pairs
- Real corporate IT documentation
- Just download CSV

**Option 2: Blog Posts (Realistic)**

- Fetch from URLs, split into chunks
- Closer to production scenarios

**Option 3: Synthetic Data (Fastest)**

- Generate in code, no dependencies
- Perfect for unit tests

### Success Criteria Checklist

- [ ] Returns `Document` objects with content, metadata, id
- [ ] Respects `top_k` parameter (limit)
- [ ] Handles empty results gracefully
- [ ] Scores are monotonically decreasing
- [ ] Metadata filtering works correctly
- [ ] Single query latency <100ms p95
- [ ] 100 queries complete in <10 seconds
- [ ] Same query repeated returns same docs in same order
- [ ] Works with framework pipeline
- [ ] Handles special characters, Unicode, edge cases

---

## Implementation Roadmap

### Per-Framework Timeline: ~4 weeks each

**Week 1**: SimpleRetriever (BM25-only)

- Implement core retriever class
- Unit tests with mocks
- Integration tests with real ParadeDB

**Week 2**: HybridRetriever (BM25 + vector)

- Combine BM25 and vector results
- RRF merging algorithm
- Async support

**Week 3**: MetadataRetriever + Packaging

- SQL filter support
- Create repository, upload to PyPI
- Documentation

**Week 4**: Ecosystem Integration

- Submit to framework's integration registry
- Co-marketing with framework team
- Community support

### Parallel Execution

All four integrations can be built in parallel (10-12 weeks total):

| Framework      | Target                       | Value                          |
| -------------- | ---------------------------- | ------------------------------ |
| **LangChain**  | Large community (100k+)      | Reach                          |
| **LlamaIndex** | RAG specialists (50k)        | Data-focused devs              |
| **Haystack**   | Production RAG (10k)         | "Transactional ES alternative" |
| **DSPy**       | Research + optimization (5k) | Fill gap (no keyword search)   |

---

## Architecture Overview

```
User Query
    ↓
┌─────────────────────────────────────────────┐
│ ParadeDB Retriever                          │
│ (implements framework-specific interface)   │
├─────────────────────────────────────────────┤
│ SimpleMode:   BM25 search only              │
│ HybridMode:   BM25 + pgvector (RRF merge)   │
│ MetadataMode: BM25 + SQL filters            │
└─────────────────────────────────────────────┘
    ↓
PostgreSQL Database (ParadeDB + pgvector)
    ↓
Framework Query Engine / Chain
    ↓
LLM (OpenAI, Claude, etc.)
    ↓
Final Response
```

---

## Key Decisions

### Why BM25 First (Not Vectors)?

1. Cheaper (no embedding model cost)
2. Faster for exact matches
3. Better for technical/legal terminology
4. Baseline for hybrid comparison

### Why Hybrid (Not Either/Or)?

- BM25 catches exact terms
- Vectors catch semantic meaning
- Together > either alone for most use cases

### Why Single Database (Not Separate Systems)?

- No ETL, no sync delays
- Transactional consistency
- Operational simplicity
- SQL power for metadata

### Why These Four Frameworks?

- **LangChain**: Largest community, broadest reach
- **LlamaIndex**: Data-focused, RAG specialists
- **Haystack**: Production-focused, debuggable DAGs
- **DSPy**: Optimization potential, research community

---

## Quick Start

1. **Set up ParadeDB**: `docker run paradedb/paradedb:latest`
2. **Choose a framework** based on your needs
3. **Start with SimpleRetriever** (BM25-only)
4. **Test with synthetic data** first (no external deps)
5. **Add HybridRetriever** after BM25 is solid
6. **Benchmark vs Elasticsearch** to prove value
7. **Publish to PyPI** and framework registry
