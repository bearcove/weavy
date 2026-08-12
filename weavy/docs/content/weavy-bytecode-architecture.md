# Weavy bytecode architecture — approved v6 specification

## Status

Authoritative architecture approved by the owner after three external review rounds and a separate evidence review of the current Vix/Vixen and Phon/Vox systems that primarily justify Weavy. All named corrections through the professional review of v5 are incorporated into this approved v6. The architecture fixes the mission, authority boundary, corpus priorities, backend economics, feature composition, task-abandonment semantics, native resource envelope, and artifact lifecycle. It is the basis for the normative repository specification and Gate 0 planning; it is not an implementation plan.

The exact PHON-backed `.weavy` v1 bootstrap, directory, and acyclic codec layering are frozen current authority. The physical program-section instruction encoding, code/table policy, streaming/predecode policy, optional optimizing tier, compact/aligned/columnar PHON storage profiles inside applicable sections, native promotion policy, and initial backend algorithms remain deliberately unfrozen pending corpus evidence.

## 1. Scope, non-goals, and conformance

### Mission

Weavy is the portable verified machine for **Vix island interiors** and other compact generated Rust-adjacent programs. Vix/Vixen gives durable meaning to demands; Weavy gives cheap, discardable execution to the eager pure computation between demand boundaries.

The primary stack is:

```text
external demand
    -> canonical Vix closure and Graph VIR
    -> compiler-selected partitioned VIR islands
    -> durable architecture-neutral Weavy execution artifacts
    -> admitted interpreter or very-low-latency native tier
    -> typed wire / primitive yield
    -> Vix/Vixen scheduler, Rust async, authority, memo, Store, and receipts
```

An island is the unit of Weavy execution, not the whole Vix graph. Island interiors are eager and pure: they perform no demand nomination or validation, memo lookup, value publication, receipt recording, capability discovery, placement decision, scheduler join, or effect-ticket management. Vix partitioning may fuse or split islands for sharing, parallelism, code size, recompilation, and native-compilation economics without changing Vix semantic identity.

The same controlled substrate also serves tiny generated programs such as Phon/Vox codecs, whose relevant envelope includes direct interpretation, very low native-compilation latency, small resident/compiler footprint, and no-exec fallback. Snark, Fable, facet-json, and facet-hash/equality broaden the machine through admitted feature sets; they do not define the base kernel or obscure its primary reason to exist.

The resulting machine provides:

- one durable typed-SSA execution vocabulary with explicit feature sets;
- one hostile-input admission verifier;
- one portable reference interpreter available everywhere;
- one target-independent legalization boundary;
- compact target-specific virtual-register MIRs and complete x86-64/AArch64 encoders for their admitted feature subsets;
- a strategically primary low-latency copy-and-patch tier where measured and supported, while remaining optional for semantic correctness;
- room for an optional heavier optimizing tier only when island size/hotness measurements justify it;
- clean per-entry-point migration with same-cut deletion of replaced consumer engines.

The architecture replaces `weavy::ir::WeavyOp` and frame-offset `weavy::task::Op` as canonical executable machines, consumer interpreters hidden behind broad intrinsics or host calls, Snark runtime parser/regex engines, and duplicate consumer-native pipelines. Immutable logical tables remain first-class operands; frontend engine objects and frontend-specific table interpreters do not.

### Conformance invariant

For every migrated public entry point:

1. the frontend constructs or loads the canonical semantic module;
2. admission returns an opaque `Arc<VerifiedProgram>`;
3. static binding returns a `BoundProgram` when imports or adapters are required;
4. invocation binding creates a task with invocation-scoped capabilities;
5. every execution lane consumes only the admitted/bound program;
6. no internal fallback reaches the prior engine;
7. the prior lowering, interpreter, host-engine fallback, and consumer-native route are deleted in the same merge.

Unmigrated public entry points may remain visibly legacy until their cut. Compatibility shims that route a migrated entry point back to legacy machinery are forbidden.

### Vix/Vixen authority firewall

Weavy MUST NOT own or derive Vix `Location`, `Recipe`, or `Content` identity; memo candidate nomination or validation; observed read-sets; receipts; demand lifecycle; effect-ticket lifecycle; capability discovery; placement; Store authority; progressive-publication policy; or scheduler join/replay/cancellation policy.

Weavy may reach a typed boundary, retain discardable continuation state, and yield an attributed request. The surrounding Vix/Vixen machine decides whether that yield creates or joins a demand or ticket, what authority admits it, what is published, and how task kill/replay affects durable work.

A Weavy task contains execution frames, program counters, resource state, and suspension slots only. It carries no Vix value identity, memo authority, capability discovery, placement policy, receipt authority, or durable demand state. It may be discarded while the Vix demand and demand-owned effect ticket survive.

### Explicit non-goals

The durable machine does not contain:

- host pointers, Rust layouts, vtables, native function addresses, ABI locations, registers, stack slots, or JIT metadata;
- persisted frontend evaluator plans, parser engines, regex-engine objects, or consumer callbacks;
- an open opcode, helper, table-relation, or dialect registry;
- arbitrary host validation callbacks;
- a general native-machine-code verifier or sandbox;
- unauthenticated persisted native code execution;
- persisted physical frame offsets as semantic SSA identity;
- a requirement for executable memory on every platform;
- eager native compilation of every function or Snark state;
- backward-compatibility shims for the replaced Weavy machines.

The interpreter is always sufficient. Native availability is nonsemantic.

## 2. Versions, features, and identities

### Version and feature negotiation

A module declares:

- semantic module version;
- required sealed opcode feature sets;
- required helper feature versions;
- required logical table predicates;
- required capability/import classes and versions;
- optional nonsemantic sections and their schemas.

Admission rejects unknown required executable features. Unknown optional nonsemantic sections may be ignored according to their section contract. There is no open consumer-defined semantic dialect mechanism.

Semantic features are assigned only by a sealed, versioned official feature catalog. `weavy-core` owns canonical types, SSA/control/calls, faults, semantic resource events, base admission, the common `LegalProgram` vocabulary, and the streaming interpreter. It does not depend on, enumerate Rust types from, initialize, or link extension crates.

Each extension may depend on core and may supply declarative semantic descriptors, admission rules, reference-interpreter handlers, and legalization into the common closed `LegalProgram` vocabulary. `weavy-vix-profile` adds wire/primitive suspension, both safepoint classes, attribution, and persistent-value handles. `weavy-phon-profile` contains generic byte/cursor/output, borrow, builder, and schema-bound native-adapter facilities. It contains no `run_phon`, whole encode/decode, or schema-traversal engine operation; Phon schema-derived codec behavior is ordinary admitted SSA.

A top-level composition crate generates one immutable `RuntimeProfileManifest` at build time from an explicit official allowlist. There is no public process-local API for registering arbitrary semantic opcodes, helpers, relation predicates, legalizers, target handlers, or dialects. Cargo additive features on one monolithic semantic crate do not constitute physical isolation. Each deployable closure has a checked dependency allowlist, feature graph, build-script asset inventory, linked-symbol/section denylist, and fresh-process binary/initialization/RSS oracle.

Consumer profiles may add only sealed operations that satisfy the same granularity firewall as helpers. A profile operation may not accept or execute a consumer evaluator plan, codec plan, grammar, parser state machine, deserializer plan, or frontend tag vocabulary. Extension-owned legalizers must lower completely to the common target-independent vocabulary. Target encoders and stencils are target-owned, not consumer-owned. A stencil emits only a bounded finalized physical-form fragment selected from target MIR, with behavior already represented in the reference lane; it may not inspect consumer plans or implement hidden control traversal.

The durable container is a fixed bootstrap header plus a PHON-encoded module directory, PHON schema closure, typed program/constants, and optional nonsemantic metadata. PHON supplies portable schemas and field encoding; Weavy supplies executable organization, IDs, admission, and discoverability. Encoding/decoding lives in an acyclic sibling/lower codec layer such as `weavy-phon`, never `weavy -> phon-engine -> weavy`. The three-way instruction study below selects only the canonical encoding of the program section inside this PHON-backed container. It does not reopen PHON as the directory/schema/typed-constant substrate. Compact versus aligned/columnar PHON profiles for hot tables remain evidence-driven representations of the same logical schemas.

The repository wire specification imports the existing PHON-backed v1 bootstrap and directory contract by exact format version, header layout, directory schema identity, required section-kind namespace, and singleton/cardinality/order/overlap/integrity rules. Gate 0 may select and version only the program-section instruction schema and approved physical table-profile descriptors. A bootstrap-header, directory-schema, schema-closure, or required-section change is a separate module-format version decision.

The existing bootstrap digest field retains its physical payload-integrity meaning and is named `PayloadIntegrityTag`; it is neither `ExecutableId` nor `ImageId`. `ImageId` is computed externally over the exact complete received file bytes and need not be embedded in the image it identifies. A future format that embeds an image digest must define a versioned zeroed-field or exclusion rule. The manifest may carry an optional `ClaimedExecutableId` for inspection and transport checks. That field is excluded from canonical semantic serialization and is never authority. Admission recomputes `ExecutableId` from canonically decoded semantic content. A present claim that differs from the recomputed value produces `AdmissionError::ExecutableIdentityMismatch`.

