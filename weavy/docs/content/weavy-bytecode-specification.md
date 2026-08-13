# Weavy bytecode normative specification

## Status and authority

This document is the normative repository specification authorized by the owner-approved [Weavy bytecode architecture v6](weavy-bytecode-architecture.md). Authority is hierarchical: architecture v6 authorizes constraints, rationale, and scope; this specification controls cross-cutting wire, semantic, admission, interpreter, legalization, lifecycle, and identity contracts; companion opcode, relation, legal-program, canonical-semantic-schema, runtime-profile, and producer-lowering catalogs are authoritative only for the descriptor slice explicitly delegated to them; the Gate 0 plan is procedural; generated registries, manifests, vectors, reports, and code are mechanically checked projections. A conflict or missing delegated descriptor fails closed and requires owner review.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, and **MAY** are interpreted as RFC 2119 requirements.

This specification freezes the semantic machine, admission contract, PHON-backed container boundary, reference-interpreter contract, target-independent legalization boundary, native artifact lifecycle, and frontend authority boundaries. It does **not** select the physical program-section instruction encoding, a code/table representation policy, a predecode policy, a native promotion threshold, a concrete register allocator, or a shipping heavy compiler tier. Those decisions require Gate 0 evidence.

## 1. Conformance surfaces

A conforming implementation provides these logically separate surfaces:

```text
encode(semantic_module, physical_profile) -> .weavy image
inspect(image) -> non-authoritative structural report
admit(image, policy, AdmissionLimits) -> Arc<VerifiedProgram>
bind(VerifiedProgram, StaticBindings) -> BoundProgram
start(BoundProgram, InvocationBindings, InvocationLimits) -> Task
interpret(Task) -> completion | typed yield | TaskFault
legalize(VerifiedProgram, LegalizationProfile) -> LegalizedProgramHandle
compile(LegalizedProgramHandle, TargetProfile, CompilationLimits) -> UnboundNativeArtifact
link(UnboundNativeArtifact, BoundProgram) -> NativeArtifact
```

The exact Rust ownership spelling MAY differ. The following separations are mandatory:

1. inspection MUST NOT confer execution authority;
2. decoding MUST NOT confer admission authority;
3. `VerifiedProgram` MUST be opaque outside trusted admission/runtime code;
4. static binding and invocation binding MUST be distinct;
5. the interpreter MUST execute every admitted semantic feature supported by the runtime profile without native code;
6. native compilation MUST be optional, bounded, fallible, and semantics-preserving;
7. a migrated public consumer entry point MUST NOT fall back to its replaced evaluator, parser, codec engine, or native route.

## 2. Durable PHON-backed container v1

### 2.1 Fixed bootstrap header

The v1 bootstrap header is exactly 64 bytes. All multi-byte integers are little-endian. No field contains a process pointer, `usize`, Rust enum representation, ABI layout, or native relocation.

| Bytes | Type | Name | Required value or meaning |
|---:|---|---|---|
| `0..8` | `[u8; 8]` | magic | `WEAVY\0\0\0` |
| `8..10` | `u16` | format major | `1` |
| `10..12` | `u16` | format minor | exactly `0`; all other minors are unsupported until a separately approved byte-exact compatibility contract exists |
| `12` | `u8` | byte-order marker | exactly `1`, meaning little-endian |
| `13..16` | bytes | reserved | exactly zero; assigning any byte requires a separate module-format decision |
| `16..24` | `u64` | total file length | MUST equal the complete received image length |
| `24..32` | `u64` | directory offset | `64` in v1 |
| `32..40` | `u64` | directory encoded length | exact PHON compact directory byte length |
| `40..44` | `u32` | directory alignment | `8` in v1 |
| `44..48` | `u32` | directory section kind | `1` |
| `48..64` | `[u8; 16]` | `PayloadIntegrityTag` | first 16 bytes of BLAKE3 over all bytes from offset 64 through end of file |

`PayloadIntegrityTag` is the first 16 bytes of unkeyed BLAKE3 over the exact bytes at offsets `64..file_len`; it authenticates no producer and is not a trust signature. It is not `ExecutableId` or `ImageId`. The current implementation names this field `executable_identity`; that symbol is legacy terminology only.

`ImageId` is the full 32-byte unkeyed BLAKE3 digest over the exact complete received file bytes `0..file_len`, with no prefix, exclusion, or zeroed field. It need not be embedded. `ExecutableId` is the distinct domain-separated 32-byte semantic identity below and becomes available only after canonical semantic decoding.

### 2.2 PHON directory schema

The directory is PHON compact data using the content-derived schema closure for:

```text
struct WeavySection {
    name: String,
    kind: u32,
    offset: u64,
    encoded_len: u64,
    decoded_len: u64,
    alignment: u32,
    schema_id: u64,
    flags: u32,
    profile: u8,
    count: u32,
    stride: u32,
}

list<WeavySection>
```

The frozen directory physical schema identities are `WeavySection = 0x27e882292860ab54` and `list<WeavySection> = 0xcf54d7568f593290`. They are derived by PHON/taxon revision `fe15715e345b72d07543fae4135f8b56cff92433`: BLAKE3 over its canonical dependency-aware structural walk, truncated to the first eight hash bytes and interpreted as little-endian `u64`. The structural walk uses little-endian integers; UTF-8 strings are `u32` byte length plus bytes; booleans are one byte; schema-kind, field, reference, generic-argument, SCC inline/back-reference, and external-reference tokens and order are exactly the `taxon::resolve_ids` algorithm at that revision. A change to either identity, the algorithm, or its token grammar is a module-format decision.

Every field is required. Profile values are:

- `0`: no table storage profile;
- `1`: PHON compact;
- `2`: PHON aligned;
- `3`: PHON dense-aligned.

All other profile values are malformed in format 1.0.

Section flag bit 0 is `REQUIRED`. All undefined flag bits MUST be zero in format 1.0. Unknown optional sections MAY be ignored. An unknown required section MUST reject admission.

### 2.3 Section-kind namespace

The v1 namespace is:

| Kind | Cardinality | Name | Required alignment | Schema/profile meaning |
|---:|---:|---|---:|---|
| `1` | bootstrap-only | directory | 8 | PHON compact directory schema above |
| `2` | exactly one | manifest | 8 | legacy custom payload, directory `schema_id = 1`, profile 0, count/stride 0 |
| `3` | exactly one | schema closure | 8 | legacy length-delimited PHON self-describing schema bytes, directory `schema_id = 1`, profile 0, count/stride 0 |
| `4` | exactly one | program | 8 | legacy frame-offset payload, `schema_id = 0x0bcb92f43d1a308a`, profile 0, count/stride 0 |
| `5` | exactly one | constants | 8 | legacy custom directory, `schema_id = 0xd87cd9d93b41e5aa`, profile 0, count/stride 0 |
| `0x1000 + n` | zero or one for each dense `n` | constant range `n` | 8 compact, 64 aligned or dense-aligned | declared PHON root schema and profile 1..3 |

