# ParadeDB + LangChain: Practical Examples & Datasets

## Dataset Examples

### Dataset 1: Kaggle IT Helpdesk (Easiest)

**Source**: https://www.kaggle.com/datasets/dkhundley/sample-rag-knowledge-item-dataset

**Download**: Click "Download" button

**What you get**: CSV file with structure:

```
ki_topic,ki_text,sample_question,sample_ground_truth
"Setting Up a Mobile Device for Company Email","Setting up a Mobile Device for Company Email Prerequisites: Mobile device with support... [full article]","How do I set up my company email on my mobile device?","To set up your company email on your mobile device, please follow these steps..."
"Resetting a Forgotten PIN","**Resetting a Forgotten PIN** If you have forgotten your PIN, you can reset it using the following...","I forgot my PIN, how can I reset it?","Don't worry, I'm here to help. To reset your forgotten PIN, you can use our self-service PIN Reset Tool..."
"Configuring VPN Access for Remote Workers","**Configuring VPN Access for Remote Workers** This article provides step-by-step instructions...","How do I set up VPN access on my laptop?","To set up VPN access on your laptop, follow these steps..."
...
```

**10 sample knowledge items** covering:

- Mobile device setup
- PIN reset
- VPN configuration
- Microsoft Office troubleshooting
- Cisco Webex setup
- File backup
- Tablet issues
- Wireless networks
- Printer jamming
- Android email setup

**How to load into ParadeDB**:

```python
import pandas as pd
import psycopg2

# Load CSV
df = pd.read_csv("rag_sample_qas_from_kis.csv")

# Connect to ParadeDB
conn = psycopg2.connect("postgresql://localhost/mydb")
cursor = conn.cursor()

# Create table
cursor.execute("""
    CREATE TABLE IF NOT EXISTS helpdesk_docs (
        id SERIAL PRIMARY KEY,
        topic TEXT,
        content TEXT,
        sample_question TEXT,
        sample_answer TEXT
    )
""")

# Insert from CSV
for _, row in df.iterrows():
    cursor.execute("""
        INSERT INTO helpdesk_docs (topic, content, sample_question, sample_answer)
        VALUES (%s, %s, %s, %s)
    """, (row['ki_topic'], row['ki_text'], row['sample_question'], row['sample_ground_truth']))

conn.commit()
cursor.close()
conn.close()
```

**Test queries**:

- "How do I set up email?" → Should find document 1
- "PIN reset" → Should find document 2
- "VPN remote work" → Should find document 3
- "printer jam" → Should find document 9

---

### Dataset 2: Blog Posts (Realistic)

**Sources** (used by LangChain tutorials):

- https://lilianweng.github.io/posts/2023-06-23-agent/
- https://lilianweng.github.io/posts/2023-03-15-prompt-engineering/
- https://lilianweng.github.io/posts/2023-10-25-adv-rl-notes/

**How to load**:

```python
from langchain_community.document_loaders import WebBaseLoader
from langchain.text_splitters import RecursiveCharacterTextSplitter
import psycopg2

# Fetch blog posts
urls = [
    "https://lilianweng.github.io/posts/2023-06-23-agent/",
    "https://lilianweng.github.io/posts/2023-03-15-prompt-engineering/",
    "https://lilianweng.github.io/posts/2023-10-25-adv-rl-notes/",
]

loader = WebBaseLoader(urls)
docs = loader.load()

# Split into chunks
splitter = RecursiveCharacterTextSplitter(
    chunk_size=1000,
    chunk_overlap=100
)
chunks = splitter.split_documents(docs)

# Insert into ParadeDB
conn = psycopg2.connect("postgresql://localhost/mydb")
cursor = conn.cursor()

cursor.execute("""
    CREATE TABLE IF NOT EXISTS blog_docs (
        id SERIAL PRIMARY KEY,
        content TEXT,
        source_url TEXT,
        chunk_index INTEGER
    )
""")

for i, chunk in enumerate(chunks):
    cursor.execute("""
        INSERT INTO blog_docs (content, source_url, chunk_index)
        VALUES (%s, %s, %s)
    """, (chunk.page_content, chunk.metadata.get('source'), i))

conn.commit()
cursor.close()
conn.close()
```

**Test queries**:

- "What is task decomposition in agents?"
- "How does prompt engineering work?"
- "What are policy gradient methods?"
- "Explain chain of thought prompting"

---

### Dataset 3: Synthetic Test Data (Fastest)

**In-memory data for unit tests** (no database needed):