A PHON `SchemaId` identifies the schema used to decode a physical section. It is never by itself a Weavy nominal `TypeKey`, `GroupKey`, adapter authorization, or binding identity. Admission parses the complete PHON schema closure, reconstructs the canonical Weavy semantic descriptor, and validates its full semantic digest. Any profile that maps a PHON schema to a semantic type declares a sealed injective mapping and validates it during admission or binding.

An admitted `.weavy` module is self-contained for all executable code, semantic constants, table schemas, relation declarations, and helper requirements. Only explicitly declared imports and invocation capabilities may remain process-local. Inspection of its manifest, identities, sections, required features, and schemas does not require linking the producing frontend.

### Two identities

Every admitted module has two distinct identities.

`ExecutableId` is the hash of a versioned canonical semantic serialization of:

- canonical types and signatures;
- functions, blocks, instructions, terminators, and semantic instruction identities;
- logical constants and logical table contents;
- imports and capability declarations;
- effects, faults, ownership behavior, suspension contracts, required features, and semantic resource contracts.

`ImageId` is the hash of the exact physical module bytes.

Every logical table schema defines canonical logical row order and canonical scalar/value encoding. While validating a physical table view, admission streams that canonical logical representation into `ExecutableId`; a claimed logical digest is never trusted as authority.

Repacking compact, aligned, columnar, or bit-packed storage may preserve `ExecutableId`, but always changes `ImageId`.

Cache keys follow content dependence:

- semantic analyses key primarily on `ExecutableId` plus verifier-analysis epoch;
- instruction-boundary data, offsets, physical table pointers, and other image-relative facts additionally key on `ImageId`;
- proof hints additionally key on hint schema;
- predecode additionally keys on predecode schema;
- native artifacts additionally key on legalization/backend/encoder/stencil epochs, full target/ABI/CPU/security/platform/instrumentation profile, and relevant binding identity.
- source-map caches, native fault/debug maps, profiler mappings, and placed-executor bundles additionally key on `AttributionId`;

Source paths, display formatting, profiling counters, proof hints, predecode, and native code are excluded from `ExecutableId`.

### Relationship to Vix identity and lowering artifacts

`ExecutableId` and `ImageId` identify rebuildable Weavy execution artifacts. They are never Vix `Location`, `Recipe`, or `Content` identity and never authorize memo reuse, publication, or receipts.

For Vix the authority chain is:

```text
canonical closure/source semantics
    -> Graph VIR
    -> partitioned VIR (partitioner/cost-model epoch)
    -> Weavy semantic artifact (`ExecutableId`)
    -> physical module (`ImageId`)
    -> optional target-native artifact
```

Graph VIR, partitioned VIR, Weavy modules, predecode, and native code are evictable lowering/cache tiers. Repartitioning or regenerating a Weavy artifact may change execution and compilation cost while leaving all Vix semantic identities unchanged. A placed executor may receive architecture-neutral lowered artifacts, source maps, primitive ABI requirements, capture identities, and grants without receiving `vixc`; admission proves execution support, not Vix memo authority.

## 3. Canonical type and signature graph

### Scalars

The core scalar set contains:

- `unit`;
- canonical `i1`, whose only inhabitants are 0 and 1;
- `i8/i16/i32/i64/i128` and unsigned counterparts;
- `f32/f64` with opcode-specific comparison and NaN behavior;
- Unicode scalar values excluding surrogate code points;
- fixed-width opaque nominal IDs where semantic identity requires them.

There is no ambient verifier-host `usize/isize`. Every frontend profile chooses a portable width or uses checked conversion at a native boundary. The initial Fable profile is an owner decision; the reviewed recommendation is fixed 64-bit VM `usize/isize`.

### Declared and structural types

The canonical type graph represents:

- nominal and structural products and sums;
- direct, indirect, import, and capability signatures;
- VM-owned arrays/sequences and other versioned facilities;
- VM handles and persistent store/value handles;
- scoped external capabilities and borrows;
- typed callable capabilities;
- ownership, lifetime, affinity, effect, fallibility, and suspension classes where observable.

Acyclic anonymous structural types are structurally interned.

In bytecode version 1, every recursive strongly connected declared type group carries a fixed-width `GroupKey` under a named and versioned `TypeKeyScheme`; every member carries a fixed-width `MemberKey` unique within that group. Keys are stable nominal names, not an unverifiable claim of global collision-free uniqueness.

Each group also carries a cryptographic `TypeDigest` computed from the canonical reordered semantic descriptor, including namespace, declaration identity and version, generic arguments, field/variant identities and order, mutability, ownership, lifetime, effects, and canonical internal binder references.

Admission reconstructs canonical member order, recomputes `TypeDigest`, rejects duplicate keys in one module, and rejects the same key paired with unequal descriptors. Imports and binding declarations compare both the nominal key and expected semantic digest. An external recursive-type reference is legal only through a declaration/import that supplies the expected key, digest, and key-scheme version.

Anonymous mutually recursive structural types remain inadmissible in version 1. Producers needing recursive structural data assign a canonical declared-group identity. This avoids placing general graph-isomorphism in the admission TCB.

Dense instruction `TypeId`s are aliases only. Producer declaration order and dense aliases never participate directly in semantic identity.

Representation equality never permits substitution across distinct nominal schema identities.

### Callable signatures

A callable or capability signature includes:

- exact parameter and result types;
- operand ownership and borrow behavior;
- effect upper bound;
- fault and fallibility contract;
- allocation behavior;
- capture and escape permissions;
- thread affinity;
- suspension and re-entry permissions.

A typed indirect call’s effect row is part of its type. Every possible bound target must be a subtype of the declared row.

## 4. Functions and durable SSA

Each function declares exact parameters and results, entry block, effect contract, suspension permissions, and applicable resource contract.

Each block declares ordered typed block parameters. Each instruction defines zero or more SSA values exactly once. Every use is dominated by its definition.

The durable terminator vocabulary includes:

- `br block(args...)`;
- `cond_br condition, then(args...), else(args...)`;
- typed `switch`, with code or immutable dispatch-table representation;
- `return values...`;
- explicit typed fault exit;
- direct and typed-indirect calls where the call is a control boundary;
- `invoke`-style normal/fault successors for recoverable operations;
- awaited-input suspension;
- asynchronous-import suspension.

Calls may be ordinary instructions only when their fault, effect, and continuation behavior has one successor and their results dominate later uses. Calls that suspend or have multiple semantic successors are terminators.

Critical edges are legal durably. Edge semantics are simultaneous. Interpreter edge schedules and target edge splitting are independently derived.

Every durable semantic operation has exactly one canonical `InstId`. In version 1, an `InstId` is the pair of canonical function identity and canonical instruction ordinal after producer canonicalization. Admission rejects duplicate, missing, out-of-range, or noncanonical instruction identities.

Helper/runtime subevents use `(InstId, StepOrdinal)` or another sealed child-event identity; they do not impersonate unrelated instructions. Legalization expansion carries the originating `InstId`. Fusion carries an ordered nonempty origin map and separately identifies the exact origin responsible for each possible fault, effect, charge, and observation.

Static attribution outside the executable semantic content is carried in an immutable `AttributionBundle`. Its `AttributionId` hashes the `ExecutableId`, mapping-schema version, exact `InstId -> VIR node -> island -> source` mapping bytes, partition/source-map epochs, and any source-content identities required by the embedding.

Admission or a dedicated attribution validator verifies that all referenced functions and `InstId`s exist and that ranges are canonical. A missing attribution bundle produces a specified `AttributionUnavailable` diagnostic state; a mismatched bundle is rejected and never silently attached. Runtime demand/task/effect links remain live Vix/Vixen state and are not part of `AttributionId`.

Producer canonicalization finishes before semantic serialization and `ExecutableId` computation. Admission validates semantic code; it does not insert drops, cleanup, faults, ownership transfers, or other semantic operations.

## 5. Ownership, borrowing, and builders

### Ownership classes and uses

Every value/reference class is one of:

- copyable scalar/value;
- affine owned;
- shared-counted immutable;
- immutable borrowed;
- mutable borrowed;
- scoped external capability;
- persistent store/value handle.

Every operand occurrence has a sealed use kind:

- copy;
- move;
- immutable borrow;
- mutable borrow;
- share/clone;
- drop;
- builder update;
- capability invocation.

Every CFG edge transfers explicit ownership-qualified values into matching block parameters.

For affine values, admission performs bounded finite path-sensitive transfer:

- one terminator may forward a value to multiple mutually exclusive successors;
- it may not forward the value twice within one successor;
- it may not forward one value into concurrently live fork/GLR branches;
- explicit `share` or `clone` creates values for concurrent ownership;
- every return, abrupt fault, and other exit consumes or transfers each affine value exactly once.

### Borrow regions

Borrow origins, region relationships, exclusivity, and ends are explicit. Producers emit `begin_borrow`, `end_borrow`, region relationships, and ownership-qualified block arguments. Admission proves them using bounded monotone CFG dataflow; it does not perform Rust-style lifetime inference.

A borrow may cross a call only when the target contract proves non-capture, no conflicting mutation, and sufficient lifetime. A borrow may cross suspension only when its explicit suspension class allows it. Ordinary input/native borrows are not suspend-safe.