Kinds `6..0x0fff` are reserved for future format-level sections. Kind `0` and directory kind `1` are malformed as directory entries. Known singleton sections and ranges MUST carry only flag bit 0 (`REQUIRED`); undefined flag bits are malformed. Unknown optional kinds may occur only in `6..0x0fff` with flags zero and profile/count/stride rules defined by their optional section schema; unknown required kinds reject admission. Assigning a required kind, changing singleton cardinality, changing a required payload schema, changing the bootstrap/directory schema, or changing schema-closure framing/membership/order requires a module-format decision.

### 2.4 Legacy required payloads and future semantic profile

The current `weavy-phon` writer/reader dispatches kinds `2..5` through custom decoders and does not validate their directory `schema_id` values. This is legacy behavior, not evidence that arbitrary IDs are authoritative. A conforming hardened format-1.0 reader MUST require the exact legacy kind-to-ID/profile values above before invoking those custom decoders.

The approved typed-SSA machine needs content-derived PHON roots for `ModuleManifestV1`, `ConstantDirectoryV1`, and each candidate program schema. Replacing the legacy required payload schemas is an explicit module-format decision, not silent format 1.0 evolution. Gate 0 Task 1 produces byte-exact semantic roots; Task 2A records and obtains owner approval for the nonshipping experimental section profile before candidate images use them. The semantic data model below specifies required content but does not assign it to frozen 1.0 required-section wire.

`ModuleManifestV1` contains:

```text
display_name: option<string>                 // nonsemantic
semantic_version: { major: u16, minor: u16 }
required_opcode_features: list<FeatureRequirement>
required_helper_features: list<FeatureRequirement>
required_relation_features: list<FeatureRequirement>
required_capability_classes: list<FeatureRequirement>
required_policy_features: list<PolicyRequirement>
root_functions: list<FunctionKey>
producer: option<ProducerInfo>               // nonsemantic
```

`PolicyKeyV1` is a lowercase ASCII dotted string. `PolicyDescriptorV1` is `{ policy_key: PolicyKeyV1, major: u16, minor: u16, canonical_semantics: bytes }`; each `minor_digest = BLAKE3("weavy.policy.descriptor.v1\0" || CanonicalPhon(PolicyDescriptorV1))`. `PolicyRequirement` is `{ policy_key, major, min_minor, required_minor_digest }`. `PolicyVersion` is `{ policy_key, major, max_minor, compatible_minor_digests: list<bytes[32]> }`, where index `i` is the approved descriptor digest for minor `i`, the list length is `max_minor + 1`, and extending it may not change earlier entries. Compatibility requires equal key/major, `max_minor >= min_minor`, and `compatible_minor_digests[min_minor] == required_minor_digest`. Lists sort canonically and reject duplicate key/major with unequal histories. `FeatureRequirement` retains its stable ID/major/minor form. Root functions sort by exported identity; `ProducerInfo` is nonsemantic.

The future typed-semantic closure model contains every nonprimitive PHON schema reachable from the approved manifest, semantic constants, table views, and required inspectable nonsemantic sections. Its exact framing, membership, ordering, and root IDs are outputs of the required module-format decision; it MUST NOT be confused with the frozen 1.0 legacy schema bundle below.

`ConstantDirectoryV1` is a PHON list in dense `ConstantId` order. Each entry is exactly one of:

```text
InlineConstant {
    semantic_type: TypeRef,
    value_schema: SchemaId,
    canonical_value: bytes,
}

TableConstant {
    semantic_type: TypeRef,
    table_schema: TableSchemaId,
    range_id: u32,
}
```

Inline `canonical_value` is the canonical PHON encoding under `value_schema`; admission decodes and re-encodes it canonically before hashing. `range_id` identifies section kind `0x1000 + range_id`. Constant IDs and range IDs are dense, zero-based, and never process handles. A table section's directory entry supplies physical profile, count, stride, and PHON schema; its semantic `TableSchema`, canonical logical order, and `RelContract`s are declared in the semantic program/table directory and participate in `ExecutableId`.

The program section root is selected by Gate 0 from E16, E32, or EV. Every candidate carries the same versioned semantic directories for types, signatures, functions, blocks, tables, imports, capabilities, cleanup plans, continuation schemas, resource contracts, and nonsemantic attribution references. Only instruction and operand physical encoding differs.

For any future typed-semantic profile, the content-derived `SchemaId` of each root is produced and verified by the pinned PHON identity algorithm. Exact roots and closure bytes MUST be recorded in the module-format authority; implementations do not assign manual substitutes.

### 2.5 Directory invariants


Before any attacker-controlled count drives allocation, a bounded first pass MUST prove all of the following with checked arithmetic:

1. exact header version `1.0`, byte-order marker, reserved bytes, directory offset/alignment/kind, file length, and payload tag;
2. directory encoded length fits `AdmissionLimits::directory_bytes`; the PHON compact decoder is configured with maximum entries, total name bytes, field nesting, decoded bytes, and fallible reservation before materializing the array;
3. directory entry count fits its limit and every encoded entry has a byte-derived minimum-size bound before reservation;
4. every section alignment is a nonzero permitted power of two, every `offset + encoded_len` is representable and within the image, and no section overlaps header, directory, or another section;
5. entries are in strictly increasing `(offset, kind)` order; kinds `2..5` occur exactly once; ranges form dense `0x1000..0x1000 + range_count`; names are valid UTF-8, canonical for known kinds, and unique;
6. kind/flag/schema/profile/count/stride/decoded-length rules are exact: kinds `2..5` use the legacy values in section 2.3 with `decoded_len == encoded_len`; every range is REQUIRED, has nonzero stride, uses profile 1..3, compact alignment 8 or aligned/dense alignment 64; aligned count equals decoded root length; dense count and stride equal the parsed dense header; compact `count` and `stride` are producer-supplied logical metadata preserved exactly by the writer and reader but not validated against the compact PHON root in frozen 1.0, so consumers MUST treat them as untrusted until a semantic range declaration validates their meaning;
7. every gap/padding byte from the end of the directory through the last section is zero, and the directory bytes equal the pinned PHON canonical re-encoding of the decoded entries;
8. the schema section's `schema_count`, each encoded length, total schema bytes, schema nodes, closure edges, generic arguments, nesting, and aggregate decoded bytes fit explicit admission limits and remaining-byte-derived ceilings before any collection reservation;
9. `PayloadIntegrityTag` matches before any executable claim is trusted.

