# Partitioning Data Structures: KD-Tree vs. Hierarchical KD-Tree vs. Separable Grid

## Context and Problem

ParadeDB supports multi-key partitioning via `partition_by = 'col1,col2'` on BM25 indexes. In the StackOverflow benchmark, `stackoverflow_posts_idx` is indexed with `partition_by = 'id,owner_user_id'` across 32 segments to support joins against both:

- `comments` on `comments.post_id = posts.id` (1:N join, large table)
- `users` on `posts.owner_user_id = users.id` (N:1 join)

When executing range-partitioned distributed joins (MPP), DataFusion requires 1D `Partitioning::Range` boundaries on the join key so that partition `i` of the build side only ever joins with partition `i` of the probe side task-locally.

However, neither table achieved clean segment alignment (`partial = 0`) in queries joining on `owner_user_id`. Instead, both sides experienced partial segments across all worker tasks.

---

## 1. Existing Unconstrained KD-Tree

### Architecture

ParadeDB's current `KdTree` (`pg_search/src/index/kdtree.rs`) builds an adaptive binary tree from a sample of data:

- At every internal node, the builder calculates the rank spread of each dimension within the subset of rows reaching that node.
- It splits along the dimension with the largest spread at that node's quantile.
- Splits alternate or repeat based dynamically on local sample distributions.

### Segment Geometry

- The 32 segments represent 2D bounding boxes in $(id, owner\_user\_id)$ space.
- Cuts on `owner_user_id` inside one subtree (e.g. where $id < 10\text{M}$) are completely independent of cuts on `owner_user_id` in other subtrees (where $id \ge 10\text{M}$).
- When these 2D bounding boxes are projected onto either 1D axis, their intervals heavily overlap.

### Trade-offs

- `+` **Balanced segment sizes**: Guarantees equal sample representation in every leaf segment (~1/32 of total rows), even under correlation or non-linear distributions.
- `+` **Fully adaptive to cardinality**: Automatically allocates cuts to high-cardinality dimensions while spending minimal cuts on low-cardinality dimensions.
- `-` **Zero 1D segment alignment**: Neither dimension has disjoint 1D projected intervals. Any 1D range partitioning on either key inevitably slices through multiple 2D bounding boxes, resulting in partial segments on all workers.

---

## 2. Separable Grid Partitioning (Tensor Grid)

### Architecture

Cuts on each dimension are global hyperplanes chosen from marginal distributions:

- $N_1 = 4$ global split points on `id` ($4$ intervals).
- $N_2 = 8$ global split points on `owner_user_id` ($8$ intervals).
- $4 \times 8 = 32$ total cells, where cell $(i, j) = I_i(id) \times J_j(owner\_user\_id)$.

### Segment Geometry

- Segments are rigid grid cells.
- Along `owner_user_id`, all 4 segments in row $j$ share identical bounds $[Y_{j-1}, Y_j)$. Across rows, intervals are strictly disjoint.
- Along `id`, all 8 segments in column $i$ share identical bounds $[X_{i-1}, X_i)$.

### Trade-offs

- `+` **Two-way 1D segment alignment**: Range joins on `id` achieve `partial = 0` (8 included segments per worker). Range joins on `owner_user_id` also achieve `partial = 0` (4 included segments per worker).
- `-` **Severe data skew**: If $X$ and $Y$ are correlated (e.g. user registrations and post timestamps), data clusters along diagonals or hotspots. Some grid cells will hold millions of rows while other cells are tiny or completely empty, degrading Tantivy segment efficiency.

---

## 3. Prioritized / Hierarchical KD-Tree (Fixed Dimension Slabs)

### Architecture

Cuts are structured in a fixed hierarchy across dimensions:

1. **Level 1 (Global Slabs on Primary Key)**: Make 3 levels of binary cuts strictly on the primary key (e.g. `id`) using marginal quantiles. This creates 8 global, disjoint slabs containing exactly $1/8$ of the rows each.
2. **Level 2 (Adaptive Splits on Secondary Key)**: Inside each slab $i$, make 2 levels of cuts on the secondary key (`owner_user_id`) using the conditional distribution within that specific slab ($P(owner\_user\_id \mid id \in \text{slab}_i)$). This creates 4 sub-segments per slab.
3. Total segments = $8 \times 4 = 32$.

### Segment Geometry

- Along the primary key (`id`), intervals are strictly disjoint across the 8 slabs.
- Along the secondary key (`owner_user_id`), cuts within each slab are chosen independently to balance that slab, so their 1D projections across different slabs still overlap.

### Trade-offs

- `+` **Guaranteed segment balance (no skew)**: Because Level 2 cuts use conditional quantiles inside each slab, every segment holds exactly $1/32$ of total rows, regardless of key correlation.
- `+` **Perfect alignment on primary key**: Joins on the primary key achieve `partial = 0` across all workers.
- `+` **Better secondary alignment than unconstrained KD-tree**: Because all 8 slabs cut the secondary key into quartiles, cut boundaries across slabs land near similar numeric values rather than random points.
- `-` **Requires explicit key ordering**: Schema must designate one key as primary and the other as secondary.
- `-` **One-way alignment only**: Secondary key still produces partial segments during 1D range joins.
- `-` **Secondary prune ceiling**: Standalone filters on the secondary key must inspect all primary slabs, capping maximum segment pruning at $(N_2 - 1) / N_2$ (e.g. 75%).

---

## Summary Comparison

| Property               | Existing KD-Tree        | Separable Grid               | Hierarchical KD-Tree         |
| :--------------------- | :---------------------- | :--------------------------- | :--------------------------- |
| **Dim 1 1D Alignment** | Partial (overlapping)   | Perfect (`partial = 0`)      | Perfect (`partial = 0`)      |
| **Dim 2 1D Alignment** | Partial (overlapping)   | Perfect (`partial = 0`)      | Partial (overlapping)        |
| **Skew Resistance**    | High (adaptive)         | Poor (correlation-sensitive) | High (conditional quantiles) |
| **Segment Uniformity** | Uniform (~1/32)         | Skewed / variable            | Uniform (~1/32)              |
| **Configuration**      | Symmetric (`id, owner`) | Grid dims ($4 \times 8$)     | Primary vs secondary order   |

In the StackOverflow schema, making `id` the primary key in a Hierarchical KD-Tree gives `posts` and `comments` (the largest join at 20M x 29M rows) 100% segment alignment (`partial = 0`), while `users` (2.2M rows, easily broadcast) takes the secondary position.