Borrowed results cannot outlive the corresponding invocation or input lease.

### Builders and cleanup

Aggregate construction uses affine VM builder handles with normative initialization state.

The durable semantic operations include:

- `builder_init_field`, which records initialization;
- `builder_overwrite_field`, which destroys the prior initialized value according to the declared order before replacement;
- `builder_commit`, which consumes the builder and yields a completed value;
- `builder_abort`, which consumes the builder and drops initialized fields in the specified order.

Every explicit `drop`, `share/clone`, `end_borrow`, builder commit/abort, recoverable fault edge, and abrupt-fault cleanup obligation is present before serialization.

A `CleanupPlan` uses a sealed cleanup instruction vocabulary. It may perform (1) infallible VM-owned destruction and builder abort and (2) an explicitly declared `CleanupAdapterCall` for an external value whose binding requires native destruction. A cleanup adapter call names an exact bound adapter method and ownership obligation. It is non-suspending, cannot capture or escape values, cannot re-enter the same task, and cannot invoke an arbitrary import. Its declaration states whether native user `Drop` code may run and records the operation as an ordered external cleanup effect.

A cleanup plan is an ordered sequence of distinct `CleanupObligationId`s. Admission proves that no two actions consume the same obligation. Destruction steps whose correctness depends on one another must be represented as one atomic VM cleanup action or one cleanup adapter call, not as separately skippable actions.

The primary task fault is committed before cleanup begins. Every cleanup action executes at most once in declared order. If a cleanup adapter panics and unwind catching is supported, the failing obligation is marked terminal and is not retried; every later obligation executes exactly once. The first cleanup-panic site is retained. Cleanup is non-suspending, runs with cancellation suppressed, and cannot be interrupted merely because the ordinary task resource budget is exhausted.

For abrupt fault, the final result is `TaskFault::CleanupFailed { primary, first_cleanup_site }` when a cleanup panic occurred. An aborting panic or process OOM remains outside the recoverable VM contract.

Binding an external builder or native owned value succeeds only if every possible admitted cleanup obligation has a compatible cleanup adapter method. Backends execute the admitted plan and never infer native destruction from layout.

Static analysis may eliminate runtime builder bookkeeping where equivalence is proven, but observable semantics remain those of the VM builder.

## 6. Faults, effects, and evaluation order

Three failure domains remain distinct:

- malformed or unsupported module → `AdmissionError`;
- missing/incompatible static or invocation binding → `BindingError` or `InvocationError`;
- execution failure → canonical typed `TaskFault` or explicit consumer value.

They never collapse into strings or host panic.

Every execution fault records:

- canonical `FaultKind`;
- `InstId` or semantic helper/runtime-event site;
- source-attribution identity where present;
- cleanup plan;
- resource/effect ordering.

Each opcode and helper specifies:

- operand and result types;
- evaluation order;
- integer overflow and division semantics;
- float and NaN semantics;
- dynamic checks and exact fault site;
- ownership and initialization transitions;
- memory, table, input, allocation, and external effects;
- partial-write behavior;
- cancellation and resource charge points.

Opcode effects are sealed. Function effects are the least fixed point over the direct-call graph; recursive call SCCs converge over a finite effect lattice. Producer effect summaries may be untrusted acceleration hints only.

Partial external writes are operation-specific:

- Facet builder cleanup follows normative initialization state;
- a failed speculative JSON probe changes neither cursor nor output state;
- generic Hasher calls commit the ordered prefix already performed;
- asynchronous import commits request ownership only at scheduler acceptance;
- no implicit retry follows a partially committed effect.

No Rust panic or language exception crosses a generated frame. Potentially panicking adapter or user calls execute inside host trampolines that catch unwind where the embedding supports unwind. A caught panic becomes a typed host fault according to binding policy. Aborting panic and process OOM remain outside the recoverable VM contract. VM paths that promise typed OOM use fallible allocation.

## 7. Deterministic resource transcript

Resource accounting is defined over semantic events, never physical instructions or elapsed time.

Event identities include:

- `InstId`;
- `HelperStepId`;
- explicit table inspection, automaton transition, lexer step, enqueue, allocation, external-effect, suspension, and consumer-category event IDs.

Every opcode/helper/event declares:

- fixed base charge;
- operand-dependent formula based only on public semantic quantities;
- exact charge point;
- whether charge precedes allocation, external effect, state mutation, suspension, or result publication.

Counter overflow is resource exhaustion at the same semantic site. Optimized-away operations retain required charge events unless the specification explicitly says skipped semantic work is not chargeable.

Native code may combine charge arithmetic only across a region with no intervening semantic fault, effect, cancellation point, or observable mutation, and only when the first exhaustion site is preserved.

Runtime budgets cover at least:

- semantic instruction/helper events;
- allocations and allocated bytes;
- collection/arena/tree/journal nodes;
- call depth;
- table inspections and automaton transitions;
- input inspection;
- external calls;
- suspension events;
- consumer categories such as GLR branches, recovery frontier, dedup entries, and scanner snapshots.

Cancellation is observed only at declared pollpoints or suspension cuts. Interpreter and native lanes produce the same resource-event transcript up to permitted nonsemantic profiling differences.

## 8. Logical tables and relation contracts

A logical table is a semantic module operand, not a frontend object and not a physical PHON profile.

Each declaration contains:

```text
TableSchema
CanonicalLogicalOrder
PhysicalViewDescriptor
RelContract[]
```

`TableSchema` defines typed columns, keys, callable-reference types, and canonical scalar/value encoding. `CanonicalLogicalOrder` defines the representation hashed into `ExecutableId`. `PhysicalViewDescriptor` selects compact, columnar, packed, aligned, or another admitted storage profile without changing semantics.

Base table operations are limited to schema-typed row/field loads, checked spans/slices, packed-field extraction, sparse/dense lookup with fully specified comparison/addressing semantics, and retrieval of typed values or typed callable references. A table can return a callable; it never invokes the callable or interprets frontend tags.

### Closed relation language

`RelContract` is a sealed, versioned enum. No producer callback, arbitrary query, or host validator is permitted. The executable specification MUST enumerate every version-1 variant with exact operands, semantics, witness type, worst-case work, and auxiliary-memory formula. Its minimum required vocabulary is:

- `RowCountEq(a, b)`;
- bounded cardinality and `ValueDomain(column, min, max_exclusive)`;
- `Sorted(keys...)` and `SortedUnique(keys...)`;
- `DenseForeignKey(column, target_count)`;
- merge-verifiable sorted foreign keys;
- `PrefixOffsets(offsets, payload_len)`;
- `SpanWithin(start_col, len_col, payload_len)`;
- `SpanPartition(offsets, payload_len)`;
- tag-to-payload legality;
- partition/index coverage;
- `StrictRankDecrease(rank_column, classifier_version)`, requiring strict decrease on every transition selected by the sealed verifier-owned non-consuming-edge classifier over the complete admitted transition relation;
- automaton state/class/transition domains;
- accepting-candidate/symbol/priority consistency;
- callable signature/effect compatibility;
- `AcyclicNonConsumingTransitions`;

Every predicate specifies worst-case time, work-unit charge, and maximum auxiliary storage. Admission checks cumulative budgets before allocation. Large-domain relations use dense domains or sorted indexes permitting linear or bounded `n log n` validation rather than attacker-sized hash sets.

Successful validation creates a private typed witness tied to exact table views, `ExecutableId`, and where physical access matters `ImageId`. Unchecked derived access is available only through accessors requiring the witness. Witnesses are not serialized as trusted facts and are not publicly constructible.

`RelContract` variants are subject to the same anti-engine review as opcodes and helpers. A relation predicate may inspect only declared logical columns, keys, domains, and relation operands. It may not contain consumer grammar semantics, evaluator behavior, schema dispatch, parser actions, recovery, tree construction, or frontend tag interpretation.

A predicate whose proof applies to a semantic subset must define that subset through a sealed verifier-owned classifier and prove coverage of every applicable row. A producer may not supply an unchecked list of only the rows it wishes to validate.

In particular, `StrictRankDecrease` binds to the complete admitted transition relation and a sealed non-consuming-edge classifier. Admission proves that every transition classified as non-consuming is represented exactly once in the checked relation and that its rank strictly decreases. The resulting witness authorizes only the exact table views and classifier version covered by that proof. Adding a relation variant requires a new reviewed official relation-feature version; it cannot occur through runtime registration.

## 9. Portable automata and progress

Regex source and process-local regex engines are compiler inputs, not executable semantics.

The portable automaton representation freezes:

- byte or Unicode-scalar class ranges;
- transition interpretation;
- dead/default state behavior;
- acceptance candidates and deterministic ordering;
- anchors, end conditions, and lookahead state;
- maximal-munch and precedence behavior;
- inspection/resource events.

Unicode properties normally expand at compile time into explicit ranges. A runtime Unicode property helper must name an exact versioned Unicode database/feature.

Automata execute through ordinary SSA and logical tables, or through a normative generic helper no larger than a bounded transition/step operation. Code/table/hybrid representation is a producer decision bounded by code, table, admission, RSS, and execution budgets.

Zero-width behavior has two distinct obligations:

1. admission proves an automaton has no non-consuming cycle using `AcyclicNonConsumingTransitions`, or validates a finite rank that strictly decreases on every non-consuming edge using `StrictRankDecrease`; both predicates have linear work and storage bounds in the executable specification;
2. a parser accepting zero-width tokens has a separate parser-level progress certificate or bounded runtime deduplication/resource policy.

Cursor advancement is not a universal progress rule. Dynamic GLR/recovery state remains protected by deterministic deduplication, category quotas, and fuel.

A table may yield a typed callable reference only when its relation witness proves signature and effect compatibility.

## 10. VM-owned mutable facilities and helper registry

The portable machine may provide versioned, bounded facilities shared across consumers:

- growable vectors and arenas;
- immutable/shared and copy-on-write state;
- stacks, queues, and min-priority worklists built from primitives or bounded helpers;
- parent-linked journals and persistent child-list/rope structures;
- VM strings and byte buffers;
- typed aggregate builders;
- task/session handles with generation and ownership checks.

Facilities enter feature sets only when corpus evidence and consumer requirements justify them. Each specifies allocation, fallibility, resource charging, ownership, and cleanup.

### Helper firewall

A normative VM helper is a sealed ISA feature, not an import. It:

1. has stable semantic ID and version;
2. has complete operand/result/effect/fault/ownership semantics;
3. operates only on VM-owned values, logical tables, or an explicitly named generic capability class;
4. cannot call host imports or consumer callbacks;
5. has no hidden process-global mutable state;
6. exposes deterministic semantic charge events;
7. has a portable reference algorithm and independent differential tests;
8. is bounded per invocation or an explicit incremental state machine with pollpoints;
9. contains no consumer schema, grammar, evaluator plan, or frontend tag vocabulary.

Plausible helpers include UTF-8 scalar decoding, checked byte search, vector growth, one DFA transition, one heap-sift step, or a versioned JSON lexical-scanner step.

Forbidden helpers include whole parser execution, GLR scheduling, parser recovery, schema dispatch, Facet construction, whole deserialization, evaluator execution, or whole-program traversal.

If a helper cannot be specified without naming Snark, Fable, facet-json, or another consumer, it belongs above the VM boundary. The helper registry receives the same semantic and security review as the opcode registry.

## 11. Imports, adapters, and bindings

### Durable declarations

Admission validates every import, capability, and adapter method declaration independently of implementations. Each declaration includes:

- stable class and semantic version;
- canonical schema/path signature where the operation addresses external data;
- exact request/result types;
- access mode and alias class;
- consumed, borrowed, and produced ownership;
- capture and escape permissions;
- effects, including whether user code may run;
- allocation and fallibility;
- partial-write and cleanup behavior;
- thread affinity;
- suspension and re-entry policy;
- panic policy;
- cleanup-only method status and whether native user `Drop` code may run;
- cleanup ordering, non-suspension, non-capture, non-escape, and secondary-panic policy where applicable.

An adapter operation is optimizer-pure only when binding proves it cannot allocate, mutate hidden state, invoke user code, trap through host behavior, or capture a reference.

Live Rust/Facet values are accessed through closed typed adapters, never serialized `Shape`, layout, offsets, pointers, or vtables. Adapter surfaces may include schema-relative product/enum projection, scalar load/store, option/result view, sequence/span acquisition, collection iteration/lookup, dynamic projection, pointer borrow/identity, initialization/overwrite/drop, and builders. Every method carries the complete declaration above.

Generic Rust `Hasher` parity uses distinct typed external-effect operations such as `hasher_write_u32`, `hasher_write_usize`, and `hasher_write_bytes`. Method identity and order are semantic. These calls are not batchable into a byte stream under the generic Hasher contract.

### Static versus invocation binding

The embedding API has separate stages:

```rust
bind(
    program: Arc<VerifiedProgram>,
    bindings: StaticBindings,
) -> Result<BoundProgram, BindingError>;

start<'inv>(
    program: &'inv BoundProgram,
    invocation: InvocationBindings<'inv>,
    limits: InvocationLimits,
) -> Result<Task<'inv>, InvocationError>;
```

Equivalent owned or lease-based APIs are acceptable when a task must outlive a Rust borrow.

`BoundProgram` contains immutable long-lived import implementations, adapter implementations, authorization, and `BindingSetId`. It contains no invocation pointer.

`InvocationBindings` contains per-task input capabilities, output targets, live native roots, hashers, scanner instances, and other invocation objects. Every entry records lifetime region, generation, mutability, ownership, affinity, and suspension policy.

`VerifiedProgram` is freely shareable. `BoundProgram` and `Task` may intentionally be non-`Send` or affinity-constrained.

A direct-linked native artifact either holds a strong immutable `BindingLease` for its entire executable lifetime or performs an admitted generation guard at each external boundary. Revocation prevents new calls according to an explicit in-flight policy. Cache-key change alone is not revocation.

Callable capabilities use the same declarations, including capture ownership, expiry, revocation, and re-entry. Re-entry into the same suspended task is forbidden in version 1.

## 12. Suspension and tasks

Every value type has a verifier-owned suspension class. Only `SuspendSafe` values may appear in a continuation schema. Copyable scalars, affine VM-owned values, shared immutable values, and generation-checked persistent handles may qualify. Transient borrows and invocation capabilities qualify only when their declared lifetime and affinity explicitly permit suspension.

Every suspension site has a `ContinuationSchema` containing:

- site and task-generation identity;
- ordered live values with exact types and ownership;
- cleanup ownership if parking fails;
- resume block and block-argument signature;
- allowed capability generations and affinity;
- semantic resource counters that survive;
- completion materialization contract.

Admission rejects any suspension live set containing a value without an allowed suspension class.

### Vix wire boundary

`await_input<T>(InputId<T>) -> T` is the generic machine form used by a Vix `AwaitWire` edge:

- it performs no external effect and creates no demand by itself;
- it yields only when the executed island path actually consumes the wire;
- it parks at the same semantic site while unready;
- it consumes one matching readiness generation only after a complete value has been validated into temporary task-owned storage;
- it supports admitted suspend-safe closed values, not every closed type indiscriminately;
- it never parks with a transient pointer into interpreter, predecode, or host-frame storage.

The typed yield reports the wire identity/site and continuation contract to the Vix/Vixen scheduler. That scheduler nominates or joins the dependency demand, performs memo/read-set work, and eventually supplies the admitted result. An unconsumed wire issues no demand.

### Authority-crossing primitive boundary

`invoke_async_import<Req, Resp>` is the generic machine boundary used by Vix `InvokePrimitive`, but Weavy does not own the Vix demand or effect-ticket protocol. The scheduler-submission ABI returns exactly one of:

```text
Accepted(AcceptanceToken)
Rejected { request: Req, fault: TaskFault }
```

Request ownership transfers only with `Accepted`. `Rejected` returns ownership-equivalent unchanged request state to the task and performs no transfer cleanup. The task then follows the declared fault successor or abrupt-fault plan at the original invocation site.

The `AcceptanceToken` binds import identity, task generation, site, request identity, and response contract and is created exactly once. No version-1 import may choose an alternative rejection-consumes-request policy. Completion is accepted at most once by the retained continuation; the full response is validated into temporary storage before continuation state changes, so invalid or stale completion performs no partial state mutation.

For Vix, the scheduler derives or joins the primitive demand, and the demand-owned effect ticket may survive task death, replay, or another waiter joining. Killing a Weavy task does not by itself cancel, restart, memoize, publish, or receipt the external operation. Final-obligation cancellation, late completion, replay, and join behavior remain Vix/Vixen policy.

Native continuation materialization exists only on a taken slow edge and is checked against the admitted `ContinuationSchema`. If materialization allocation fails, the task faults before scheduler handoff and runs the exact cleanup plan. An untaken interior pollpoint performs no continuation spill.

Full edge safepoints and cheap interior pollpoints are distinct. Edge safepoints may yield to the embedding at wire/primitive boundaries. Interior pollpoints occur at loop backedges and bounded long operations for cancellation, cooperative preemption, debugging, profiling, counter flushing, and future memory management; when unarmed they perform no Vix identity, memo, receipt, publication, demand, or scheduler operation and cannot publish partial molten state.

### Task abandonment

Every admitted task state at which the embedding may discard a task has a verifier-owned `AbandonPlan`. For a parked task, the plan is part of its continuation facts. For a running task, discard becomes effective only at an admitted safepoint or pollpoint after the runtime has exclusive ownership of the task; asynchronous destruction of a generated stack is forbidden.

The abandonment plan covers every live task-owned affine value, builder, borrow end, invocation capability, and frame obligation across the complete frame chain. It excludes any request whose ownership already transferred with `Accepted` and excludes scheduler-owned demand or ticket state.

Abandonment first marks the task terminal and invalidates its completion generation, then executes its plan exactly once on the required affinity. A concurrent or later completion is stale and cannot mutate the task. Cleanup failure leaves the task discarded and produces a typed task-cleanup diagnostic; it does not cancel, restart, publish, or otherwise alter the Vix demand or ticket.

An interior pollpoint that may park and later resume carries a `ContinuationSchema` and is subject to the same `SuspendSafe` live-set validation as any other suspension site. A pollpoint without such a schema may observe cancellation or request transfer to a later full safepoint, but may not park a resumable task with unmaterialized state.