The existing writer places the directory immediately after the header and converges directory length and section offsets to a fixed point. Canonical format 1.0 is: directory at offset 64, zero padding, then sections in kind order at required alignments. Noncanonical-but-decodable images are rejected, not normalized. Two semantically equivalent images may differ only through a permitted physical range profile/content choice; then their full-file `ImageId`s differ.

### 2.6 Frozen schema bundle and PHON identity

The format-1.0 schema section is exactly:

```text
u32 schema_count
repeat schema_count times:
    u32 encoded_schema_len
    encoded_schema_bytes
```

Its membership/order is the current writer order: first the resolved `WeavySection` and directory-list schemas in that order, then each constant range's supplied schemas in range order and schema order, dropping later duplicate IDs by first occurrence. The legacy custom manifest/program/constants payload schemas are absent. Changing framing, membership, or ordering is a module-format decision.

Each `encoded_schema_bytes` uses the unversioned PHON self-describing tag-led encoding at revision `fe15715e345b72d07543fae4135f8b56cff92433`; there is no wire-visible schema-encoding version. Admission decodes every message, rejects trailing/noncanonical bytes by requiring byte-identical re-encoding, recomputes the complete dependency-aware IDs with `resolve_ids`, requires every embedded and referenced `SchemaId` to equal the recomputed value, and only then resolves roots/references and validates duplicates, generic arguments, allowed recursion, and section root IDs. All decoding and identity resolution obey the phase-0 count/work/scratch limits above.

A PHON `SchemaId` identifies a physical decoding schema. It MUST NOT be used as a Weavy `TypeKey`, `GroupKey`, adapter identity, binding authorization, or proof of semantic equivalence. A runtime profile mapping a PHON schema to a Weavy semantic type MUST declare a sealed injective mapping and admission or binding MUST validate the complete semantic descriptor.

### 2.7 Table storage profiles

Format 1.0 admits only compact, aligned, and dense-aligned physical table views represented by directory profile values `1..3`. A profile change among those views MUST NOT change logical rows, canonical row order, `ExecutableId`, opcode behavior, or frontend APIs. Columnar, packed, or any other profile without a format-1.0 directory discriminator may be studied only as a nonselectable prototype; selecting it requires an explicit module-format decision.

Aligned profiles MUST define explicit scalar widths, byte order, aggregate offsets, aggregate size/alignment, array stride/count, union tags/layouts, and relative references. They MUST prohibit native pointers, platform `usize`, Rust layout, and compiler-selected enum layout. Borrowed access is legal only after complete bounds, alignment, schema, count, stride, and relation validation and only while an immutable `ImageLease` keeps the bytes stable.

## 3. Versioned semantic feature closure

A module declares:

- semantic module version;
- required sealed opcode feature sets;
- required helper feature versions;
- required logical relation-feature versions;
- required import/capability classes and versions;
- optional nonsemantic sections and schemas;
- root entry points;
- optional non-authoritative `ClaimedExecutableId`.

There is no runtime registration of semantic opcodes, helpers, relation predicates, legalizers, target handlers, or consumer dialects. A deployable runtime is produced from a build-time official allowlist into one immutable `RuntimeProfileManifest`. Admission MUST reject a required feature absent from that manifest.

`weavy-core` owns canonical types, SSA/control/calls, faults, resources, base admission, `LegalProgramV1`, and the reference interpreter. Extension profiles MAY provide sealed semantic descriptors, admission rules, interpreter handlers, and total lowering into the common closed legal vocabulary. They MUST NOT accept or execute a frontend evaluator plan, parser state machine, codec plan, grammar, deserializer plan, or frontend tag vocabulary.

The required Gate 0 build-time closure schemas and consumer firewalls are defined by [Runtime profile manifests v1](weavy-runtime-profiles-v1.md). Shipping generated profile manifests, including exact feature/version values and digests, are approved canonical-semantic artifacts. The sole preselection exception is a nonshipping `Gate0StudyPolicy`: it accepts only domain-separated `StudyFeatureId`s, roots, handlers, and immutable study manifests named by owner-approved `helper-study-authority.styx` and the experimental module-format authority. Ordinary admission rejects every study identity; study identities never alias `FeatureIdV1`, never enter shipping caches/profiles/images, and are invalidated when Task 9A regenerates final authority.

Every semantic feature uses `FeatureIdV1 = BLAKE3("weavy.feature.v1\0" || namespace || "\0" || canonical_name)[0..16]`. `namespace` is one of `opcode`, `helper`, `relation`, or `capability`; `canonical_name` is lowercase ASCII dotted notation recorded in the normative catalog. Policy semantics use the separate canonical `policy_key` identity and version in `PolicyRequirement`; policies are not feature-ID aliases. Feature and policy major changes may alter semantics incompatibly; a minor change may add only behavior preserving earlier minor contracts. Duplicate feature IDs with unequal namespace/name descriptors reject generation and admission. Numeric opcode/variant tags are encoding-local aliases.

Admission derives the complete semantic feature and policy use sets from decoded instruction, helper, relation, capability, import, policy, and descriptor references, including the greatest required minor for each identity/major. The module declarations MUST equal those derived canonical sets exactly. Missing, understated, duplicate, and unused declarations reject admission; a producer may not request a speculative superset. The runtime manifest proves support for every exact major and at least the derived minimum minor.

`ProfileIdV1` is the full 32-byte unkeyed BLAKE3 digest of `bytes("weavy.profile.v1\0") || CanonicalPhon(ProfileSemanticProjectionV1)`. `ProfileSemanticProjectionV1` contains fields in exactly this order: (1) semantic module versions sorted by `(major,max_minor)`; (2) feature records sorted by `(namespace_tag,stable_id,major,max_minor)`, each carrying canonical name, semantic descriptor digest, interpreter handler ID, optional legalizer ID, and target handler IDs sorted by bytes; (3) `PolicyVersion` records sorted by `(policy_key,major,max_minor)`, each carrying its ordered `compatible_minor_digests` history; and (4) `dependency_allowlist_digest`. Namespace tags are opcode=0, helper=1, relation=2, capability=3. It excludes `profile_id`, presentation package lists, assets, symbol/section denylists, measurements, and `manifest_digest`. Canonical PHON uses the owner-approved content-derived root and exact enum/option/string/list encodings recorded with cross-language vectors. `ManifestDigest` is BLAKE3 of `bytes("weavy.profile.manifest.v1\0") || CanonicalPhon(manifest-with-profile_id-but-without-manifest_digest)` and covers all remaining nonsemantic build evidence.
## 4. Identities and canonical semantic serialization

