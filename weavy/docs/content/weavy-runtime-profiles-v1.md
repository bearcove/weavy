# Weavy runtime profile manifests v1

## Status

Normative schema, closure rules, and boundary templates for Gate 0 build-time profiles. A runtime profile is an immutable generated `RuntimeProfileManifest`, not a Cargo-feature union discovered at runtime and not an open registry. Generated concrete manifests are the exact closure instances; this document does not fabricate their IDs before the exercised feature corpus and owner policy decisions are approved.

Each manifest records the following canonical fields in this order:

```text
RuntimeProfileManifestV1 {
    profile_id: bytes[32],
    semantic_module_versions: sorted list<{ major: u16, max_minor: u16 }>,
    features: sorted list<{
        namespace: opcode | helper | relation | capability,
        stable_id: bytes[16],
        canonical_name: string,
        major: u16,
        max_minor: u16,
        semantic_descriptor_digest: bytes[32],
        interpreter_handler_id: bytes[16],
        legalizer_id: option<bytes[16]>,
        target_handler_ids: sorted list<bytes[16]>,
    }>,
    portable_width_and_policy_versions: sorted list<PolicyVersion>,
    dependency_allowlist_digest: bytes[32],
    dependency_allowlist: sorted list<PackageIdentity>,
    build_script_assets: sorted list<AssetIdentity>,
    linked_symbol_and_section_denylist: sorted list<string>,
    manifest_digest: bytes[32],
}
```

`PolicyVersion` is exactly `{ policy_key: PolicyKeyV1, major: u16, max_minor: u16, compatible_minor_digests: list<bytes[32]> }`; the digest history has length `max_minor + 1` and satisfies the normative append-only compatibility rule. Records sort canonically with no duplicate `(policy_key,major)`.

`PackageIdentity` is `{ name: String, source: Registry { canonical_url, checksum } | Git { canonical_url, revision: bytes[20] } | Path { workspace_relative_path, tree_digest: bytes[32] }, version: String, enabled_features: sorted list<String>, target_predicate: String }`, sorted by canonical PHON bytes; exact duplicates reject. `dependency_allowlist_digest = BLAKE3("weavy.dependencies.v1\0" || CanonicalPhon(sorted dependency_allowlist))`. `AssetIdentity` is `{ workspace_relative_path: String, content_digest: bytes[32], executable: bool, role: String }`, sorted by path then remaining fields; duplicate paths with unequal records reject. Exact roots and vectors are canonical-semantic-schema artifacts.

Feature identity, version compatibility, collision rejection, and the exact `ProfileSemanticProjectionV1`/`ManifestDigest` domains are defined by section 3 of the [normative specification](weavy-bytecode-specification.md); their exact PHON roots and vectors are approved Gate 0 canonical-semantic-schema artifacts. Handler IDs identify reviewed implementations of one unchanged semantic descriptor; they are not open dispatch names.

The generated manifest and its digest are embedded at build time. Admission compares every required feature against this closure. There is no public API to add a semantic operation, helper, relation, legalizer, target handler, capability class, or consumer dialect after build.

The only nonshipping exception is an owner-approved Gate 0 study manifest referenced by `helper-study-authority.styx`. It uses the same closed manifest field order and profile digest procedure; within `features`, the existing 16-byte `stable_id` slot carries `StudyFeatureId = BLAKE3("weavy.study.feature.v1\0" || study_authority_digest || namespace || "\0" || candidate_id || "\0" || alternative)[0..16]`, and the descriptor digest binds the complete provisional semantics/handler set. It is accepted only when admission is explicitly invoked with `Gate0StudyPolicy` for the matching authority digest and experimental module-format roots. Ordinary admission rejects it. Study IDs, manifests, and cache entries cannot alias or be promoted into shipping `FeatureIdV1`/`ProfileId` authority; Task 9A invalidates them and regenerates final canonical artifacts.

## Dependency rule

`weavy-core` owns the canonical type/SSA machine, admission, reference interpreter, common legal vocabulary, generic VM facilities, and feature-descriptor interfaces. Extension profiles depend on core. Core does not depend on, enumerate Rust types from, initialize, or link extensions. A top-level composition crate selects an explicit allowlist and generates the final manifest.