`AbandonPlanId` is part of admitted continuation/task facts and interpreter/native parity transcripts.

## 13. Admission

```text
admit(image, policy, limits) -> Arc<VerifiedProgram>
bind(program, static_bindings) -> BoundProgram
start(bound_program, invocation_bindings, limits) -> Task
```

### Phase 0

Before attacker-controlled counts can drive allocation, validate:

- total raw bytes and maximum decoded bytes;
- section count, ranges, overlap, ordering, alignment, and integrity;
- checked cumulative counts and arithmetic;
- maximum types, recursive groups, functions, blocks, instructions, operands, CFG edges, tables, rows, imports, and capabilities;
- maximum nesting/recursion depth;
- declared physical-table expansion;
- impossible lengths and noncanonical encodings.

Decoder APIs never reserve directly from untrusted counts.

### Admission limits

`AdmissionLimits` includes:

- raw and decoded byte limits;
- total operands and CFG edges;
- total derived-facts bytes;
- work units per phase and in total;
- table-validation scratch;
- per-function values, edges, block arguments, and liveness facts;
- recursive-group size and refinement rounds;
- physical-table expansion and owned-decompression budgets.

Every phase publishes worst-case time and auxiliary-memory complexity. Allocation follows checked cumulative accounting. Dense value-by-block or table-by-table matrices are forbidden unless their exact admitted size fits the relevant budget.

Exceeding work or memory budgets yields typed `AdmissionError::Limit`, never allocator panic or process OOM.

### Ordered phases

1. bootstrap, section bounds/order/integrity, `ImageId`;
2. feature/version/import-vocabulary support negotiation;
3. canonical type graph, declared recursive groups, nominal identity;
4. logical constants/table schemas and physical borrowed-view parsing;
5. logical constant and table-content canonicalization, streaming their canonical component digests into a provisional semantic-hash accumulator; this phase does not make `ExecutableId` available;
6. function/block directory and canonical instruction boundaries;
7. operand shapes, definitions, uses, terminators, and block signatures;
8. canonical decoding of every remaining semantic section followed by final canonical semantic serialization and `ExecutableId` computation; compare any `ClaimedExecutableId`;
9. CFG, normalized edge sets, reachability, predecessors, dominance, SSA/block arguments;
10. ownership, initialization, borrows, explicit cleanup, capability non-escape;
11. logical-region and VM-owned value safety;
12. closed table-domain and relation-contract validation;
13. effects, imports, calls, faults, source sites, evaluation order;
14. suspension live sets, continuation schemas, response materialization, and abandonment plans;
15. static resource obligations and pollpoint coverage;
16. private immutable `TrustedFacts` finalization.

No semantic-analysis cache, proof hint, attribution bundle, or other object keyed by `ExecutableId` may be consulted before phase 8 completes. An invalid module may have a recomputed content identity, but it cannot produce `TrustedFacts`.

Duplicate/redundant CFG encodings are normalized or rejected before expensive analyses according to the canonical encoding rule.

`TrustedFacts` contain admitted image ownership, identities, canonical types, block/instruction views, CFG/dominance, private table witnesses, effects, ownership/cleanup states, fault/source maps, suspension cuts, execution requirements, and cache identities. They are neither publicly constructible nor mutable.

Derived interpreter slot plans, move schedules, and other large execution metadata may be lazy immutable verifier-owned caches. They need not inflate initial admission, but their maximum work and retained bytes are reserved within checked per-program derived-cache limits established by admission. Construction uses fallible reservation, consumes the reserved work budget, and publishes only a complete immutable cache. Concurrent builders for the same cache key are coalesced or independently bounded so duplicate work/residency cannot exceed the program limits. Budget exhaustion, allocation failure, cancellation, or construction failure discards partial state and preserves the safe streaming-interpreter path; it never invalidates the admitted program or produces process OOM.

Admission establishes a mandatory bounded `BaseExecutionPlan` sufficient to run the reference interpreter without optional slot, move-schedule, or predecode caches. The base plan records exact maximum frame bytes, value-cell requirements, maximum block-argument arity, edge scratch bytes, call-depth contribution, and suspension-resident bytes. A conforming fallback may use one typed cell per SSA value and a generic simultaneous edge-copy algorithm with scratch bounded by the maximum admitted edge arity, or an observationally equivalent representation.

If the mandatory base bounds cannot be represented within admission limits, admission rejects the module. Per-task allocation of admitted base storage is fallible and produces the specified execution-allocation fault. Slot reuse, precomputed cycle schedules, fused handlers, and predecode are optional immutable optimizations. Their construction may fail or be discarded without preventing execution through the base plan.

Proof hints are optional, untrusted, discardable, and locally revalidated. They are keyed by both identities as required, verifier-analysis epoch, and hint schema.

## 14. Reference interpreter

The reference lane is a safe streaming switch interpreter over immutable bytes owned or leased by `VerifiedProgram`.

Execution uses:

- admitted block/instruction directories;
- type-specialized handlers over admitted operands;
- exact typed storage banks/classes;
- the mandatory bounded `BaseExecutionPlan`, which may use one typed cell per SSA value;
- a generic simultaneous edge-copy algorithm bounded by admitted edge arity;
- explicit task call stack and parked state;
- verified logical table views;
- exact cleanup/fault actions;
- semantic resource charging;
- optional nonsemantic profiling counters.

SSA values remain semantic values, not persisted slots. Slot assignment is derived execution metadata. The initial allocator may use conservative live intervals over deterministic block order; it need not freeze graph coloring. Aggregate values are VM handles or independently typed components, never overlapping raw aggregate byte ranges.

Edge moves are simultaneous. A derived schedule uses one typed temporary per cycle or equivalent safe cycle breaking. Drops occur only after all edge sources have been captured. Move scheduling cannot fault.

Only values named by an admitted continuation schema become task-resident at suspension. Ordinary call-local values remain interpreter-local.

Lazy predecode is immutable and discardable. It may contain decoded internal tags, slot assignments, block/table indices, and move plans, but no semantic state or unowned process pointer. It keys on `ExecutableId`, `ImageId`, verifier epoch, and predecode schema. Its construction and residency obey the admitted per-program derived-cache work/byte limits, fallible atomic publication, concurrent-build coalescing/bounding, and streaming fallback above.

Fused handlers and predecode preserve original `InstId`, fault, effect, source, and resource identities.

## 15. Target-independent legalization

`LegalProgram` remains target-independent SSA. It preserves:

- blocks and block parameters;
- semantic instruction identities;
- effects, faults, and source identity;
- ownership outcomes and cleanup obligations;
- suspension sites and continuation schemas;
- semantic resource events.

Legalization may:

- expand rich operations into a closed portable low-level vocabulary;
- choose versioned target-independent representations for explicitly VM-owned storage based on logical offsets and handles;
- emit primitive arithmetic/comparisons, logical VM-address operations, checked/proven accesses, table operations, and normative helper calls;
- expose materialization requirements only at actual suspension slow paths.

It does not choose:

- pointer width or host aggregate layout;
- ABI argument/result locations;
- register pairs, stack slots, flags, branch forms, or stencil IDs;
- external adapter native layout.

Values such as `i128` and logical aggregates may remain abstract through legalization when premature limb/layout selection would constrain allocation or ABI choices. External adapter operations remain abstract capability operations.

## 16. Target backends and artifact lifecycle

### Native-tier policy

Interpretation is the universal correctness lane. On supported desktop/server profiles, the first native target is a compact, very-low-latency baseline whose compile cost remains useful for Vix island granularity and tiny Phon/Vox codec programs. Copy-and-patch is optional with respect to semantics and platform support and is the strategically preferred baseline hypothesis, but corpus evidence selects among interpreter-only execution, ordinary compact encoding, ordinary encoding with measured stencil substitution, or a size-dependent combination.

The compact backend always has a complete target-MIR encoding path. Gate 0 measures identical admitted programs in three modes with identical metadata, pollpoint, attribution, and source-map obligations: (A) streaming interpretation; (B) complete ordinary compact native encoding with stencil substitution disabled; and (C) the same backend with measured copy-and-patch substitutions enabled. Every admitted form has an ordinary macro-assembler/encoder path; missing or rejected stencils fall back to it.

Variants B and C consume byte-identical admitted programs and identical `LegalProgram`, target-MIR, instruction selection, physical operand constraints, register assignment, frame/root/fault/safepoint/continuation maps, ABI/CPU/security profile, and benchmark build flags.

The primary B→C experiment also uses one identical finalized physical-form plan. Each fragment has a canonical selected instruction sequence and known length before layout. Variant B emits that sequence through the ordinary encoder. Variant C may emit the same sequence through a validated stencil. After normalizing relocation values and placement addresses, the emitted instruction bytes and metadata must agree.

A stencil that changes instruction selection, fuses operations, changes fragment length, or otherwise changes the physical-form plan is a separate superinstruction/peephole experiment. Its execution and code-quality effects are not attributed to copy-and-patch emission.

Report A→B generic-native benefit and B→C stencil-emission benefit separately. Break compile latency into admission, legalization, target lowering, allocation, physical-form selection, layout, ordinary emission, stencil lookup/copy/patch, publication, and metadata/security registration. Include build-time stencil extraction, embedded stencil bytes, relocations, clean-build cost, and binary footprint.