### 4.1 `ExecutableId`

`ExecutableId` is the full 32-byte unkeyed BLAKE3 digest of the following byte stream:

```text
bytes("weavy.executable.v1\0")
canonical_semantic_module_v1
```

`canonical_semantic_module_v1` is canonical PHON compact serialization under the exact content-derived `CanonicalSemanticModuleV1` schema root recorded by the owner-approved canonical-semantic-schema authority. That authority assigns every enum/discriminator tag, integer/key width, option representation, field order, sort key, recursive-key scheme, descriptor schema, and PHON root; it is required before any Gate 0 corpus identity or candidate image is frozen. Markdown companion catalogs define semantics but MUST be mechanically checked against that schema authority. Unknown or unrepresented semantic fields reject admission; they are never hash-skipped.

The projection includes, in schema order: semantic module version and declaration-key scheme; exact required features; canonical types and callable signatures; import/capability declarations; logical constants and tables independent of physical profile; functions, blocks, instructions, terminators, cleanup, continuations, resource contracts, and roots. Stable nominal `FunctionKey`, `BlockKey`, `GroupKey`, and `MemberKey` widths and comparison order are assigned by the schema authority; dense aliases and producer declaration order never substitute for them. Admission rejects duplicate keys, noncanonical order, references to absent keys, and the same key paired with unequal descriptors.

The projection excludes physical offsets, padding, table profile, source/display/provenance metadata, `ClaimedExecutableId`, profiling counters, proof hints, predecode, and native code. Gate 0 Task 1 MUST publish identity vectors covering every descriptor family, every key form, each enum variant, empty/nonempty option and sequence forms, recursive types, constants/tables, suspension, and cleanup. Two independent implementations MUST reproduce every vector before candidate work begins.

Admission semantic phases 3 and 4 MAY stream canonical logical constant/table components into bounded temporary digests, but semantic phase 7 MUST serialize the final projection in schema order and MUST NOT expose `ExecutableId` earlier. A claim mismatch is `AdmissionError::ExecutableIdentityMismatch`.


Every semantic operation has exactly one canonical `InstId`, formed in version 1 from canonical function identity and canonical instruction ordinal after producer canonicalization. Admission rejects duplicate, missing, noncanonical, or out-of-range identities.

Legalization expansion preserves the originating `InstId`. Fusion carries an ordered nonempty origin map and identifies the exact origin for each possible fault, effect, resource event, and observation.

An `AttributionBundle` is nonsemantic immutable metadata. Its `AttributionId` covers `ExecutableId`, mapping schema, exact instruction-to-VIR/island/source mapping bytes, relevant source identities, and partition/source-map epochs. A missing bundle is a typed unavailable state; a mismatched bundle is rejected.

### 4.2 Cache keys

Semantic analyses key on `ExecutableId` plus verifier epoch. Image-relative facts additionally key on `ImageId`. Proof hints and predecode additionally key on their schema versions. Native artifacts additionally key on legalization/backend/encoder/stencil epochs, target/ABI/CPU/security/platform/instrumentation profile, and binding identities. Attribution-dependent native maps additionally key on `AttributionId`.

No identity-keyed semantic cache may be consulted before semantic phase 7 finalizes `ExecutableId`.

## 5. Canonical type, function, and SSA machine


The core scalar set is `unit`, canonical `i1`, signed and unsigned `i8/i16/i32/i64/i128`, `f32/f64`, Unicode scalar, and fixed-width opaque nominal IDs. There is no ambient host `usize/isize`.

The type graph supports nominal and structural products/sums, callable signatures, VM collections, persistent handles, scoped capabilities/borrows, ownership, affinity, effects, fallibility, and suspension classes.

Acyclic anonymous structural types are structurally interned. Every recursive declared strongly connected component has a versioned `TypeKeyScheme`, a `GroupKey`, member-local `MemberKey`s, and a cryptographic `TypeDigest` over its canonical semantic descriptor. Keys are stable nominal names, not unsupported claims of global collision freedom. Admission canonicalizes member order, recomputes the digest, rejects duplicate keys and unequal descriptors, and compares scheme, key, and digest at imports. Anonymous mutually recursive structural types are inadmissible in version 1.

A callable signature includes exact parameter/result types, operand ownership, borrow/capture behavior, effect upper bound, faults, allocation, affinity, suspension, and re-entry. Typed indirect calls admit only targets whose signature and effect row are subtypes of the declared contract.

### 5.2 Functions, blocks, and values

Each function declares exact parameters/results, entry block, effect contract, suspension permission, and resource contract. Each block declares ordered typed block parameters. Each instruction defines zero or more SSA values exactly once; every use is dominated by its definition.

The durable terminators are versioned sealed operations including unconditional branch, conditional branch, typed switch, return, typed fault exit, control-boundary direct/indirect call, invoke-style normal/fault successors, awaited-input suspension, and asynchronous-import suspension.

Critical edges are legal. Edge arguments are simultaneous. Interpreter move schedules and native edge splitting are derived facts and are not durable semantics.

Producer canonicalization MUST complete before serialization. Admission validates but MUST NOT invent semantic drops, cleanup, ownership transfers, faults, or control edges.

### 5.3 Ownership, borrows, builders, and cleanup

Values have sealed ownership classes: copyable, affine owned, shared immutable, immutable borrow, mutable borrow, scoped external capability, or persistent handle. Every use has an explicit use kind.

Admission performs bounded path-sensitive affine transfer over mutually exclusive CFG edges and proves exact consumption or transfer at every exit. Borrow origins, relationships, exclusivity, and ends are explicit and proven by bounded monotone dataflow. Borrowed results cannot outlive their invocation/input lease.

Normal-path borrow operations are explicit. `begin_borrow(origin, region, access)` creates an immutable or exclusive borrow under the declared region relationship and consumes no ownership of the origin. `end_borrow(region, borrow)` consumes that borrow and closes its ordinary-path obligation. Admission proves origin lifetime, exclusivity, nesting/relationship legality, edge transfers, and that every path ends or transfers each borrow exactly once. Neither operation suspends; both charge one semantic borrow event before state transition and lower to logical lifetime facts plus any target-required address materialization. `cleanup_end_borrow` is reserved for an admitted abrupt-path cleanup obligation and is not a substitute for ordinary `end_borrow`.

Aggregate construction uses affine builders with normative initialization state. Overwrite destroys the previous initialized value before replacement. Commit and abort consume the builder. Every possible cleanup obligation exists before serialization.

