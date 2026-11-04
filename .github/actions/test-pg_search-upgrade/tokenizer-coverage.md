# Tokenizer upgrade coverage inventory

This inventory tracks the SQL upgrade fixtures. Each fixture captures results
and tokenizer names on the starting extension version, then compares them after
upgrade and after mutable and immutable inserts. The existing action matrix
starts at v0.25.0 and the latest release; special fixtures are gated where a
tokenizer option or historical format is not available in every starting
version.

| Tokenizer or persisted name                                                        | Upgrade coverage                                                                                                                                                                                                                                                                            |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `default` / `simple`, `whitespace`, `source_code`, `literal`, `literal_normalized` | `tokenizer_option_matrix` checks positive and negative matches, source-code identifier splitting, and whole-value matching. Existing normalizer assertions distinguish raw and lowercase fast-field behavior.                                                                               |
| `keyword`, historical `keyword`, `raw`, legacy text keys                           | `tokenizer_compatibility` compares persisted field-tokenizer names and query results. `key_field_tokenizers` checks old text keys, whole-value case sensitivity, planner pushdown, and writes on the pre-0.26 row.                                                                          |
| `unicode_words_removeemojis:*`, historical `remove_emojis:*`                       | `tokenizer_compatibility` exercises emoji keep/remove, explicit and uncast queries, lowercase-sensitive behavior, result snapshots, and persisted-name comparisons for the starting versions in the action matrix. The reader also registers deprecated names used by older stored schemas. |
| `chinese_lindera`, `japanese_lindera`, `korean_lindera`, `*_keepwhitespace:*`      | `tokenizer_compatibility` checks legacy/current names for all three languages, both whitespace settings, Japanese NFKC, Japanese/Korean reading forms, trim interactions, with search overrides isolated from field-tokenizer registration.                                                 |
| `chinese_compatible`                                                               | `tokenizer_recent_options` checks all six conversion modes and conversion combined with ASCII folding and case preservation.                                                                                                                                                                |
| `jieba`                                                                            | `tokenizer_recent_options` checks all six conversion modes, both search-mode values, and conversion combined with ASCII folding and case preservation.                                                                                                                                      |
| `icu`                                                                              | `tokenizer_option_matrix` checks Arabic and Devanagari text, ASCII folding, and negative queries.                                                                                                                                                                                           |
| `ngram_mingram:*`                                                                  | `tokenizer_compatibility` checks prefix/full grams and positions; `tokenizer_option_matrix` checks the minimum gram and maximum-size boundary with negative controls.                                                                                                                       |
| `edge_ngram_mingram:*`                                                             | `tokenizer_compatibility` checks individual token character classes; `tokenizer_option_matrix` checks minimum/maximum gram boundaries and letter+digit, whitespace+punctuation, and symbol+digit class combinations.                                                                        |
| `regex`, `regex_pattern:*`                                                         | `regex_tokenizers` exercises the pre-0.26 legacy pattern-name collision, while `regex_pattern_names` covers current pattern-specific names on 0.26+; both retain result/schema checks across writes.                                                                                        |
| range tokenizer                                                                    | `range_tokenizers` snapshots stored names and checks old and new range queries after mutable and immutable writes. The fixture runs across the action’s starting-release rows.                                                                                                              |
| filter and normalizer names                                                        | `tokenizer_compatibility` covers short/long limits, lowercase, language and custom stopwords, alphanumeric filtering, ASCII folding, trimming, Lindera whitespace, and raw/lowercase normalizers. New tests combine filters with the Chinese, Jieba, ICU, and Lindera options above.        |

## Coverage boundary

A coverage case is a persisted configuration plus query/write assertions. Covering
a family is not exhaustive configuration coverage. There is no defensible overall
percentage: regex patterns and stopword lists have unbounded domains. The source
also has 11 optional shared filter fields; unset/false/true for just four boolean
filters already yields 81 combinations per applicable tokenizer.

Finite dimensions exercised here include six conversion modes for both Chinese
compatible and Jieba tokenizers, both Jieba search modes, both Unicode emoji
settings, three Lindera languages with both whitespace settings, both ngram prefix
settings, and five individual edge-ngram character classes. Numeric boundaries,
Japanese NFKC, Japanese/Korean reading forms and selected filter combinations are
sampled. We do not systematically enumerate unset versus explicit defaults, all
stemmer languages, every filter pair, or higher-order interactions. The SQL is the
executable case list; family coverage must not be read as a full configuration claim.

The boundary prioritizes persisted-name lookup, historical aliases and known
collision behavior. Save results and names on the starting release; compare after
upgrade and mutable/immutable writes. Preserve historical behavior rather than
inventing a new naming contract. Use exact-term negative controls to detect ignored
filters, and isolate search overrides so they cannot mask field lookup failures.

## Alternatives and cost

Handwritten SQL fits the existing old-release setup/upgrade/query action and keeps
expected results independent of the current naming implementation. Its cost is
manual coverage maintenance. Full Cartesian enumeration is unbounded for patterns
and lists and grows rapidly for finite settings. Random generation provides no
fixed interaction guarantee and complicates reproduction. Generated pairwise cases
would improve systematic interaction coverage, but need a reviewed catalog of
option domains and valid combinations. This patch does not claim pairwise coverage.

No extra release builds or CI jobs are added. Index creation and assertions add
work within existing jobs. No before/after runtime improvement or overhead bound
is claimed without measurement. Production tokenizer naming/registration changes
are excluded; an exposed engine defect needs its own reproduction and fix.

## Version eligibility and extension

`fixture-versions.json` declares inclusive minimum and exclusive maximum starting
versions. `select_fixtures.py` applies them uniformly to any stable release supplied
by the workflow; it does not narrow or choose the workflow release matrix.
Existing fixtures without bounds remain eligible for earlier starting releases.

The broad fixture currently declares 0.25.0 as its validated starting-release
floor, not the introduction version of every option. Lindera NFKC/reading forms
landed in `630961263` and were backported in `d27617335` (stable 0.24.2).
The complete combined fixture has not been validated on that earlier release;
its existing Unicode fixture is retained below the validated floor. Extending the
broader cases to earlier releases requires separating cases by their actual
availability and running those starting releases, rather than assuming support.
Chinese-compatible conversion (`b12272911`), configurable Jieba search mode
(`5d93a1cf9`) and pattern-specific regex names (`93d455f8b`) first appear in stable
0.26.0. These tag boundaries were checked with `git tag --contains`.

New SQL fixtures require catalog entries: selection fails on missing or stale
metadata. Add supported-version evidence, positive/negative probes and a coverage
inventory entry. If a new option requires a later release, isolate its cases in a
fixture with their own minimum instead of skipping unrelated older cases. Run
`python3 test_select_fixtures.py` to check eligibility boundaries. Production option
additions do not automatically fail a coverage audit today; this is a remaining
maintenance limitation, not an extensibility guarantee.