Encoding/decoding of the PHON-backed container remains in the acyclic sibling/lower `weavy-phon` layer. No dependency path `weavy -> phon-engine -> weavy` is allowed.

Every deployable closure has a checked dependency allowlist, feature graph, build-script asset list, linked-symbol/section denylist, and fresh-process binary/initialization/RSS oracle. Cargo additive features alone do not prove isolation.

## `weavy-core-v1`

Every concrete core manifest includes the canonical machine infrastructure required to represent and admit its corpus, but semantic feature entries are the exact exercised subset generated from the approved registry:

- canonical scalar/type/signature graph, recursive groups, functions, typed SSA, block parameters, sealed terminators, and `InstId` infrastructure;
- ownership, explicit borrows, builders, cleanup, typed faults, deterministic resources, bindings, suspension/continuation/abandonment, hostile-input admission, `BaseExecutionPlan`, streaming interpreter, and `LegalProgramV1` infrastructure;
- only the opcode/helper/relation/capability descriptors actually used by at least one required Gate 0 corpus module or mandatory negative oracle;
- generic VM facilities and relation validators only when their exact generated operations/variants are exercised.

Infrastructure that has no independently selectable semantic feature ID is not removed by coverage pruning. Every semantic feature in a concrete manifest MUST have corpus or negative-oracle coverage; an uncovered feature is removed from that manifest. Every required corpus feature MUST appear in its manifest. The `full` manifest is the union of approved exercised consumer manifests, not every prose-listed possible facility.

Forbidden dependencies/symbols:

- Snark grammar/parser/recovery/tree types;
- Fable evaluator/query-plan types;
- facet-json parser/deserializer engine types;
- facet-hash/equality plans;
- Vix memo, demand, ticket, receipt, Store, placement, or publication authorities;
- consumer-specific target encoders or stencils.

## `weavy-vix-profile-v1`

Adds only:

- typed `await_input` boundary;
- registered primitive/import yield boundary;
- edge safepoints and parking/nonparking interior pollpoints;
- persistent Store/value **data handles**, gated on owner-approved generation, revocation, suspension, replay, and abandonment contracts;
- causal attribution records from `InstId` to VIR/island/source;
- Vix-specific resource-event category IDs where required by the corpus.

It MUST NOT contain or invoke:

- `Location`, `Recipe`, or `Content` identity derivation;
- memo nomination/read-set validation;
- demand or effect-ticket creation/join/cancellation policy;
- receipts or publication;
- capability discovery or placement;
- scheduler replay meaning.

Its interpreter handlers yield typed requests only. Version 1 prohibits callback re-entry. Its legalizers lower to common legal suspension/import operations. Target handlers are generic Weavy boundary handlers, not Vix engine callbacks. No concrete Vix manifest is approved until persistent-handle policy is closed.

## `weavy-phon-profile-v1`

Adds generic:

- byte cursor/input and output operations;
- checked spans and borrowed/owned byte/string results;
- schema-bound product/sum/sequence projection declarations;
- typed builders and cleanup adapters;
- bounded lexical/byte helpers justified by the codec corpus.

It MUST NOT contain `run_phon`, whole encode/decode, schema traversal, schema dispatch, a serializer/deserializer plan, or a codec-specific target backend. PHON schema-derived control is ordinary SSA. Cache identity is per canonical admitted program/shape, not a hidden Rust type registry.

## `weavy-snark-profile-v1`

Adds generic facilities required by parser programs only when absent from core:

- external scanner capability declarations, gated on owner-approved snapshot ownership/cancellation policy;
- parser resource category IDs and quotas, gated on owner-approved exact categories and limits;
- relation/automaton features, gated on owner-approved parser-level zero-width progress and deterministic tie-break rules.

It MUST NOT add opcodes/helpers for parser actions, grammar-symbol interpretation, LR/GLR scheduling, recovery, tree construction, incremental reuse, regex engine objects, or whole parser execution. Those semantics are ordinary SSA plus logical tables/portable automata. The external scanner is the only Snark-specific host boundary.