A `CleanupPlan` is an ordered sequence of distinct `CleanupObligationId`s. Admission proves no duplicate consumption. Its sealed actions are infallible VM-owned destruction/builder abort or a declared `CleanupAdapterCall` naming one exact bound cleanup-only adapter method and ownership obligation. Such a call is non-suspending, non-capturing, non-escaping, cannot re-enter the task or invoke an arbitrary import, and declares whether native user `Drop` code may run. Binding fails unless every possible external cleanup obligation has a compatible method. Cleanup is non-suspending and cancellation-suppressed. The primary fault is committed before cleanup. Each action executes at most once; after a caught cleanup panic, the failed obligation is terminal, later obligations still execute once, and the first cleanup-panic site is retained. The final recoverable result is `TaskFault::CleanupFailed { primary, first_cleanup_site }`. Aborting panic and process OOM remain outside the recoverable VM contract.

## 6. Faults, effects, resources, tables, and helpers

Malformed modules produce `AdmissionError`; incompatible bindings produce `BindingError` or `InvocationError`; execution produces canonical `TaskFault` or a consumer value. They MUST NOT collapse into host panics or strings.

Every semantic operation specifies operand/result types, evaluation order, overflow, floating/NaN behavior, dynamic checks and exact fault sites, ownership transitions, effects, partial writes, cancellation points, and semantic resource charges.

Resource accounting is over versioned semantic events identified by `InstId` or a sealed child event, never elapsed time or physical instruction count. Optimized execution preserves the first exhaustion site and the interpreter/native semantic-event transcript.

Logical tables declare `TableSchema`, canonical logical order, physical view descriptor, and closed `RelContract`s. Base operations perform typed loads, checked spans, packed extraction, bounded sparse/dense lookup, and typed callable retrieval. Tables never interpret frontend tags or invoke returned callables.

The complete version-1 relation vocabulary, operands, witnesses, work formulas, and auxiliary-memory formulas are defined by [Relation contracts v1](weavy-relation-contract-v1.md). Successful validation creates a private witness tied to the exact logical table declaration, physical view, `ExecutableId`, and when physical access matters `ImageId`. A producer cannot supply a trusted witness or an unchecked subset of rows.

The sealed core opcode and helper contracts used by Gate 0 are defined by [Opcode catalog v1](weavy-opcode-catalog-v1.md). Every operation has complete type, evaluation-order, effect, fault, ownership, cleanup, resource-event, interpreter, and legalization semantics. A normative helper is a sealed ISA feature with a portable reference algorithm, deterministic charges, bounded/incremental execution, and no consumer grammar, schema dispatcher, evaluator, callback, or frontend tag vocabulary. Whole parsing, recovery, deserialization, schema dispatch, and evaluator execution are forbidden helpers.

Function effects are verifier-derived. Admission ignores producer summaries except as untrusted acceleration hints, computes the least fixed point over the direct-call graph and each recursive SCC in a finite versioned effect lattice, and rejects any declared function effect contract that does not upper-bound the result. Indirect calls use their admitted signature/effect upper bound. `AdmissionLimits` bounds effect-lattice height, call edges, SCC size, refinement rounds, and cumulative effect-analysis work/facts.

## 7. Bindings, suspension, and task lifecycle

Static declarations name stable import/capability class and version, exact types, schema-relative paths, access/alias modes, ownership, capture/escape, effects, allocation/fallibility, partial writes, cleanup, affinity, suspension, re-entry, and panic policy.

`BoundProgram` contains immutable implementations, authorization, and `BindingSetId`; it contains no invocation pointer. `InvocationBindings` contains per-task capabilities and records lifetime, generation, mutability, ownership, affinity, and suspension policy. Direct-linked native artifacts hold a binding lease for their complete executable lifetime or perform an admitted generation guard at every boundary.

Only verifier-classified `SuspendSafe` values may occur in a `ContinuationSchema`. Each suspension site records task-state revision provenance and, where applicable, readiness epoch `q` or completion epoch `e`; ordered live values; cleanup on failed parking; declared ready/resume/accepted-resume/rejected block signatures; capability generations/affinity; surviving resource counters; abrupt-fault attribution; and response materialization. `await_input` and `invoke_async_import` are sealed terminators, not result-producing instructions.

The task lifecycle is a closed state machine. `r` is a monotonically changing task-state revision used only by CAS. `q` is a stable `await_input` readiness epoch; `e` is a stable async completion epoch. Epochs remain unchanged through their operation and are invalidated only by success, fault, abandonment, or terminal completion. A yield becomes externally visible only through an atomic scheduler publication gate paired with the parked-state commit: completion/supply cannot advance past the parked state until that gate either publishes the yield or atomically suppresses it and retains the response for direct resume. Thus no observer can receive a yield after the task has resumed.