### Native compilation resource envelope

Native compilation is optional, bounded, and fallible:

```text
compile(program, target_profile, compilation_limits)
    -> Result<NativeArtifact, NativeCompileError>
```

`CompilationLimits` bounds total functions, semantic and target-MIR operations, CFG edges, allocator work, spill and copy records, layout/relaxation iterations, veneers, literal pools, relocations, code bytes, read-only data bytes, metadata bytes, peak scratch bytes, retained compiler bytes, and total compiler work units.

Every cumulative size and allocation is checked before reservation. In-process compilers use fallible allocation. A compiler that cannot meet those guarantees executes in an isolated worker with enforced memory and CPU limits. Limit, allocation, timeout, unsupported-form, or internal-validation failure publishes no artifact and leaves the admitted interpreter lane available. No module feature or consumer request can make successful native compilation a semantic requirement.

Automatic promotion is additionally governed by bounded process-wide queue and code-cache policy so many admitted programs cannot force unbounded concurrent compilation or retained code.

A heavier optimizing tier, including a possible Cranelift path, is optional and subordinate to Weavy’s single execution authority. Gate 0 may perform a non-shipping feasibility comparison on representative medium/large islands, including achieved execution speedup and end-to-end saved work as well as compile latency, peak/retained compiler memory, dependency/binary footprint, code memory, and partition impact. A valid result is no heavy tier. A later production experiment on already migrated programs is the only shipping/promotion decision. Promotion requires measured net execution time saved over expected remaining invocations and stable hotness after all compile, memory, and code-cache costs; otherwise execution remains in the interpreter or selected compact baseline. Consumers never select or bypass execution lanes, and no tier may change semantics, fault/resource transcripts, attribution, or no-exec fallback.

Heavy-tier studies additionally report compile-queue contention, artifact-cache hit/eviction/invalidation, repartition churn, p50/p95/p99 latency, instruction/data-cache effects, promotion-estimate error, and retained state after cache quiescence.

Before publication, the backend computes exact physical frame requirements, including spills, callee saves, outgoing arguments, ABI shadow space, alignment, trampolines, and suspension scratch.

Native execution must never rely on reaching an ambient host-stack guard as ordinary resource enforcement. A target profile either:

1. uses an explicit checked task-owned or segmented native stack; or
2. establishes before native entry a `NativeStackEnvelope` proven sufficient for every physical frame sequence permitted by the invocation’s semantic call-depth limit.

If that envelope cannot be established, the artifact is ineligible for the invocation and execution remains interpreted. Every semantic call-depth check occurs at the same originating `InstId` as in the interpreter. A guard-page fault, stack-overflow signal, or language exception is never the normal representation of VM resource exhaustion. Tail-call treatment is specified explicitly by the semantic call contract.

The common backend order is:

```text
Legal SSA
  -> target virtual-register MIR
  -> target-required edge splitting
  -> explicit parallel-copy bundles
  -> register allocation
  -> post-allocation copy resolution / SSA destruction
  -> frame, fault, safepoint, ownership-root, and continuation maps
  -> canonical physical-form selection and stencil eligibility
  -> fixed-point layout and relaxation using known fragment lengths
  -> final emission of each fragment through:
       ordinary complete encoder, or
       manifest-equivalent validated stencil
  -> internal artifact validation
  -> all-or-nothing W^X publication and metadata/security registration
  -> entrypoint withdrawal and quiescent retirement
```

The complete ordinary encoder remains mandatory for every physical form.

The allocator is replaceable. Linear scan is a plausible baseline implementation, not an architectural promise. Every stencil manifest specifies operands, physical locations, defs, uses, fixed registers, flags, clobbers, patches, branch behavior, and security requirements. Missing or rejected stencils fall back to ordinary encoding.

### Native trust

The native backend, encoder, publisher, and embedded stencil assets are in the TCB. `validate NativeArtifact` validates internal output from that trusted pipeline: profile consistency, patch bounds, emitted instruction forms, defs/uses/clobbers, branch reachability, frame maps, unwind records, and publication metadata. It is not a proof that arbitrary machine code obeys VM rules.

Version 1 never executes machine-code bytes from an untrusted persistent cache. A cache may provide semantic modules, profiles, or discardable hints, but native code is regenerated. A future persisted native cache requires an explicit trust domain, such as machine-local authenticated artifacts tied to exact backend, stencil, target, and binding identities. Authentication failure is a cache miss.

Embedded stencil assets are immutable trusted build products and are checked against manifests in build/backend oracles.

### x86-64 obligations

Target MIR/profile models:

- explicit condition values or verified compare/consumer bundles rather than freely live RFLAGS;
- two-address operations;
- fixed `RCX` shift forms where selected;
- `RAX/RDX` division/multiplication constraints;
- byte-register restrictions;
- GPR/vector classes;
- caller/callee saves;
- ModRM/SIB legality and immediate encodability;
- call clobbers and stack maps;
- distinct System V and Win64 ABI obligations;
- fixed-point short/near/far branch relaxation;
- CET/IBT and Windows CFG as profile requirements.

### AArch64 obligations

Target MIR/profile models:

- GPR and SIMD/FP classes;
- explicit NZCV condition values or verified fused bundles;
- immediate and shifted-immediate constraints;
- pair/multi-register constraints;
- literal materialization;
- call-clobber behavior;
- platform reservations including veneer scratch (`x16/x17`) and platform-dependent `x18`;
- distinct AAPCS64, Darwin, Windows ARM64, and arm64e profiles;
- fixed-point branch/call relaxation with veneers/islands and declared scratch/flag clobbers;
- BTI, PAC, unwind/debug, instruction-cache invalidation, and executable-memory policy.

### Safepoints, publication, and retirement

Non-suspending safepoints carry precise ownership-root and live-location maps. A taken suspension slow edge materializes only the admitted continuation schema.

One owning `NativeArtifact` controls:

- executable code;
- veneers, trampolines, and literal pools;
- jump tables and read-only constant data;
- writable side streams, counters, or patch cells;
- frame, fault, safepoint, continuation, unwind, debug, and security metadata;
- every code, read-only, and writable mapping used by the artifact;
- strong references to every `VerifiedProgram`, immutable `ImageLease`, static binding, and `BindingLease` whose bytes, tables, implementations, or generations the artifact may access.

`VerifiedProgram` owns its image or holds an `ImageLease` guaranteeing stable address, length, and byte-for-byte immutability for the complete admitted lifetime. A read-only mapping of an externally mutable file is not sufficient; the file must be sealed, content-store immutable, privately copied, or otherwise protected from mutation.

A callable entrypoint is exposed through an `EntrypointHandle` or equivalent call gate. Every call acquires an artifact execution reference before loading or invoking the code address. An untracked raw address may be borrowed only while an `EntrypointLease` pins the artifact; a freely copyable raw pointer may not outlive that lease.

Publication is all-or-nothing:

1. stage, patch, and validate every code and associated-data mapping while no entrypoint is visible;
2. finalize all frame, safepoint, continuation, unwind, debug, and security metadata;
3. apply platform W^X and read-only protections;
4. complete applicable cache synchronization, CFG registration, landing-form checks, PAC policy, unwind registration, and debug registration;
5. atomically expose entrypoint handles.

Failure before exposure reverses every completed registration and releases every staged mapping and reference.

Retirement atomically withdraws entrypoint handles, preventing new execution-reference acquisition; waits for executing references, unwind users, entrypoint leases, image leases, and binding leases to quiesce; removes applicable unwind/debug/CFG registrations; releases all executable, read-only, and writable mappings; and finally drops retained program, image, and binding references.

No mapping or reference may be released while generated code can execute, unwind, inspect a literal/table, or call a direct-linked binding through it.

Interpreter-only operation is valid on iOS, WASM, and no-exec environments. Platform publishers implement MAP_JIT/W^X, dual mapping or RW→RX, instruction-cache synchronization, applicable CFG/unwind/debug registration, emitted CET/IBT/BTI landing forms, PAC/arm64e pointer policy, rollback, and retirement under these common ordering invariants.

## 17. Consumer semantic profiles

### Vix

Vix is the primary consumer and defines the base machine’s first relevance gate. Graph VIR describes typed immutable demand wiring; partitioned VIR groups compiler-selected eager pure islands; Weavy executes only island interiors. The partition is nonsemantic and must be free to respond to interpreter/JIT compilation economics.

The Vix base feature set contains typed pure scalar/aggregate computation, local control and calls, immutable/persistent values, path-sensitive wire yields, registered-primitive yields, the two safepoint classes, typed faults, causal attribution, deterministic resources, and interpreter/native transcript parity. Persistent Store/value handles are data capabilities, not authority to perform memo or publication work inside Weavy.

**Gate 1A — Vix execution-authority cut.** At execution time, record the current consecutive ratchet score, exact production `AST -> Graph VIR -> partitioned VIR -> Weavy -> runtime` entrypoint, and complete current caller closure. Migrate that production entrypoint and its currently supported authoritative rungs/corpus programs to admitted programs and the reference interpreter. Run the complete current consecutive prefix and applicable real-program corpus. Delete the old `task::Op` interpreter route for every recorded migrated caller in the same merge. Gate 1A does not invent a fixture and does not require later language surfaces to be implemented prematurely.