```python
SYNTHETIC_DOCS = [
    {
        "id": "1",
        "content": "Machine learning is a subset of artificial intelligence that enables systems to learn from data without being explicitly programmed. Common applications include recommendation systems, image recognition, natural language processing, and predictive analytics. Machine learning algorithms can be supervised (with labeled data) or unsupervised (discovering patterns).",
        "category": "AI"
    },
    {
        "id": "2",
        "content": "Deep learning uses artificial neural networks with multiple layers to process data. Popular architectures include CNNs for image processing, RNNs for sequences, and Transformers for natural language tasks. Deep learning has achieved state-of-the-art results in computer vision and NLP.",
        "category": "AI"
    },
    {
        "id": "3",
        "content": "Natural language processing helps computers understand and generate human language. Core NLP tasks include sentiment analysis for opinion mining, named entity recognition for identifying entities, machine translation for language conversion, and text summarization for condensing information.",
        "category": "NLP"
    },
    {
        "id": "4",
        "content": "Computer vision enables machines to interpret images and video streams. Common applications include face detection for security, object recognition for inventory, autonomous vehicles for navigation, and medical image analysis for diagnostics.",
        "category": "Vision"
    },
    {
        "id": "5",
        "content": "Data science combines statistics, programming, and domain knowledge to extract insights from data. Essential tools include Python for scripting, SQL for databases, Pandas for data manipulation, and Jupyter for interactive analysis. Data scientists work with structured and unstructured data.",
        "category": "Data"
    },
]

SAMPLE_QUERIES = [
    {
        "query": "machine learning framework",
        "expected": ["1"],  # Exact keyword match
        "type": "keyword"
    },
    {
        "query": "neural network layers",
        "expected": ["2"],  # BM25 match
        "type": "keyword"
    },
    {
        "query": "understand text meaning",
        "expected": ["3"],  # Semantic match with NLP
        "type": "semantic"
    },
    {
        "query": "cars see environment",
        "expected": ["4"],  # Semantic match with autonomous vehicles
        "type": "semantic"
    },
    {
        "query": "python data analysis",
        "expected": ["5"],  # Hybrid match
        "type": "hybrid"
    },
]
```

**How to use in tests**:

```python
def test_retriever_with_synthetic_data():
    for test in SAMPLE_QUERIES:
        results = retriever.invoke(test["query"])
        result_ids = [r.metadata.get("id") for r in results]
        assert any(doc_id in result_ids for doc_id in test["expected"])
```

---

## Sample Queries for Testing

### For Helpdesk Dataset

```
Basic keyword queries:
  "setup email mobile"
  "PIN reset forgot"
  "configure VPN"
  "Office Word freeze"
  "Webex video call"

Multi-word queries:
  "how to set up company email on my phone"
  "I forgot my PIN what should I do"
  "I need to access company resources from home"

Edge cases:
  "" (empty query) → Should handle gracefully
  "xyz123 abc456" (non-existent) → Should return no results
  "a" (single letter) → Depends on tokenization
```

### For Blog Posts

```
Conceptual queries:
  "What is an agent?"
  "How do you improve model performance with prompting?"
  "Explain reinforcement learning"

Specific technique queries:
  "chain of thought prompting"
  "task decomposition"
  "policy gradient methods"
  "in-context learning"

Comparative queries:
  "difference between supervised and unsupervised"
  "when to use CNN vs RNN"
  "transformer vs recurrent networks"
```

### For Synthetic Data

```
Keyword-only (BM25 excels):
  "machine learning supervised"
  "neural networks convolutional"
  "database SQL Python"

Semantic-only (vectors excel):
  "what enables machines to see" (→ computer vision)
  "tech for language understanding" (→ NLP)
  "cars driving themselves" (→ autonomous vehicles)

Hybrid (both needed):
  "deep learning frameworks for image recognition"
  "Python tools for analyzing customer feedback" (sentiment analysis)
  "algorithms that learn from examples without labels" (unsupervised)
```

---

## What Elasticsearch Does (Learn From)

Elasticsearch's LangChain integration provides:

### 1. Multiple Retrieval Strategies (Built-in)

- **DenseVectorStrategy**: Semantic similarity search
- **BM25Strategy**: Keyword search only
- **HybridStrategy**: Combine both with RRF
- **SparseVectorStrategy**: For ELSER embeddings
- **CustomQuery**: Full Query DSL control

### 2. Flexible Query Building