| Authoritative state | Event | Sole commit transition | Winner action | Loser action |
|---|---|---|---|---|
| Running(r) | ready `await_input` validates | Running(r) -> Running(r+1) at `ready` | commit temporary value once | not concurrent at this execution point |
| Running(r) | already-ready `await_input` value fails validation | Running(r) -> Faulting(r+1,primary_fault at await `InstId`) | destroy temporary value; run applicable obligation ledger; allocate no q; publish no yield | not concurrent at this execution point |
| Running(r) | unready `await_input` continuation prepared with fresh q; publication wins gate | Running(r) -> ParkedInput(r+1,q,continuation) | publish exactly one typed yield before supply may advance | destroy unpublished continuation; no yield |
| Running(r) | unready `await_input`; valid supply wins publication gate | Running(r) -> ResumingInput(r+1,q,continuation,value) | suppress yield and retain validated value for `resume` | later supply diagnostic only |
| Running(r) | unready `await_input`; invalid supply wins publication gate | Running(r) -> Faulting(r+1,primary_fault) | suppress yield, destroy continuation/value, invalidate q, run fault cleanup | later supply diagnostic only |
| Running(r) | `await_input` preparation fails | Running(r) -> Faulting(r+1,primary_fault) | run failed-parking cleanup through obligation ledger | discard temporary failure record |
| ParkedInput(r,q,continuation) | first valid value for q | ParkedInput -> ResumingInput(r+1,q,continuation,value) | retain validated value | stale/duplicate/wrong-q supply diagnostic only |
| ParkedInput(r,q,continuation) | first value for q fails validation | ParkedInput -> Faulting(r+1,primary_fault) | retire continuation, invalidate q, run fault cleanup | duplicate failure diagnostic only |
| ResumingInput(r,q,continuation,value) | commit succeeds | ResumingInput -> Running(r+1) at `resume` | commit value; retire continuation; invalidate q | later supply diagnostic only |
| Running(r) | async submission rejects | Running(r) -> Running(r+1) at `rejected` | commit unchanged request/fault; no token | not concurrent at this execution point |
| Running(r) | async submission accepts with fresh e | Running(r) -> AwaitingAsync(r+1,e,token,empty) | request becomes scheduler-owned; begin continuation preparation | acceptance cannot publish/yield |
| AwaitingAsync(r,e,token,empty) | continuation arrives; publication wins gate | AwaitingAsync -> ParkedAsync(r+1,e,continuation,token) | publish exactly one typed yield before response may advance | destroy duplicate continuation |
| AwaitingAsync(r,e,token,empty) | valid response wins publication gate with prepared continuation | AwaitingAsync -> ResumingAsync(r+1,e,continuation,response) | suppress yield; consume token and retain response | later completion diagnostic only |
| AwaitingAsync(r,e,token,empty) | invalid response wins publication gate with prepared continuation | AwaitingAsync -> Faulting(r+1,primary_fault) | suppress yield; retire token/continuation, invalidate e, run fault cleanup | later completion diagnostic only |
| AwaitingAsync(r,e,token,empty) | valid response arrives first | AwaitingAsync -> EarlyResponse(r+1,e,token,response) | retain validated response; no yield | duplicate completion diagnostic only |
| AwaitingAsync(r,e,token,empty) | first response for e fails validation | AwaitingAsync -> Faulting(r+1,primary_fault) | detach token, invalidate e, run fault cleanup; no yield | duplicate failure diagnostic only |
| AwaitingAsync(r,e,...) | preparation fails | AwaitingAsync -> Faulting(r+1,primary_fault) | detach token, request cancellation, run failed-parking cleanup; no yield | observe winner |
| EarlyResponse(r,e,token,response) | continuation finishes | EarlyResponse -> Running(r+1) at `accepted_resume` | consume response; destroy unpublished continuation; retire token; invalidate e | duplicate completion diagnostic only |
| EarlyResponse(r,e,token,response) | preparation fails | EarlyResponse -> Faulting(r+1,primary_fault) | detach token, dispose response, run failed-parking cleanup; no yield | observe winner |
| ParkedAsync(r,e,continuation,token) | valid first response for e | ParkedAsync -> ResumingAsync(r+1,e,continuation,response) | consume token once; retain response | stale/duplicate/wrong-e diagnostic only |
| ParkedAsync(r,e,continuation,token) | first response for e fails validation | ParkedAsync -> Faulting(r+1,primary_fault) | retire token/continuation, invalidate e, run fault cleanup | duplicate failure diagnostic only |
| ResumingAsync(r,e,continuation,response) | commit succeeds | ResumingAsync -> Running(r+1) at `accepted_resume` | commit response; retire continuation/token; invalidate e | later completion diagnostic only |
| Running/ParkedInput/ResumingInput/AwaitingAsync/EarlyResponse/ParkedAsync/ResumingAsync | abandonment | current(r) -> Abandoning(r+1,primary_outcome) | invalidate q/e, detach token, execute `AbandonPlan` | observe winner; no cleanup duplication |
| any executable state with empty/discharged obligation ledger | normal return | current(r) -> Terminal(r+1,result) | publish completion/join visibility after CAS | no task mutation |
| any executable state with pending cleanup | normal return | current(r) -> Cleaning(r+1,primary_result) | invalidate q/e, detach token, dispatch obligation ledger | observe winner |
| any executable state | primary abrupt fault not covered by a specific row above | current(r) -> Faulting(r+1,primary_fault) | invalidate q/e, detach token, dispatch fault obligation ledger | observe winner |
| Cleaning/Abandoning(r,primary_outcome) | obligation ledger completes | current -> Terminal(r+1,primary_outcome) | publish completion/join visibility after terminal CAS; retain diagnostics separately | duplicate completion no mutation |
| Faulting(r,primary_fault) | ledger completes without cleanup panic | Faulting -> Terminal(r+1,primary_fault) | publish completion/join visibility after terminal CAS | duplicate completion no mutation |
| Faulting(r,primary_fault) | ledger completes after cleanup panic | Faulting -> Terminal(r+1,TaskFault::CleanupFailed { primary: primary_fault, first_cleanup_site }) | retain `TaskCleanupDiagnostic` additionally; publish completion/join visibility after terminal CAS | duplicate completion no mutation |

Scheduler ownership begins at `Accepted`. Detaching a token requests advisory cancellation; late completion is disposed without task mutation. Neither task nor failed-parking cleanup reclaims an accepted request. For async work, only `e` validates completion; task revision `r` never does. For `await_input`, only readiness epoch `q` validates supply. A first active-epoch value/response may mutate state only before any valid first response has been retained; everything observed in `EarlyResponse`, `ResumingInput`, or `ResumingAsync` is duplicate/stale and diagnostic-only.


Abandonment observed after `Cleaning`, `Faulting`, or `Abandoning` has committed is a loser event with no state mutation; it never replaces the preserved outcome or redispatches the obligation ledger.
Every discardable state has a verifier-owned `AbandonPlan` covering the complete frame, continuation, temporary response, partially initialized aggregate, resource-counter, and capability-lease obligations. `Cleaning`, `Abandoning`, and `Faulting` use the same required-affinity dispatcher and exactly-once obligation ledger and remain committed until deferred cleanup completes. Running generated code may abandon only at an admitted safepoint/pollpoint under exclusive runtime ownership. Cleanup failure always emits `TaskCleanupDiagnostic { abandon_plan, first_failed_obligation, panic_or_fault, originating_inst }`; later obligations still run once and cleanup failure never revives the task. For abrupt `Faulting`, the first caught cleanup panic changes the final task result to `TaskFault::CleanupFailed { primary, first_cleanup_site }`; normal completion cleanup and abandonment preserve their primary outcome while retaining the diagnostic. Accepted requests and scheduler-owned state are excluded from task cleanup.
## 8. Admission

Admission has one pre-allocation phase followed by fifteen ordered semantic phases:

