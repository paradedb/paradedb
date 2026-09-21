# TopK Aggregate (Single Table, Tantivy)

- **Join**: None (single table)
- **Description**: GROUP BY a high-cardinality field with COUNT(\*) ordered DESC
  and LIMIT 10. Tests the Tantivy TopK optimization (TermsAggregation.size=K)
  versus full aggregation + post-hoc sort.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%
