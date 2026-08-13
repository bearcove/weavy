# Weavy relation contracts v1

## Status

Normative companion to the [Weavy bytecode specification](weavy-bytecode-specification.md). `RelContractV1` is a sealed enum. Producers select variants and operands; they do not provide executable validators, queries, classifiers, or trusted witnesses.

All counts and indexes are unsigned fixed-width logical values admitted into host-representable bounds before validation. All arithmetic is checked. `n` denotes the number of rows in the primary relation, `m` a referenced relation's row count, `k` a key arity, `p` a payload length, and `e` a transition count. Work units below count logical scalar reads/comparisons plus explicitly named sorting/index work; implementations may vectorize but charge the same semantic formula.

Every successful check returns a private witness containing the contract version, exact operands, logical table identities, physical view identities where used, `ExecutableId`, `ImageId` where physical access matters, and verifier epoch. Witness constructors are private to admission.

Each variant's canonical feature name is `core.relation.` followed by the snake-case variant name converted to lowercase dotted words. Every variant is feature version `1.0`. `RelContractV1` also has enum-schema version `1.0`; its canonical semantic `u32` tags are assigned below and are distinct from candidate-local physical wire tags. The canonical-semantic-schema authority copies these tags and exact field order verbatim.

## Common rules

1. A contract reads only declared columns, keys, domains, and referenced relation operands.
2. A contract MUST validate every applicable row. A producer-supplied subset is never authoritative.
3. A subset-specific contract names a sealed verifier-owned classifier version; admission recomputes membership over the complete relation.
4. Validators reserve cumulative scratch before use and fail with `AdmissionError::Limit` rather than process OOM.
5. Hash tables sized from attacker domains are forbidden. Dense bitsets require an admitted dense domain bound. Otherwise validators use sorted scans or bounded sorting.
6. Validation order is canonical: contracts sort by `(variant tag, operand tuple)` and execute in that order after physical table parsing.

## Enum

```text
RelContractV1 =
    0  RowCountEq { left, right }
  | 1  Cardinality { table, min, max_inclusive }
  | 2  ValueDomain { table, column, min, max_exclusive }
  | 3  Sorted { table, keys, order }
  | 4  SortedUnique { table, keys, order }
  | 5  DenseForeignKey { source, column, target }
  | 6  SortedForeignKey { source, source_keys, target, target_keys }
  | 7  PrefixOffsets { offsets_table, offset_column, span_count, payload_len }
  | 8  SpanWithin { table, start_column, len_column, payload_len }
  | 9  SpanPartition { offsets_table, offset_column, span_table, span_index_column,
                       payload_len }
  | 10 TagPayloadLegal { table, tag_column, payload_column, mapping }
  | 11 PartitionCoverage { table, partition_column, domain_count }
  | 12 StrictRankDecrease { transitions, source_column, target_column,
                            rank_table, rank_state_column, rank_column,
                            state_count, classifier_version, rank_lookup }
  | 13 AutomatonDomains { transitions, state_column, class_column,
                          target_column, state_count, class_count,
                          dead_state_policy }
  | 14 AcceptingCandidateConsistency { candidates, state_column,
                                       symbol_column, priority_column,
                                       state_count, symbol_count,
                                       priority_min, priority_max_inclusive,
                                       identity_columns, tie_break_order }
  | 15 CallableCompatibility { table, callable_column, signature, effect_upper_bound }
  | 16 AcyclicNonConsumingTransitions { transitions, source_column, target_column,
                                        classifier_version, state_count,
                                        source_order_witness }
```

The fields shown for each variant are the exact canonical semantic field order. Nested descriptor tags and widths are fixed by the canonical-semantic-schema authority; changing a tag or reordering/adding/removing a field is a major-version change to that variant descriptor.

## Scalar and key comparison

Keys are nonempty lists of scalar columns. Supported key scalar types are canonical integers, Unicode scalar, fixed-width byte strings, nominal IDs with declared bytewise order, and tuples thereof. Float keys are forbidden in v1. `order` is lexicographic ascending or descending with explicit per-column direction; there is no locale or host collation.

One key comparison costs one unit per inspected component through the first unequal component, or `k` for equal keys. Complexity formulas conservatively charge `k` for every comparison.

## Contracts

### `RowCountEq { left, right }`

Semantics: `rows(left) == rows(right)`.

Witness: the shared row count and both table identities.

Worst-case work: `2` metadata reads. Scratch: `O(1)`.

### `Cardinality { table, min, max_inclusive }`