0. bounded raw bootstrap/directory/section scan: exact header, integrity, `ImageId`, all byte-derived count ceilings, impossible lengths, ordering, overlap, alignment, canonical directory/schema framing, and fallible reservations; output is a bounded structural image view containing no semantic authority;
1. feature/version/import support negotiation;
2. canonical type graph and nominal identity;
3. logical constants/table schemas and borrowed physical views;
4. canonical logical constant/table streaming into bounded temporary semantic component digests;
5. function/block directory and canonical instruction boundaries;
6. operand shapes, definitions, uses, terminators, and block signatures;
7. complete semantic decoding, canonical serialization, `ExecutableId`, and claim comparison;
8. CFG, normalized edges, reachability, predecessors, dominance, SSA/block arguments;
9. ownership, initialization, borrows, cleanup, and capability non-escape;
10. logical-region and VM-owned memory safety;
11. relation validation and private witnesses;
12. effects, imports, calls, faults, source sites, evaluation order, and equality of used versus declared features;
13. suspension live sets, continuation schemas, completion materialization, and abandonment;
14. static resource obligations and pollpoint coverage;
15. immutable private `TrustedFacts` publication.

References elsewhere to historical architecture phases 1..16 map to the semantic phases above by subtracting one; historical phase 8 is semantic phase 7. Phase 0 alone may inspect untrusted container counts, and it MUST establish every allocation ceiling needed by later decoders.
`AdmissionLimits` bounds raw/decoded bytes, sections, types, groups, functions, blocks, instructions, operands, edges, tables/rows, imports, capabilities, nesting, per-function facts, physical expansion, work per phase, total work, scratch, and retained derived facts. Import/capability limits include cumulative declaration bytes, binding-validation work, effect-analysis work, and retained binding facts. Decoders MUST NOT reserve directly from untrusted counts. Limit failure is typed and MUST NOT rely on process OOM.

Optional derived caches are immutable, fallibly reserved, atomically published, bounded in work and retained bytes, and coalesced or independently bounded under concurrency. Failure discards partial cache state and preserves streaming interpretation.

## 9. Reference interpreter

The reference interpreter is the universal correctness lane. It is a safe streaming switch over immutable admitted bytes owned by `VerifiedProgram` or pinned by `ImageLease`.

It uses admitted directories, type-specialized handlers, exact typed storage banks, the mandatory base plan, a generic simultaneous edge-copy algorithm bounded by maximum edge arity, an explicit task call stack, verified tables, exact cleanup/fault actions, and semantic resource charging.

SSA values are semantic values, not persisted frame slots. Slot assignment and move schedules are derived. Edge moves are simultaneous; cycle breaking uses typed temporaries. Drops happen only after all sources are captured. Move scheduling cannot fault.

Only values named by the admitted continuation schema become task-resident. Predecode and fused handlers are optional immutable caches that preserve every `InstId`, fault, effect, source, and resource identity.

## 10. Legalization and native artifacts

`LegalProgramV1` is the closed target-independent PHON payload defined by the opcode catalog. `LegalizedProgramHandle` is a runtime-owned opaque pair of that payload with strong `Arc<VerifiedProgram>`/`ImageLease` ownership and private witness authority; it cannot be constructed from standalone PHON bytes. It preserves blocks/parameters, semantic identities, effects/faults/source, ownership/cleanup, suspension/continuations, resource events, and origin maps. Legalization MUST be total for every admitted semantic entry and MUST NOT choose pointer width, host aggregate layout, ABI locations, registers, flags, stack slots, branch forms, stencil IDs, or native adapter layout.

Every supported native profile has a complete ordinary encoder for every admitted physical form. The backend order is:

```text
LegalProgramV1
 -> target virtual-register MIR
 -> target-required edge splitting
 -> parallel-copy bundles
 -> register allocation
 -> post-allocation copy resolution
 -> frame/fault/safepoint/root/continuation maps
 -> physical-form selection and stencil eligibility
 -> fixed-point layout/relaxation with known lengths
 -> ordinary emission or manifest-equivalent stencil emission
 -> internal validation
 -> all-or-nothing publication
 -> quiescent retirement
```

Copy-and-patch MAY replace only final emission of a bounded finalized physical-form fragment. Ordinary and stencil emission consume an identical physical-form plan and, after relocation/address normalization, produce identical instruction bytes and metadata. Selection-changing, fusion, or length-changing templates are a separate optimization experiment.

`CompilationLimits` bound functions, semantic/target operations, edges, allocator work, spill/copy records, layout iterations, veneers, pools, relocations, code/data/metadata bytes, scratch, retained compiler bytes, and work. Failure publishes no artifact and leaves interpretation available.

Native compilation and promotion also obey a process-wide `NativeResourcePolicy`: maximum queued jobs, active workers, aggregate compiler scratch, retained compiler state, code-cache bytes, artifact count, and publication work. Admission to the queue is deterministic under the policy; refusal, cancellation, timeout, or eviction leaves interpretation available. Cache eviction first withdraws entrypoints and uses the same quiescent retirement protocol. No set of individually admitted programs may create unbounded concurrent compilation or retained native code.

x86-64 profiles model condition values, two-address constraints, fixed shift/divide registers, byte-register rules, register classes, ABI saves, ModRM/SIB legality, calls/maps, System V versus Win64, branch relaxation, CET/IBT, and Windows CFG.

AArch64 profiles model GPR/SIMD classes, NZCV, immediate constraints, register pairs, literal materialization, call clobbers, `x16/x17` veneer scratch, platform `x18`, AAPCS64/Darwin/Windows/arm64e, veneers/islands, BTI, PAC, unwind/debug, instruction-cache synchronization, and executable-memory policy.

A native invocation uses a checked task-owned/segmented stack or proves a `NativeStackEnvelope` sufficient for every physical frame sequence allowed by semantic call-depth limits. Otherwise it remains interpreted.

Compilation produces an `UnboundNativeArtifact`: immutable finalized physical-form plans, code/data/metadata images, relocation/binding slots, target/security requirements, origin maps, and strong `VerifiedProgram`/`ImageLease` references. It has no public entrypoint, executable publication, direct binding pointer, or `BindingLease`. Its cache key excludes a concrete binding set unless compilation specialized on a declared binding guard.

`link(UnboundNativeArtifact, BoundProgram)` validates exact static declarations and either installs guarded indirect binding slots or acquires one immutable `BindingLease` for every direct-linked declaration. Linking then performs mappings, relocations, protections, cache synchronization, security and unwind/debug registrations, internal validation, and all-or-nothing publication. Failure rolls back every completed step and exposes no entrypoint. The resulting `NativeArtifact` owns all code, read-only, writable, metadata, table, literal, veneer, trampoline, patch, and binding mappings plus strong unbound-artifact/program/image/binding references. Calls enter only through tracked `EntrypointHandle`s; a raw entrypoint is valid only while an `EntrypointLease` pins the artifact and MUST NOT escape.

