# Weavy core opcode and helper catalog v1

## Status

Normative companion to the [Weavy bytecode specification](weavy-bytecode-specification.md). This document defines the closed descriptor schema, family-generation rules, portable semantics, and anti-engine boundary for Gate 0. It is not by itself an enumerable opcode registry: Gate 0 Task 1 expands these rules into an owner-approved `canonical-semantic-schema.styx` and `feature-registry.styx`. Those artifacts assign every concrete canonical name, ordered descriptor, semantic tag, feature version, schema root, and identity vector. No module, codec, interpreter, or backend may consume an entry until that expansion is complete and mechanically checked against this document. Runtime registration and consumer engines hidden behind an opcode/helper/import remain forbidden.

Every semantic operation carries a canonical `InstId`. Unless specified otherwise, operands are evaluated left-to-right, all checks occur before result publication, and the fixed base resource charge occurs before operation-specific work. Checked arithmetic overflow produces `TaskFault::IntegerOverflow` at the originating `InstId`. Resource-counter overflow produces `TaskFault::ResourceExhausted` at that same site.

Every generated entry is one `OperationDescriptorV1` record with fields in this order: `canonical_name`, `major`, `minor`, `kind`, `type_parameters`, `operands`, `results`, `normal_successors`, `fault_successors`, `effects`, `faults`, `ownership`, `borrow_transition`, `builder_transition`, `cleanup_transition`, `capture_escape`, `affinity`, `suspension`, `reentry`, `evaluation_order`, `commit_point`, `base_charge`, `operand_charge_formula`, `charge_point`, `reference_algorithm`, `legalization_recipe`, and `required_oracles`. The canonical-semantic-schema authority assigns enum tags for these fields and their nested variants. An omitted field or unresolved parameter makes an entry inadmissible.

Canonical names are lowercase ASCII dotted strings. Family roots are `core.control`, `core.scalar`, `core.memory`, `core.builder`, `core.facility`, `core.table`, `core.call`, `core.adapter`, `core.suspension`, and `core.cleanup`. Family expansion is the lexicographic Cartesian product of the finite parameter sets stated below; parameters appear as dotted name suffixes in their listed order. The generated registry records derived stable IDs and candidate-local wire tags; wire tags do not define semantic identity.

## 1. Type and ownership notation

```text
T             exact semantic type
copy T        copyable value
move T        affine ownership transfer
borrow T      immutable borrow
borrow_mut T  exclusive mutable borrow
shared T      shared immutable handle
vmvec<T>      VM-owned fallible vector
builder<T>    affine typed aggregate builder
cap<C>        scoped external capability of class C
```

The family rules below supply fields shared by every generated member. Gate 0 Task 1 MUST reject an expansion unless every record has all `OperationDescriptorV1` fields, a bounded portable reference algorithm, a total legalization recipe, and differential vectors. Abbreviated signatures are explanatory only and never replace the generated descriptor.

## 2. Structural and control operations

### Constants and aliases

- `const<T>(ConstantId) -> T`: retrieves a typed admitted logical constant. Admission proves type/schema compatibility. Charge: one constant lookup.
- `copy<T: Copy>(copy T) -> T`: semantic copy. Charge: one value copy event.
- `share<T: Shareable>(borrow T) -> shared T`: fallible or infallible exactly as declared by `T`; allocation precedes publication. Charge: one share event plus declared allocation bytes.
- `drop<T>(move T) -> unit`: consumes one ownership obligation and executes the declared VM cleanup semantics. Charge precedes destruction.

### Borrow regions

- `begin_borrow<T>(borrow|borrow_mut origin, RegionId, access) -> borrow T|borrow_mut T`: opens the declared immutable or exclusive region borrow. It neither clones nor consumes the origin. Faults: none after admission. Charge: one borrow event before region-state transition. Suspension: forbidden unless a later continuation transfers a separately admitted `SuspendSafe` borrow class. Legalization retains the logical origin/region fact and materializes an address only at a checked target boundary.
- `end_borrow<T>(RegionId, move borrow T|move borrow_mut T) -> unit`: consumes the normal-path borrow obligation and closes the region use. Faults: none after admission. Charge: one borrow-end event before closure. It is distinct from `cleanup_end_borrow`, which exists only in an abrupt-path cleanup plan.

Admission proves region parent/child relationships, origin lifetime, immutable sharing versus exclusive access, CFG-edge transfers, exact one-time end/transfer on every path, and non-escape beyond invocation/input leases.

Dense `TypeId`, `ValueId`, `BlockId`, and `FunctionId` values are local aliases validated against semantic directories. They do not independently define identity.

### Terminators