These policies are required for the Gate 0 Snark corpus and MUST be resolved before the concrete Snark manifest is approved, not deferred to consumer migration.

## `weavy-fable-profile-v1`

Adds closed schema-relative Facet adapter declarations needed for:

- read-only scalar/product/enum projection;
- strings and sequences;
- aggregate initialization/overwrite/drop;
- stable imports;
- typed allocation/OOM and partial-write cleanup.

It MUST NOT import evaluator plans, branch/query plans, closures as hidden evaluator callbacks, host-native aggregate layouts, or consumer-native code generators. Read-only predicates, short-circuiting, recursion, mutation, and construction remain ordinary SSA.

The concrete Fable manifest remains blocked on owner approval of VM `usize/isize` width, Unicode database/version, checked-arithmetic and ordered-NaN rules, and partial-write policy. The reviewed 64-bit width recommendation is not normative until approved. After approval, every corpus module declares the exact policy feature versions it uses; the runtime supplies no ambient fallback.

## `weavy-facet-json-profile-v1`

Adds closed adapter/capability declarations for:

- invocation-scoped byte/string input;
- raw/decoded spans and borrow leases;
- schema-relative value construction and projection;
- typed builder initialization/overwrite/abort;
- owned output and borrowed-result lifetime contracts.

It MAY require the generic `json_lex_step` helper version. It MUST NOT contain a whole parser, schema dispatcher, replay engine, deserializer plan, root host call, or consumer-specific target handler. Cursor, checkpoint, rollback, policy branching, field matching, enum replay, and control are ordinary SSA.

The concrete facet-json manifest remains blocked on owner approval of the exact JSON/JSONC and trailing-comma matrix, duplicate/unknown-field behavior, suffix policy, probe rollback, and OOM surface. After approval, the runtime allowlist and every affected module name the exact policy feature versions; module requirements cannot substitute for the runtime allowlist.

## `weavy-facet-hash-equality-profile-v1`

Adds:

- closed typed `FacetValueView` projection declarations;
- generic Hasher capability methods as distinct ordered effects;
- float equality/hash policy and generic-hasher panic/effect/fault policy, gated on owner approval.

It MUST NOT contain `HashIntrinsic`, `EqualityPlan`, whole traversal, consumer-native traversal, or byte-stream substitution for generic Hasher calls. Traversal, recursion, short-circuit, and policy branches are ordinary SSA.

The first-cut profile excludes pointer identity, cyclic runtime object graphs, and unordered native containers. Those require separately approved policy features and corpus before they can enter a concrete manifest.

## Composition profiles used by Gate 0

Gate 0 generates, builds, and measures these physical closure families separately:

1. `core-{interpreter,native,native-stencils}`;
2. for each consumer profile `P` in `{vix, phon, snark, fable, facet-json, facet-hash-equality}`, `core-P-{interpreter,native,native-stencils}` for every execution variant applicable to its corpus;
3. `full-{interpreter,native,native-stencils}` containing every approved Gate 0 consumer profile;
4. optional `full-heavy-experiment`, isolated from shipping decisions.

Every approved named closure is emitted as a concrete manifest. A prerequisite decision of `Deferred` yields `Blocked(ProfilePolicyDeferred { policy_key, major })`; `Rejected` yields `Blocked(ProfilePolicyRejected { policy_key, major })`; neither fabricates a manifest or `ProfileId`, and neither supports selection. `weavy-phon` is the lower/sibling codec; consumer oracle code remains outside the runtime closure. Composition crates may depend downward, but core/extensions never depend upward.

Each closure records stripped binary sections, dependency tree, initialization latency, baseline RSS, linked symbols/sections, and manifest digest. A candidate cannot claim a tiny-program footprint using the full closure or claim feature isolation from a Cargo flag without this physical closure evidence.

## Extension and compatibility

A profile operation/helper/relation/capability change requires its official feature version to change. A new implementation of unchanged semantics may change implementation/backend epochs but not semantic feature identity. A runtime accepting a module minor version MUST support every exact required feature and compatible minor range; absence rejects admission rather than silently lowering to a legacy consumer path.