**Vix certification sequence.** Continue climbing the checked-in ratchet consecutively through the same new interpreter/runtime lane. When existing authoritative rungs first make lazy wires, registered primitive suspension, Rust-async completion, task kill/replay, or ticket joining load-bearing, implement and prove each mechanism there. No additional evaluator, compatibility route, or alternate runtime lane may be introduced. A named existing higher rung or corpus root may be exercised early only under the ratchet’s priority-track rules: it retains its original source and prerequisites and does not increase the canonical score above a red predecessor.

The complete wire/primitive/replay transcript and PC-to-demand attribution oracle is cumulative across those authoritative rungs. It includes final typed values/faults; demanded versus undemanded wire sequence; primitive request and completion; semantic resource events; edge safepoints and interior pollpoints; suspension/continuation; task kill/replay/join; ticket not restarted by task death; and the causal chain `interpreter PC -> Weavy InstId -> VIR node -> island -> Vix source span -> demand chain`, with typed task and primitive-effect links. The later compact-native cut proves parity for the already migrated production lane and deletes only route-specific native machinery.

Vix/Vixen retains `Location`/`Recipe`/`Content` identities, memo/read-set validation, demand and ticket lifecycle, receipts, capability discovery, Store/publication authority, placement, scheduler policy, and task replay meaning. Owner decisions still required include callback re-entry beyond the v1 prohibition and persistent-handle/store-generation rules.

### Phon/Vox

Phon/Vox is the second anchor consumer because it exercises the opposite end of Weavy’s intended envelope: tiny schema-derived encode/decode programs that need immediate interpretation, low-latency native specialization where available, per-shape caching, and a small dependency/residency footprint.

The frozen Phon/Vox anchor manifest is the checked-in wire-shape regression set currently exercised by `vox-phon`: `Message`, `MessagePayload`, `RequestCall`, `RequestMessage`, `SchemaMessage`, `Payload`, and the large representative `DodecaParseResult`, plus deliberately selected small/median/large schema-derived programs covering encode and decode, borrowed and owned results, enums/options/collections, nested and `CallBlock` behavior, and native-supported/fallback cases.

The manifest does not by itself claim that every root is linked through the production `vox-phon` dependency graph or has measured production frequency. `DodecaParseResult` is classified as a scale representative unless production evidence shows otherwise. Record semantic-operation and byte-size distributions; report every root independently. Observed-frequency weighting is reported only for roots with traced production call sites or telemetry.

The first cut runs this manifest through canonical admitted modules and the reference interpreter, then through the Gate-0-selected compact-baseline authority on supported profiles. Copy-and-patch is exercised when Gate 0 selects stencil substitution for that profile. Unsupported profiles use the same public execution authority and fall back to interpretation without a consumer-owned mode switch or JIT feature.

Oracle: exact encoded bytes and decoded values against the current codec path; typed malformed-input faults; per-shape construction/admission and cache behavior; and fresh-process measurements for interpreter-only core+Phon, ordinary compact native, compact native with copy-and-patch, full-feature Weavy, and any heavy-tier build. Report stripped application text/rodata/data deltas, transitive dependency closure, load/initialization latency, baseline RSS before program creation, peak construction/admission/compile RSS, retained RSS after first result and cache quiescence, code-cache bytes, first uncached interpreted/native result, compile latency, break-even count, and warmed throughput. Same-cut deletion removes the covered duplicate Phon/Vox execution route. A “competitive with Serde” claim requires a separate direct controlled benchmark.

### Snark

Portable code plus logical tables owns lexer, LR/GLR control, recovery, tree construction, and incremental reuse. The only Snark-specific host boundary is the external scanner.

Required facilities include portable classes/transitions/acceptance tables, deterministic worklists, bounded vectors/arenas/COW branch state, stacks/queues, journals/ropes, portable automata, relation contracts, progress certificates, and exact category quotas.

Parser actions, grammar symbols, LR action semantics, GLR scheduling, recovery, and tree construction are not opcodes or helpers. Ordinary SSA interprets typed action data.

First cut: deterministic LR parsing and tree construction for a production grammar, including portable literal/regex automata and the Dibs-scale grammar. Delete that path’s `RuntimeParserFacts`, `SnarkIntrinsic`, Rust parser loop, and runtime regex route in the same cut. GLR/recovery/incremental paths remain visibly legacy until their cuts.

Oracle: complete parse trees, spans, errors, recovery decisions where included, scanner calls, table/automaton event counts, resource faults, module/code/table bytes, admission CPU/RSS, cold start, and throughput. The 117k-state grammar is a required gate.

Owner decisions still required: formal parser-level zero-width progress certificate, deterministic tie-break rules, exact quota categories, and scanner-snapshot cancellation policy.

### Fable

Fable is the first **broad semantic-coverage extension** after the load-bearing Vix and Phon/Vox envelope is proven. It is not the first proof of Weavy’s relevance.

First cut: a specific read-only predicate/query public family covering exact required scalar widths, short-circuit CFG and block parameters, strings, direct recursion, nominal products/enums, read-only schema-relative Facet projection, and precise faults. Mutation, closures, broad collections, and all evaluator operations are not prerequisites.

Second cut: aggregate construction, mutable adapter operations, stable imports, partial-write cleanup, and typed OOM.

Compiler inputs such as evaluator plans, branch/query plans, and host-native aggregate layouts do not become imports.

Oracle: existing behavior differential, exact values/errors/source spans, adapter transcript, resource events, integer boundaries, NaNs, Unicode whitespace, and aggregate cleanup. Delete the covered Fable intrinsic/evaluator/native route with each public family.

Owner decisions still required: fixed `usize/isize` width, exact Unicode data version, exhaustive checked arithmetic/ordered-NaN rules, and partial-write policy.

### facet-json

The external boundaries are invocation-scoped input and schema-bound Facet builder/value adapters. JSON parsing, field matching, schema dispatch, replay, and control remain portable code.

First cut: one owned `from_slice`/`from_str` family covering scalars, products, options, nested arrays/sequences, and errors. Second cut: borrowed-result APIs proving input/result lifetimes, raw spans, and escaped ownership.

Required semantics include cursor/checkpoint/rollback, exact JSON/JSONC and suffix policy, iterative strict skip, lexical scanning, ordered probes, root completion, duplicate/unknown behavior, enum replay, zero-copy spans, owned escaped strings, builder cleanup, and fallible allocation.

Oracle: exact values, cursor/error offsets, policy behavior, allocations, drop order, adapter calls, hostile truncation, and memory-safety tooling around partial initialization. Delete covered `JsonOp`, interpreter, and root-host-call paths.

Owner decisions still required: JSONC/trailing-comma matrix, duplicate/unknown policy, exact probe rollback, and OOM surface.

### facet-hash/equality

Portable SSA owns schema traversal, recursion, short-circuiting, and policy. Live values use a closed `FacetValueView`; generic `Hasher` remains an ordered typed-effect capability.

First cut: scalars, products, enums, option/result, and acyclic recursive declared shapes. Cyclic runtime object graphs, unordered native containers, and pointer-heavy policies remain excluded until semantics are frozen.

Hash oracle uses a custom hasher recording typed method identity and order. Equality records projection counts to prove global short-circuit. Preserve bitwise float equality and checked hash/equality congruence.

Delete covered `HashIntrinsic`, `EqualityPlan`, interpreter, and consumer-native routes.

Owner decisions still required: runtime object-cycle behavior, pointer identity policy, set/map iteration and user-code effects, and generic-hasher panic policy. Recursive type support does not decide runtime object-cycle semantics.

## 18. Experimental gates and delivery ledger

### Decisions frozen before implementation

Freeze in the executable specification:

- Vix islands as the execution unit and the complete Vix/Vixen authority firewall;
- exact PHON v1 bootstrap/directory/schema-closure contract and PHON-schema versus Weavy-type identity;
- official sealed feature catalog, build-time profile composition, dependency direction, and closure manifests;
- canonical semantic serialization, `ExecutableId`, `ImageId`, canonical `InstId`, `AttributionId`, and cache binding;
- stable nominal and recursive-group key schemes, descriptor digests, and collision behavior;
- SSA/block/value/terminator form and simultaneous block arguments;
- ownership use kinds, borrows, builders, cleanup-only adapter actions, ordered cleanup obligations, task abandonment, secondary cleanup failure, and abrupt faults;
- fault taxonomy, exact sites, evaluation order, and partial effects;
- sealed opcode effects and call-graph derivation;
- semantic resource-event transcript;
- logical schemas, canonical row order, closed `RelContract` predicates, and the anti-engine firewall;
- helper/profile-operation firewall and helper versioning;
- static/invocation bindings, affinity, generations, revocation, and leases;
- suspension protocols, suspend-safe classes, continuation schemas, abandonment plans, armed-pollpoint rules, and the exact primitive submission handshake;
- admission complexity, exact `ExecutableId` finalization point, and mandatory cache-independent `BaseExecutionPlan`;
- reference-interpreter semantics and the common `LegalProgram` boundary;
- bounded fallible native compilation, native stack-envelope safety, trusted generation, complete ordinary encoding, publication, and retirement invariants;
- clean per-entry-point cutover, corrected Vix execution-authority gate, and same-cut deletion.