- `br target(args...)`: simultaneous ownership-qualified transfer into exact target block parameters.
- `cond_br(copy i1, then(args...), else(args...))`: condition MUST be canonical 0 or 1. Only the selected edge transfers values.
- `switch<K>(copy K, cases, default)`: cases are canonical sorted unique keys or an admitted immutable dispatch table. Exactly one edge is selected.
- `return(values...)`: exact function result signature and ownership transfer.
- `fault(FaultKind)`: commits the primary typed fault then runs the admitted cleanup plan.
- `call direct/indirect`: ordinary instruction only for one-successor non-suspending contracts; otherwise uses `invoke`.
- `invoke callee(args...) normal(...) fault(...)`: target contract defines exact result/fault edges and partial-effect point.

Branch edge argument semantics are simultaneous. Implementations may derive move schedules but MUST capture all sources before dropping overwritten values.

## 3. Scalar operations

The scalar family is generated only from these finite parameter sets:

```text
IntWidth = i8 | i16 | i32 | i64 | i128 | u8 | u16 | u32 | u64 | u128
IntBinary = add | sub | mul
IntOverflow = checked | wrapping | saturating
IntDiv = div | rem
Shift = shl | shr_logical | shr_arithmetic | rotl | rotr
Bit = and | or | xor
Count = clz | ctz | popcount
Compare = eq | ne | lt | le | gt | ge
FloatWidth = f32 | f64
FloatBinary = add | sub | mul | div
FloatCompare = bit_eq | ordered_eq | ordered_ne | ordered_lt | ordered_le | ordered_gt | ordered_ge
Bool = not | and | or | xor
```

Canonical names are `core.scalar.int.<width>.<operation>.<overflow>` for `IntBinary`; `core.scalar.int.<width>.<operation>` for negation, division/remainder, shifts/rotates, bit/count/compare; `core.scalar.convert.<source>.<target>.<checked|bit_preserving>` for the exact conversions admitted by the generated type table; `core.scalar.float.<width>.<operation>`; `core.scalar.bool.<operation>`; and `core.scalar.unicode.validate` / `core.scalar.unicode.from_u32` / `core.scalar.unicode.to_u32`. Every entry is version `1.0` unless its generated descriptor explicitly records a later approved minor. Signed/unsigned legality, result type, overflow/fault set, shift-count handling, NaN rule, exact IEEE bit behavior, and legalization are generated fields, not ambient host behavior. Combinations without a complete descriptor are absent, not implicitly admitted. Base charge is one scalar event; conversion validation adds one inspected-value event. The portable reference algorithm is fixed-width mathematical/IEEE evaluation without host fast-math or target shift masking.

## 4. VM-owned memory and aggregates

### Logical VM address operations

VM addresses are typed `(RegionId, LogicalOffset)` or typed handles, never process pointers. Operations include:

- checked/proven scalar load/store;
- checked span creation and slicing;
- immutable byte/string access;
- VM-owned allocation, resize, and release;
- typed element access under admitted bounds facts.

A `proven` operation is emitted only when admission/legalization carries the exact private proof; otherwise the checked form is used. Native lowering may materialize addresses but durable semantics remain logical.

### Builders

- `builder_new<T>() -> builder<T>`: canonical name `core.builder.new`; creates uninitialized normative field state.
- `builder_init_field<T, F>(...)`: canonical name `core.builder.init_field`; rejects already initialized field.
- `builder_overwrite_field<T, F>(...)`: canonical name `core.builder.overwrite_field`; destroys prior value before replacement.
- `builder_commit<T>(...)`: canonical name `core.builder.commit`; requires all required fields and consumes builder.
- `builder_abort<T>(...)`: canonical name `core.builder.abort`; destroys initialized fields exactly once.

Each allocating action performs resource charge and fallible reservation before state mutation. Every abrupt fault uses the admitted cleanup plan. These are the sole builder feature identities; the generic facility family does not generate builder aliases.

### Generic facilities

The generic-facility catalog is generated from finite `(facility, operation)` pairs: vector/byte-buffer `{new,len,get,get_mut,reserve,push,pop,drop}`; arena `{new,alloc,get,drop}`; stack `{new,len,push,pop,drop}`; queue `{new,len,enqueue,dequeue,drop}`; min-heap `{new,len,peek,push_step,pop_step,drop}`; journal `{new,append,parent,replay_step,drop}`; rope `{leaf,concat,cursor_step,drop}`; COW state `{new,share,make_mut,get,drop}`; string `{new,len,push_scalar,slice_checked,drop}`; generation handle `{validate_generation,close}`. Canonical names are `core.facility.<facility>.<operation>`, version `1.0`. Each descriptor fixes exact types, faults, commit point, allocation-before-mutation, ownership/cleanup, and one bounded-step algorithm. Longer work is explicit SSA looping with pollpoint coverage.