Compilation cache keys cover `ExecutableId`, `ImageId` when physical bytes are embedded, legalization/backend/encoder/stencil epochs, target/ABI/CPU/security/instrumentation profile, and compilation policy. Link cache keys additionally cover `BindingSetId`, declaration implementation identities, generation-guard policy, and direct-link leases. Publication generations prevent an artifact retired or superseded during linking from becoming visible.

Retirement first withdraws entrypoints and prevents new execution-reference acquisition. `NativeArtifact` distinguishes artifact-owned backing leases from independently acquired external pins: `EntrypointLease`, active execution/unwind pins, and any exported image/binding inspection pins. Retirement waits only for those external pins to reach zero, then removes unwind/debug/CFG registrations, releases executable/read-only/writable mappings, and finally drops the artifact-owned `BindingLease`s, `ImageLease`, unbound artifact, program, image, and binding references. Shared refcounts use the predicate `strong_count == retirement_owner_count`; the retirement owner's own references never participate in its wait. No mapping or backing reference may disappear while code can execute, unwind, inspect a literal/table, or call a direct-linked binding. Interpreter-only operation is conforming on iOS, WASM, and no-exec environments.

## 11. Frontend profile boundaries

### Vix/Vixen

The required build-time closure descriptor for this boundary is `weavy-vix-profile-v1` in [Runtime profile manifests v1](weavy-runtime-profiles-v1.md). Its typed yields are requests, not permission to create demand, publish results, or interpret scheduler policy. The concrete manifest cannot be approved until the persistent-handle generation/revocation/suspension rules are owner-approved; callback re-entry is prohibited in version 1.


### Phon/Vox

The required build-time closure descriptor for this boundary is `weavy-phon-profile-v1` in [Runtime profile manifests v1](weavy-runtime-profiles-v1.md).


### Snark

The required build-time closure descriptor for this boundary is `weavy-snark-profile-v1` in [Runtime profile manifests v1](weavy-runtime-profiles-v1.md). Parser tables may contain callable references only under admitted exact signatures; retrieving one never invokes it. The concrete manifest cannot be approved until scanner snapshot/cancellation, parser-level zero-width progress, deterministic tie-break, and quota-category policies are owner-approved.


### Fable

The required build-time closure descriptor for this boundary is `weavy-fable-profile-v1` in [Runtime profile manifests v1](weavy-runtime-profiles-v1.md). Its VM `usize/isize` width, Unicode data version, arithmetic/NaN rules, and partial-write policy remain explicit owner decisions; the specification does not select the reviewed 64-bit recommendation without that approval.


### facet-json

The required build-time closure descriptor for this boundary is `weavy-facet-json-profile-v1` in [Runtime profile manifests v1](weavy-runtime-profiles-v1.md). JSON dialect, suffix, duplicate/unknown-field, probe-rollback, and OOM policies remain explicit owner decisions; they are exact module requirements after approval, never ambient runtime defaults.


### facet-hash/equality

The required build-time closure descriptor for this boundary is `weavy-facet-hash-equality-profile-v1` in [Runtime profile manifests v1](weavy-runtime-profiles-v1.md). Float equality/hash and generic-hasher panic/effect/fault policy remain explicit owner decisions. Pointer identity, runtime cycles, and unordered-container behavior remain outside the first-cut profile unless separately approved.


### VM-owned facilities

The version-1 semantic machine exposes the bounded VM-owned facilities enumerated by [Opcode catalog v1](weavy-opcode-catalog-v1.md): fallible vectors, byte buffers, typed builders, arenas, stacks, queues, min-priority worklists, parent-linked journals, persistent child-list/rope structures, immutable/shared and copy-on-write state, strings, and generation-checked task/session handles. Each facility's feature descriptor defines ownership, allocation failure, cleanup, semantic charges, maximum per-operation work, retained-byte bounds, and required pollpoints. These facilities are generic; none may embed or invoke a consumer engine.

### Producer lowering contract

Every producer emits a typed `ProducerLoweringManifestV1` beside each canonical semantic module. It records producer revision and policy versions; source construct IDs; resulting `FunctionKey`/`BlockKey`/`InstId` ranges; required features; logical tables/relations; imports/adapters; ownership/cleanup; suspension sites; attribution references; and the production-path differential oracle. Every source construct in the corpus maps exactly once or is a typed rejected/unsupported case. The manifest is provenance and coverage evidence, not semantic authority; admission derives features independently.

The required lowering families are:

- Vix/Vixen: VIR functions/islands/edges become functions, blocks, branches, table references, `await_input`/async-import terminators, pollpoints, and attribution; demand, memo, tickets, receipts, placement, and publication stay outside Weavy.
- Phon/Vox: schema-derived branches, cursor/span operations, products/sums/sequences, builders, cleanup, and byte boundaries become ordinary SSA plus approved generic helpers; no serializer/deserializer plan or schema dispatcher crosses the boundary.
- Snark: grammar states/actions/recovery become ordinary SSA, explicit automaton tables/relations, exact callable references, scanner imports, resource categories, and cleanup; no parser engine, grammar callback, or recovery helper crosses the boundary.
- Fable: predicates, short circuit, recursion, mutation, aggregate construction, projections, imports, OOM, and partial writes become SSA, adapters, builders, explicit fault/cleanup; no evaluator/query plan or hidden closure callback crosses the boundary.
- facet-json: cursor/checkpoint/rollback, policy branches, field matching, replay, construction, spans, suffix, OOM, and cleanup become SSA, adapters, tables, builders, and only approved lexical helpers; no parser/deserializer engine crosses the boundary.
- facet-hash/equality: traversal, recursion, early exit, projection, floats, and ordered generic-hasher calls become SSA plus exact adapter/effect declarations; no traversal plan or byte-stream substitution crosses the boundary.

Gate 0 Task 3 validates manifest coverage against producer traces and the frozen corpus. A missing construct mapping, hidden consumer engine, or undeclared feature blocks that corpus cell.

## 12. Conformance and change control

A format or semantic change MUST identify which authority changes:

- bootstrap/directory/schema closure/required section kind: module-format version;
- canonical semantic serialization or identity: semantic module version and identity epoch;
- opcode/helper/relation meaning: official feature version;
- legal vocabulary: legalization epoch;
- target MIR/allocator/layout internals: backend epoch only when semantic and oracle behavior is unchanged;
- stencil set: stencil-manifest epoch;
- source mapping: attribution schema/epoch.

Conformance requires malformed-image, admission, interpreter, legalization, native parity, publication rollback, retirement, and consumer-boundary oracles described in [Gate 0 plan](weavy-gate-0-plan.md). No implementation may select the new physical program encoding or begin consumer cutover before Gate 0 produces an approved decision record.