Semantics: `min <= rows(table) <= max_inclusive`, with `min <= max_inclusive` checked during declaration validation.

Witness: admitted row count and bounds.

Worst-case work: `3` scalar comparisons. Scratch: `O(1)`.

### `ValueDomain { table, column, min, max_exclusive }`

Semantics: every row value `v` satisfies `min <= v < max_exclusive`. Nullability is forbidden unless the column's semantic type and contract explicitly wrap the domain in `Option`; absent values then do not satisfy an unwrapped domain.

Witness: column identity and bounds.

Worst-case work: `2n`. Scratch: `O(1)`.

### `Sorted { table, keys, order }`

Semantics: adjacent keys are monotonically ordered under canonical lexicographic comparison. Equal keys are allowed.

Witness: key descriptor and order.

Worst-case work: `k * max(n - 1, 0)`. Scratch: one prior key, bounded by admitted key width.

### `SortedUnique { table, keys, order }`

Semantics: adjacent keys are strictly ordered. This proves both sorting and uniqueness.

Witness: key descriptor and order.

Worst-case work: `k * max(n - 1, 0)`. Scratch: one prior key.

### `DenseForeignKey { source, column, target }`

Semantics: each source value `v` satisfies `0 <= v < rows(target)`. The source column is an unsigned canonical integer.

Witness: source column, target table, and target row count.

Worst-case work: `n`. Scratch: `O(1)`.

### `SortedForeignKey { source, source_keys, target, target_keys }`

Preconditions: source keys are proven `Sorted`; target keys are proven `SortedUnique`; key types and arity match.

Semantics: every source key occurs in target keys. Validation is a merge scan; duplicate adjacent source keys are permitted and consume one target match.

Witness: both key descriptors plus prerequisite witness identities.

Worst-case work: `k * (n + m)` comparisons. Scratch: one source and one target key.

### `PrefixOffsets { offsets_table, offset_column, span_count, payload_len }`

Semantics: `rows(offsets_table) == span_count + 1`; the first offset is `0`; offsets are nondecreasing; the final offset equals `payload_len`. Checked adjacent pairs define exactly `span_count` half-open spans. The offset column is an unsigned canonical integer.

Witness: offset-table identity, offset column, span count, payload length, and the proven row-count equation.

Worst-case work: `span_count + 4` scalar reads/comparisons. Scratch: one prior offset.

### `SpanWithin { table, start_column, len_column, payload_len }`

Semantics: for every row, `start <= payload_len` and checked `start + len <= payload_len`.

Witness: columns and payload length.

Worst-case work: `3n`. Scratch: `O(1)`.

### `SpanPartition { offsets_table, offset_column, span_table, span_index_column, payload_len }`

Preconditions: `span_index_column` is proven `SortedUnique` and has dense domain `0..rows(span_table)`; the offsets relation satisfies `PrefixOffsets` with `span_count = rows(span_table)`.

Semantics: row `i` of `span_table` maps to adjacent offsets `[offset[i], offset[i + 1])`; these spans cover every payload element exactly once. Empty spans are permitted unless the span-table schema separately forbids them.

Witness: prerequisite witness identities plus the exact row-to-span mapping and payload length.

Worst-case work beyond prerequisites: `rows(span_table) + 2`. Scratch: one prior offset and one span index.

### `TagPayloadLegal { table, tag_column, payload_column, mapping }`

`mapping` is a canonical sorted list of `(tag, PayloadRule)`, where `PayloadRule` is `None`, `RequiredType(TypeRef)`, or `OptionalType(TypeRef)`.

Semantics: every tag occurs exactly once in `mapping`; each row's payload presence and semantic type obey the mapped rule. Unknown tags reject admission.

Witness: mapping and exact columns.

Worst-case work: mapping declaration validation is linear in mapping length; row validation is `n * ceil(log2(mapping_len + 1))` tag comparisons plus `n` payload checks. Scratch: `O(1)` beyond canonical mapping storage.

### `PartitionCoverage { table, partition_column, domain_count }`

Semantics: each partition ID is `< domain_count` and every ID in `0..domain_count` occurs at least once. This does not require rows to be grouped.

Validation uses a dense bitset only. Admission first proves `ceil(domain_count / 8)` bytes fit the cumulative table-validation scratch limit, then scans all rows, sets each admitted bit, and scans the complete bitset/domain for absence. An implementation may vectorize this exact algorithm but may not substitute an attacker-sized hash table or implementation-dependent sorting path.

Witness: domain count and exact dense-bitset validation identity.