## 5. Logical table operations

- `table_row_count<Table>() -> u64`;
- `table_load<Row, Field>(table, row) -> Field`;
- `table_span(table, row) -> checked_span<T>`;
- `table_dense_lookup<K,V>(table, key) -> option<V>`;
- `table_sorted_lookup<K,V>(table, key) -> option<V>`;
- `table_packed_extract<T>(table, row, field) -> T`;
- `table_callable<Sig>(table, row, field) -> callable<Sig>`.

All operations require the exact private relation/physical-view witnesses established by admission. Lookup comparison, absent/default behavior, address computation, and charge formula are part of the opcode feature version. A callable retrieval never invokes the callable.

Table operations may inspect only the declared generic schema. No opcode interprets parser actions, grammar symbols, schema tags, evaluator plans, JSON policy tags, or consumer-specific row meanings.

## 6. Calls, imports, and adapters

### Direct and indirect functions

Direct calls serialize a semantic `FunctionKey`; a validated dense `FunctionId` may be used only as an admitted in-memory alias. Indirect calls carry an exact callable signature/effect upper bound; admission proves every possible target compatible or runtime binding validates the target before invocation.

### External imports and adapters

`call_import` and `call_adapter` name a durable declaration. The declaration fixes exact types, ownership, access/alias class, effects, allocation/fallibility, partial-write point, cleanup, affinity, suspension, re-entry, and panic policy. Host panics are caught where supported and converted to the declared typed host fault; aborting panic remains outside the recoverable contract.

Live Rust/Facet data is exposed only through closed typed adapter methods. Generic adapter classes include scalar load/store, schema-relative product/enum projection, option/result view, sequence/span acquisition, iteration/lookup, pointer borrow/identity under declared policy, field initialization/overwrite/drop, and builder operations.

`hasher_write_u8/u16/u32/u64/u128/usize/isize/bytes` are distinct external-effect operations. Method identity and call order are semantic. A profile fixes VM `usize/isize` width; no host-width inference occurs.

## 7. Suspension operations

### `await_input<T>` terminator

```text
await_input<T> {
    input: InputId<T>,
    ready: BlockKey(T),
    resume: BlockKey(T),
    continuation: ContinuationSchemaId,
}
```

The terminator creates no demand. Ready validation commits to `ready`. An already-ready invalid value is an abrupt `TaskFault` at this `InstId`; it does not enter bytecode, allocate an epoch, or yield. If unready, continuation preparation and the atomic parked-state publication gate bind stable readiness epoch `q` and either publish one yield or suppress publication when supply wins and retain the value for `resume`. A first invalid value for `q` is likewise an abrupt task fault; stale/duplicate/wrong-`q` supply is diagnostic-only. Task-state revision `r` is never an input-validity key. Canonical feature name `core.suspension.await_input`, version `1.0`.

### `invoke_async_import<Req, Resp>` terminator

```text
invoke_async_import<Req, Resp> {
    import: ImportKey,
    request: move Req,
    accepted_resume: BlockKey(Resp),
    rejected: BlockKey(Req, TaskFault),
    continuation: ContinuationSchemaId,
}
```

Submission returns exactly `Accepted(AcceptanceToken)` or `Rejected { request, fault }`. Acceptance commits token and epoch `e` into `AwaitingAsync`; continuation and response race through the atomic publication gate, which publishes one yield-before-resume or suppresses it and retains the response for `accepted_resume`. A first malformed response for active `e` is an abrupt `TaskFault` at this `InstId`; it does not enter bytecode. Stale/duplicate/wrong-`e` completion is diagnostic-only. If termination wins, the token detaches and scheduler cancellation/late disposal applies. Canonical feature name `core.suspension.invoke_async_import`, version `1.0`.

Both are sealed control terminators, never result-producing ordinary instructions. Their descriptors fix successor signatures, continuation schema, failed-parking cleanup, acceptance and sole parking commit points, orphan/cancellation handoff, affinity, readiness/completion epoch rules, and exact `InstId` attribution.

### Pollpoints and safepoints

- `edge_safepoint`: may yield at an admitted wire/import boundary and has precise live ownership roots.
- `pollpoint`: observes cancellation/preemption/debug/profiling/resource work at loop backedges or bounded long operations. An unarmed pollpoint performs no Vix authority operation and no continuation spill.
- `parking_pollpoint`: a pollpoint with a `ContinuationSchema`; all live values must be `SuspendSafe`.

## 8. Cleanup operations

The cleanup-only catalog contains:

- `cleanup_drop_vm<T>`;
- `cleanup_abort_builder<T>`;
- `cleanup_end_borrow<Region>`;
- `cleanup_release_capability<Class>`;
- `cleanup_adapter_call<Method>`.