### Decisions made by corpus evidence

- 16-bit versus 32-bit versus byte/varint canonical **program-section instruction encoding inside the frozen PHON-backed `.weavy` container**, using the full controlled corpus below; PHON remains the module directory, schema, and typed-constant substrate;

- Vix island partition/cost-model policy, using real tiny/medium/large island distributions and compilation break-even data while keeping partitioning nonsemantic;
- compact-baseline policy, using the controlled A/B/C comparison of streaming interpretation, complete ordinary encoding without stencils, and the same backend with measured copy-and-patch substitutions; copy-and-patch is the preferred hypothesis, not a predetermined winner;
- heavy-tier feasibility at Gate 0 and any later shipping/promotion policy as separate decisions, using achieved execution speedup, end-to-end net saved work, compile latency, hotness stability, dependency/binary footprint, peak and retained compiler state, code memory, and impact on island formation; “no heavy tier” is a valid result;
- code/table/hybrid policy, using Snark, automata, and large-switch code/module/RSS budgets;
- streaming versus optional predecode retention policy, using cold/warm throughput and metadata size;
- primitive versus stdlib-bytecode versus bounded-helper placement, using representative consumer benchmarks and the helper firewall before freezing the selected feature version;
- physical PHON table profiles, using mmap behavior, validation cost, bounded compression/expansion, and access throughput.

### Backend-local and replaceable decisions

The following are not durable semantic contracts and remain replaceable behind the frozen boundaries:

- target-MIR spelling;
- instruction-selection algorithm;
- register allocator;
- interpreter slot-reuse algorithm;
- optional move-schedule optimization;
- frame-layout and branch-layout heuristics;
- macro-assembler internals;
- exact measured stencil set;
- per-platform W^X publication technique;
- debug/unwind emitter implementation;
- cache replacement policy;
- promotion thresholds and profiling implementation.

The controlled encoding study represents exactly the same semantic functions with the same opcode vocabulary, type aliases, block directory, out-of-line switch/automaton payloads, logical tables, verifier obligations, source metadata, and derived-cache allowances. No candidate receives a richer mandatory or optional predecode than the others. The physical variants are:

1. 16-bit static short/wide forms;
2. 32-bit fixed wordcode with canonical AUX words;
3. byte opcode plus bounded canonical operands/varints.

The first acceptance corpus has two load-bearing anchor slices before broader semantic and scale coverage:

1. **Authoritative Vix production lane:** Gate 1A migrates the current recorded production caller closure and complete current consecutive ratchet prefix to the admitted interpreter. The same lane is then certified consecutively as existing authoritative rungs introduce local pure control/aggregates, lazy and unconsumed wires, registered primitives, Rust-async park/resume, kill/replay/ticket join, edge safepoints, interior pollpoints, and exact `interpreter/native PC -> Weavy InstId -> VIR node -> island -> Vix source span -> demand chain` attribution with typed task/effect links.
2. **Phon/Vox codec manifest:** the checked-in wire-shape regression roots `Message`, `MessagePayload`, `RequestCall`, `RequestMessage`, `SchemaMessage`, `Payload`, and scale representative `DodecaParseResult`, plus frozen small/median/large programs spanning encode/decode, borrowed/owned, enums/options/collections, nested/`CallBlock`, and native-supported/fallback cases. Compare interpreter, ordinary compact encoder, copy-and-patch-enabled compact encoder, full-feature, and any heavy-tier closure in fresh processes; report every shape independently and frequency-weight only traced production roots. Claims against Serde require a separate direct benchmark.

The full encoding corpus additionally includes:

- Vix tiny, medium, and large pure/continuation-heavy islands, indirect calls, awaited inputs, primitive yields, and pollpoint densities;
- Fable predicates, recursion, strings, aggregates, and imports;
- facet-hash/equality scalar, aggregate, recursive-shape, and adapter-heavy traversals;
- facet-json scalar, product, nested collection, ordered-probe, and borrowed-result programs;
- Snark small LR, production grammars, GLR/recovery examples, and full Dibs-scale grammar, each with code-heavy, table-heavy, and candidate-hybrid outputs.

Report semantic code bytes, directory bytes, total module bytes, table bytes, admission wall/CPU, peak RSS, allocations, boundary metadata, time to first interpreted result, time to first native result, compilation time per semantic operation and per byte of program, break-even invocation count, generated code size, resident compiler/JIT data, application binary and dependency footprint, clean-build cost, warm streaming and end-to-end throughput, random block seek, predecode build/size/throughput, producer encode time, fuzz throughput, malformed/nonminimal rejection, source-map cost, pollpoint cost, suspension-site cost, and impact on Vix partition choices. Run on x86-64 and AArch64, including interpreter-only/no-exec configuration and cold/warm filesystem/cache conditions. Report island-size distributions, medians, and tails.

The selected encoding must have fixed maximum instruction length, canonical shortest forms, phase-0 impossible-length detection, direct block-directory access, no mandatory whole-program decode, bounded verifier state, and no whole-module owned table decompression requirement. Selection is Pareto-based across size, admission, memory, execution, producer, and validation complexity—not “smallest code wins.” A compact transport format whose acceptable execution requires predecode still has one canonical semantic physical encoding; predecode remains a derived cache.

### Production sequence

0. Corrected executable specification plus controlled Vix/Phon-first and full encoding/backend corpus. Gate 0 freezes the PHON-backed container while comparing only program-section instruction encodings, performs the answer-neutral interpreter/ordinary-compact/copy-and-patch comparison, and runs a non-shipping heavy-tier feasibility study; it may select interpreter-only or no heavy tier. No public cutover and no instruction-encoding selection before evidence.
1. Gate 1A migrates the recorded current Vix production runtime caller closure and complete current consecutive ratchet prefix to admitted programs and the reference interpreter; delete that caller closure's old `task::Op` interpreter route in the same cut.
2. Continue the Vix certification sequence consecutively on that single lane as authoritative rungs introduce lazy wires, primitives, async suspension, kill/replay/join, attribution, and safepoints. Priority-track evidence does not advance the canonical score past a red predecessor.
3. Phon/Vox frozen codec manifest through the same admitted interpreter and Gate-0-selected compact-baseline authority; prove feature-isolated tiny-program latency, footprint, cache, and no-exec behavior; delete its covered duplicate execution route.
4. Vix selected compact-baseline native slice on the already migrated production lane, gated by interpreter/native semantic and attribution transcripts plus compilation-economics measurements; delete route-specific native machinery.
5. Fable read-only predicate/query family as the first broad semantic extension; delete covered evaluator/intrinsic/native route.
6. Fable construction/mutation/import family; cleanup/effect/OOM oracle; delete covered old route.
7. Optional heavier-tier production experiment on already migrated Vix/Fable programs; ship and promote only when measured net saved work is positive under the specified cost model. “No heavy tier” remains a valid result. It is not a prerequisite for correctness or compact-native delivery.
8. facet-hash/equality acyclic structural family; typed Hasher and short-circuit oracle; delete covered paths.
9. facet-json owned-result family; cursor/builder/policy/allocation oracle; delete covered paths.
10. facet-json borrowed-result family; lifetime/span oracle; delete covered paths.
11. Snark deterministic parser and automata with Dibs-scale gate; delete deterministic runtime facts/intrinsic/parser/regex routes.
12. Snark GLR/recovery/tree/incremental/external scanner; quota/scanner/transcript oracle; delete remaining engines and side APIs.
13. Additional x86-64/AArch64 OS/ABI profiles reuse the target-MIR contracts and gain publication/unwind/security oracles.
14. Consolidation: repository-wide audit and deletion of `WeavyOp`, `task::Op`, duplicate verifiers/interpreters/JITs, aliases, and obsolete encodings.

No gate is a free-floating foundation. Every production cut after Gate 0 migrates named public entry points, proves them through a production oracle, and deletes the replaced path. Snark and Facet feature sets extend the admitted kernel without delaying or contaminating the Vix/Phon execution rail.

## Approval decision

This professionally reviewed candidate closes the original eleven bytecode blockers, the Vix-first review's thirteen corrections, and the final five professional findings:

1. `ExecutableId` becomes available only after complete canonical semantic decoding and claim comparison;
2. verifier-owned cleanup obligations and `AbandonPlan`s govern task kill, replay, concurrent completion, and armed pollpoints;
3. native compilation is bounded and fallible, and every native invocation has a proven stack envelope or remains interpreted;
4. ordinary encoding and copy-and-patch share one finalized physical-form plan, with stencil emission before artifact validation;
5. `NativeArtifact` owns every mapping, program/image/binding reference, and gated entrypoint through quiescent retirement.

The previously closed contracts remain intact: sealed build-time profiles, exact PHON v1 wire authority, recursive type digests, canonical instruction attribution, relation and helper firewalls, exact scheduler acceptance, cache-independent interpretation, Vix gate separation, accurate consumer corpora, nonoverlapping freeze classification, and complete x86-64/AArch64 platform lifecycle obligations.

Owner approval of architecture v6 authorizes writing the normative repository specification and planning Gate 0. It does not authorize choosing a physical instruction encoding or beginning consumer implementation before the corpus experiment.