Worst-case work: `2n + domain_count`. Scratch: exactly `ceil(domain_count / 8)` bytes plus constant scalar state.

### `StrictRankDecrease { ... }`

The sealed `classifier_version` maps each complete transition row to `consuming` or `non_consuming` using only declared transition columns and versioned VM semantics. The producer cannot override classification. `rank_lookup` is either `DenseByState` or `SortedByState`.

Preconditions: `rank_state_column` is an unsigned state ID in `0..state_count` and is `SortedUnique`; `rows(rank_table) == state_count`, proving a total one-to-one state-to-rank mapping; `DenseByState` additionally proves row ordinal equals state ID, while `SortedByState` uses the sorted unique state key; transition source and target columns are valid state IDs.

Semantics: every transition classified non-consuming is checked exactly once, and for each edge `(s, t)`, the unique mapped ranks satisfy `rank(t) < rank(s)`.

Witness: classifier version, complete transition identity, total state-to-rank mapping identity, lookup strategy, and validated coverage.

Worst-case work after prerequisite domain/sorted witnesses: dense `e * (classifier_cost + 3)`; sorted `e * (classifier_cost + 2 * ceil(log2(state_count + 1)) + 1)`. Scratch: `O(1)` over the admitted dense or sorted table view.

### `AutomatonDomains { ... }`

Semantics: source/target states and input classes lie in their declared domains. `dead_state_policy` is one of `Explicit(id)`, `MissingTransitionIsDead`, or `TotalWithDefault(column)` and is validated exactly. Every transition row is checked.

Witness: state/class counts and dead-state policy.

Worst-case work: `3e` plus one policy check per row when applicable. Scratch: `O(1)`.

### `AcceptingCandidateConsistency { ... }`

`identity_columns` is a nonempty ordered list whose types are valid key scalars. `tie_break_order` is an explicit lexicographic direction for every identity component and MUST begin with `state_column`; it is not an opaque comparator.

Semantics:

- state and symbol IDs are in range;
- every priority satisfies `priority_min <= priority <= priority_max_inclusive`;
- rows are strictly ordered by `tie_break_order` over `identity_columns`;
- duplicate complete candidate identities are forbidden;
- each accepted candidate's symbol and priority type match the table schema.

Witness: domain bounds, identity columns, and exact tie-break descriptor.

Worst-case work: `3n + k * max(n - 1, 0)`, where `k = len(identity_columns)`. Scratch: one prior complete candidate key.

### `CallableCompatibility { ... }`

Semantics: every non-null callable reference resolves to an admitted function/import whose exact parameter/result/ownership/suspension contract is compatible with `signature` and whose effect row is a subtype of `effect_upper_bound`. The table retrieves references but never invokes them.

Witness: callable column, expected signature/effect, and resolved target set.

Worst-case work: `n * (lookup_cost + signature_components + effect_lattice_height)`, with `lookup_cost` bounded by the admitted dense function/import directories. Scratch: `O(1)` beyond resolved directory views.

### `AcyclicNonConsumingTransitions { ... }`

The sealed classifier covers every transition row exactly as in `StrictRankDecrease`. `source_order_witness` names a prerequisite `Sorted` witness over `(source_column, canonical_transition_tie_break...)`, so all outgoing edges are contiguous and deterministic.

Semantics: the directed graph formed by all non-consuming edges is acyclic over `0..state_count`.

Validation uses deterministic Kahn traversal:

1. classify every edge, validate source/target domains, and accumulate checked indegrees;
2. in one ordered scan, build an exact `state_count + 1` row-offset array over the source-grouped transition view;
3. enqueue zero-indegree states in ascending state ID;
4. traverse contiguous outgoing edges in canonical row order;
5. prove the visited count equals `state_count`.

Witness: classifier version, source-order witness, graph identity, row-offset identity, and traversal proof summary.

Worst-case work after the sorted prerequisite: `e * (classifier_cost + 4) + 4 * state_count + 1`. Scratch: exactly `(state_count + 1) * offset_width + state_count * indegree_width + state_count * queue_entry_width` bytes, with widths declared by the relation feature version and checked before allocation.

## Extension rule

Adding a variant creates a new canonical feature ID at initial version `1.0` and appends a previously unused enum tag after semantic, complexity, and anti-engine review; it does not bump an unrelated variant. A compatible change to an existing variant may bump that feature's minor version only when every prior contract remains valid; an incompatible descriptor/tag/field/semantic change bumps that variant's major version and the enum-schema major when decoding changes. No extension may arrive through runtime registration, an import, helper callback, producer-defined classifier, or opaque proof bytes.