Each action consumes one unique `CleanupObligationId`. Cleanup adapter calls are non-suspending, non-capturing, non-escaping, cannot re-enter the task, and cannot invoke arbitrary imports. A caught panic marks the obligation terminal, retains the first cleanup-panic site, and continues later obligations exactly once.

## 9. Evidence-gated helper candidates

Gate 0 does not preselect primitive versus ordinary SSA/stdlib bytecode versus bounded-helper placement. The following are non-normative candidates whose exact semantics must first be expressed by the ordinary core vocabulary and whose helper form becomes an official feature only if controlled corpus evidence justifies it:

- UTF-8 scalar decode step;
- checked bounded byte-search step;
- VM-vector growth step;
- one DFA transition step;
- one heap-sift compare/swap step;
- one versioned JSON lexical-scanner step without schema dispatch or value construction.

Every candidate comparison uses identical semantic behavior, faults, resource events, and consumer oracle. Before measurement, Gate 0 may assign a descriptor a domain-separated `StudyFeatureId`, study-only schema root, and immutable study profile under `Gate0StudyPolicy` as specified by the Gate 0 plan; these identities are nonshipping, rejected by ordinary admission, and never alias `FeatureIdV1`. If selected, Gate 0 assigns a final canonical feature name/`FeatureIdV1`, complete signature, portable reference algorithm, exact work/charge formula, legalization, and differential tests before approving shipping manifests. No shipping module may require an undecided helper, and no candidate encoding receives helper-only predecode or metadata.

Any selected helper cannot call imports or callbacks, access process-global mutable state, interpret frontend tags, traverse a whole module/schema/grammar, execute parser recovery, or deserialize a value. A consumer-specific name or operand is evidence that the operation does not belong in the helper pack.

## 10. Deterministic resource events

Every operation declares:

```text
base_charge
operand_formula(public semantic quantities)
charge_point
allocation/effect/mutation/suspension ordering
```

Required event classes include instruction/helper step, allocation count/bytes, collection/arena/tree/journal node, call depth, table inspection, automaton transition, input inspection, external call, suspension, and profile-specific bounded categories such as GLR branches or scanner snapshots.

Native and fused execution may combine arithmetic only across a region with no intervening semantic fault, effect, cancellation point, or mutation and only when it preserves the first exhaustion `InstId` and complete semantic transcript.

## 11. `LegalProgramV1` contract

`LegalProgramV1` is the owner-approved target-independent PHON schema recorded by Gate 0 Task 1 before native baseline work. It contains canonical types, function signatures, functions, blocks, block parameters, legal values, origin maps, and only these closed operation classes:

1. fixed-width scalar arithmetic/bit/compare/convert with explicit fault edge;
2. logical VM load/store/span/address and checked bounds forms;
3. aggregate extract/insert plus VM allocation/release and cleanup actions;
4. table row/count/load/lookup/span operations carrying private witness references;
5. direct/indirect call, import/adapter call, and exact normal/fault successors;
6. branch, conditional branch, switch, return, and fault exit;
7. resource charge, pollpoint/safepoint, suspension, continuation materialize/resume, and abandonment operations;
8. logical parallel-copy bundles on CFG edges.

Every legal operation has fixed operand/result types, effect/fault/resource identity, `InstId` origin set, and no consumer-specific meaning. Legal values are SSA values or explicit logical frame/VM handles; host pointers, registers, stack offsets, ABI aggregates, condition flags, stencil IDs, and native relocations are forbidden. Validation proves exact block signatures, dominance, ownership, witness compatibility, effect/fault edges, resource sites, continuation schemas, and origin-map coverage.

Each approved `OperationDescriptorV1.legalization_recipe` is a total terminating rewrite into this vocabulary. A recipe may use only finite descriptor-local expansion or explicit legal loops whose bounds/resource/pollpoint contracts are preserved. Gate 0 generates a coverage table proving one recipe and differential vector for every admitted semantic entry. An unresolved recipe removes that entry from every concrete profile; target backends never add consumer-specific semantic handlers.

The schema root, enum tags, field order, validation rules, and identity vectors live in the canonical-semantic-schema authority. Task 6 cannot begin until owner approval of that artifact and complete recipe coverage.

## 12. Extension rule

Adding or changing an opcode/helper requires an official feature version and review of:

- exact types and canonical encoding;
- evaluation and partial-effect order;
- faults and exact sites;
- ownership/borrow/builder transitions;
- cleanup and panic behavior;
- suspension and affinity;
- deterministic resource events;
- portable reference semantics;
- legalization;
- anti-engine firewall;
- admission and differential oracles.

Runtime registration, producer callbacks, opaque native handlers, and consumer-owned target handlers are forbidden.