```python
# Elasticsearch allows building complex queries
query_template = {
    "bool": {
        "must": [
            {"match": {"title": "machine learning"}},
        ],
        "filter": [
            {"range": {"created_date": {"gte": "2024-01-01"}}},
            {"term": {"category": "AI"}}
        ]
    }
}
```

### 3. Testing Patterns

Elasticsearch's tutorials show:

- Start simple (just vector search)
- Then add hybrid (vector + BM25)
- Then add filters (metadata)
- Finally add custom queries (complex scenarios)

### 4. Real-World Features They Demo

- **Agentic RAG**: LLM chooses which knowledge base to query
- **Date filtering**: Constrain results to specific timeframes
- **Semantic reranking**: Use Cohere to rerank results
- **Multi-field search**: Search across titles, content, metadata
- **Tool composition**: Multiple search tools for different purposes

---

## Testing Harness Example

A simple test framework you could build:

```python
class RetrieverHarness:
    def __init__(self, retriever):
        self.retriever = retriever
        self.tests = []
        self.results = []

    def add_test(self, query, expected_keywords, test_type="keyword"):
        """Add a test: does retrieval find docs with these keywords?"""
        self.tests.append({
            "query": query,
            "expected": expected_keywords,
            "type": test_type
        })

    def run_tests(self):
        """Run all tests and report results"""
        for test in self.tests:
            results = self.retriever.invoke(test["query"])

            # Check if any result contains expected keywords
            found = False
            for doc in results:
                if any(keyword in doc.page_content.lower() for keyword in test["expected"]):
                    found = True
                    break

            self.results.append({
                "query": test["query"],
                "type": test["type"],
                "passed": found,
                "num_results": len(results)
            })

    def report(self):
        """Print test results"""
        passed = sum(1 for r in self.results if r["passed"])
        total = len(self.results)

        print(f"\n=== Test Results ===")
        print(f"Passed: {passed}/{total}")

        for result in self.results:
            status = "✓" if result["passed"] else "✗"
            print(f"{status} {result['query'][:40]:<40} ({result['type']:<10}) - {result['num_results']} results")


# Usage:
harness = RetrieverHarness(my_retriever)
harness.add_test("machine learning", ["machine", "learning"], "keyword")
harness.add_test("cars see", ["autonomous", "vision"], "semantic")
harness.add_test("PIN forgot", ["reset", "password"], "keyword")
harness.run_tests()
harness.report()
```

---

## Comparison Test Setup

Test both ParadeDB and Elasticsearch on same data:

```python
class ComparisonBenchmark:
    def __init__(self, paradedb_retriever, elasticsearch_retriever):
        self.pdb = paradedb_retriever
        self.es = elasticsearch_retriever

    def compare_query(self, query):
        """Run same query on both, compare results"""
        import time

        # ParadeDB
        start = time.time()
        pdb_results = self.pdb.invoke(query)
        pdb_time = time.time() - start

        # Elasticsearch
        start = time.time()
        es_results = self.es.invoke(query)
        es_time = time.time() - start

        return {
            "query": query,
            "paradedb": {
                "num_results": len(pdb_results),
                "time_ms": pdb_time * 1000,
                "top_doc": pdb_results[0].page_content[:50] if pdb_results else None
            },
            "elasticsearch": {
                "num_results": len(es_results),
                "time_ms": es_time * 1000,
                "top_doc": es_results[0].page_content[:50] if es_results else None
            }
        }

# Usage:
bench = ComparisonBenchmark(pdb_retriever, es_retriever)

queries = [
    "machine learning",
    "neural network deep",
    "how to setup email",
]

results = [bench.compare_query(q) for q in queries]

# Print side-by-side
for result in results:
    print(f"\nQuery: {result['query']}")
    print(f"ParadeDB: {result['paradedb']['num_results']} results in {result['paradedb']['time_ms']:.1f}ms")
    print(f"Elasticsearch: {result['elasticsearch']['num_results']} results in {result['elasticsearch']['time_ms']:.1f}ms")
```

---

## Key Takeaways from Elasticsearch

**What they do well**:

1. **Multiple strategies** - let users choose (dense, sparse, BM25, hybrid)
2. **Flexible queries** - full Query DSL for power users
3. **Practical examples** - agentic RAG, filtering, reranking
4. **Clear testing patterns** - show before/after, simple to complex

**What ParadeDB can borrow**:

1. Start with SimpleParadeDBRetriever (just BM25)
2. Add HybridParadeDBRetriever (BM25 + pgvector)
3. Support SQL metadata filtering natively
4. Provide reranking examples
5. Show agentic patterns (LLM chooses which retriever)
