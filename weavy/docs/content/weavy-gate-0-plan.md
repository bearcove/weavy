# Weavy Gate 0 execution plan

> **For agentic workers:** Execute this plan task-by-task with independent review gates. Gate 0 is an experiment and executable-specification phase, not a consumer migration.

**Goal:** Produce sufficient controlled evidence to select the canonical Weavy program-section instruction encoding, compact-baseline execution policy, table-profile policy, and optional predecode policy while preserving the approved architecture and frozen PHON-backed container.

**Architecture:** Every A/B/C variant consumes the same admitted semantic program and semantic/resource/attribution/pollpoint/source/consumer oracle. Only B and C share `LegalProgramV1`, target MIR, allocation, physical-form plan, native metadata, and target profile; A constructs no native-only intermediate. Gate 0 may conclude interpreter-only, ordinary compact native, ordinary native plus measured stencil substitutions, a size-dependent combination, and no heavy tier.

**Tech stack:** Rust 2024, PHON schema/storage, Weavy admission/reference interpreter, complete ordinary x86-64 and AArch64 encoders, copy-and-patch stencil manifests, cargo-nextest, criterion/divan-style controlled benches where already used, and production-path consumer oracles.

## Global constraints

- Authority precedence is: architecture v6 constraints; normative cross-cutting specification; delegated companion catalogs/schemas; this procedural plan; mechanically checked generated artifacts. Conflicts and missing delegated descriptors fail closed.
- The existing PHON 1.0 bootstrap, directory, schema bundle, section namespace, and physical integrity tag are the compatibility baseline. Typed semantic candidate roots require the explicit experimental-format authority below and do not silently redefine 1.0.
- Gate 0 compares only program-section instruction encodings and evidence-driven physical table profiles inside that container.
- No public consumer entry point is migrated in Gate 0.
- No legacy consumer route is deleted in Gate 0.
- No candidate receives a richer mandatory or optional predecode, helper set, table witness, source map, or verifier hint than another candidate.
- Copy-and-patch is a hypothesis, not a required result.
- Every native physical form has a complete ordinary encoder.
- No persisted native machine code is loaded or executed.
- Interpreter-only and no-exec configurations are first-class results.
- Claims against Serde require a separate direct benchmark and are outside Gate 0 unless explicitly added.

## 1. Required artifacts

Gate 0 produces these version-controlled artifacts:

1. `weavy/docs/content/weavy-bytecode-specification.md` — normative semantic, admission, interpretation, legalization, native-lifecycle, and frontend-boundary clauses;
2. section 2 of `weavy/docs/content/weavy-bytecode-specification.md` — exact bootstrap, directory, schema closure, section invariants, canonical image layout, and identity terminology;
3. `weavy/docs/content/weavy-program-encoding-candidates.md` — exact grammars for all three candidate instruction encodings;
4. `weavy/docs/content/weavy-relation-contract-v1.md` — complete version-1 `RelContract` enum with work and storage formulas;
5. `weavy/docs/content/weavy-opcode-catalog-v1.md` — sealed opcode/helper semantics, effects, faults, ownership, resource events, and legalization rules used by the corpus;
6. `weavy/docs/content/weavy-runtime-profiles-v1.md` — normative profile schema, closure rules, and boundary templates; generated concrete manifests are the exact closures;
7. `weavy/gate0/canonical-semantic-schema.styx` — owner-approved exact PHON schemas, roots, enum tags, field/key widths, sort rules, `ExecutableId`/`ProfileId` projections, `LegalProgramV1`, and identity vectors;
8. `weavy/gate0/feature-registry.styx` — exhaustive concrete canonical feature records generated from the approved catalogs/schema;
9. `weavy/gate0/owner-policy-decisions.styx` — one versioned approval/rejection/defer row per profile prerequisite;
10. `weavy/gate0/experimental-module-format.styx` — nonshipping candidate section roots/closure framing/version and promotion rule, explicitly approved by the format owner;
11. `weavy/gate0/producer-survey.styx` and `helper-study-authority.styx` — total producer construct survey, closed helper-candidate enumeration, domain-separated study identities, roots, handlers, and study manifests;
12. `weavy/gate0/helper-placement-results.phon` and `helper-placement-decision.md` — controlled ordinary-SSA/primitive/helper evidence and owner selection/defer result;
13. `weavy/gate0/corpus-manifest.styx` and producer lowering manifests — immutable corpus identities, policies, features, provenance, construct mapping, and oracle commands;
14. `weavy/gate0/target-profiles.styx` — exact ABI/CPU/security/no-exec profiles;
15. `weavy/gate0/decision-policy.styx` — frozen applicability, estimands, confidence, materiality, regression/footprint budgets, and selection predicates;
16. `weavy/gate0/experiment-manifest.styx` — complete typed provisional/study cell matrix, measurement protocol, seeds, retry/outlier/quiescence rules, and build flags;
17. `weavy/gate0/final-authority-experiment-manifest.styx` — post-Task-9A immutable mapping and rerun matrix from provisional study cells to regenerated final-authority cells;
18. `weavy/gate0/transcript-schema-registry.styx` and `weavy/gate0/result-schema.styx` — normative transcript/result schemas, including `Gate0ResultV1` and `HeavyTierResultV1`;
19. `weavy/gate0/results/*.phon` — typed raw measurements, failures, and semantic transcripts;
20. `weavy/gate0/report.md` — generated human-readable tables and Pareto analysis;
21. `weavy/gate0/decision.md` — approved selection or explicit no-selection result, including rejected candidates and evidence.

Generated reports MUST be regenerated from typed result data; they MUST NOT become a second hand-edited source of truth.

## 2. Candidate program encodings

Every candidate represents identical canonical functions, types, aliases, block directory, instruction identities, opcode vocabulary, operands, out-of-line switch/automaton payloads, logical tables, verifier obligations, and nonsemantic metadata.

### E16 — 16-bit static short/wide forms

- Base code units are little-endian `u16`.
- Every opcode has a fixed short form and a statically declared finite set of wide forms.
- Wide forms are selected canonically from operand bounds before emission.
- Instruction length has a fixed versioned maximum.
- A decoder can skip or validate one instruction without decoding prior instructions.
- Non-shortest legal forms are rejected.

### E32 — 32-bit fixed wordcode with AUX words

- Base code units are little-endian `u32`.
- The primary word contains opcode and statically assigned operand fields.
- A canonical bounded number of AUX words carries overflow operands or immediates.
- AUX ownership is local to the immediately preceding primary word.
- Orphan, surplus, reordered, or nonminimal AUX words are rejected.
- Instruction length has a fixed versioned maximum.

### EV — byte opcode with bounded canonical operands/varints

- One-byte opcode followed by opcode-specific operands.
- Integer operands use bounded canonical unsigned/signed varints where specified.
- Nonminimal varints and overlong encodings are rejected.
- Operand count and total instruction length have fixed maxima.
- Block-directory offsets permit direct random access; whole-program scanning is never required to enter a block.

### Common acceptance requirements

Each candidate MUST provide:

- precise ABNF/pseudocode grammar and byte examples;
- canonical shortest-form rule;
- impossible-length detection in container phase 0;
- exact maximum instruction length;
- direct block-directory access;
- bounded decoder/verifier state;
- streaming execution without mandatory whole-program decode;
- malformed/nonminimal fuzz oracle;
- deterministic encode/decode/re-encode identity;
- stable `InstId` mapping independent of byte offsets;
- equivalent source, fault, resource, suspension, and attribution behavior.

## 3. Corpus manifest

Every corpus entry records:

```text
CorpusId
ProducerRepository
ProducerRevision
ProducerCommand
SemanticModuleDigest
RequiredRuntimeProfile
FeatureVersions
ExpectedOracle
FrequencyClass = traced-production | production-root-unweighted | scale-representative | synthetic-boundary
SizeClass = tiny | small | median | large | extreme
```

The corpus manifest also records `InputCorpusDigest`, `InvocationDistribution`, `ExpectedTranscriptSchemaId`, `PolicyFeatureVersions`, and one closed applicability status for every complete experiment cell. A cell is never silently absent. Producer commands run from a clean recorded revision and emit canonical semantic modules, `ProducerLoweringManifestV1`, and typed expected transcripts; generated physical images are not corpus authority.

### 3.1 Vix/Vixen anchor

Record the current production `AST -> Graph VIR -> partitioned VIR -> Weavy -> runtime` entry point, exact caller closure, current consecutive ratchet score, and every currently green rung in that complete consecutive prefix. Freeze canonical semantic modules from that production lowering path; Gate 0 does not redirect it.

The frozen prefix must cumulatively cover every behavior it currently reaches, including tiny/medium/large eager pure islands, block parameters and local calls, immutable aggregates, lazy consumed and unconsumed inputs, primitive yields, Rust-async park/resume, task kill/replay/join, ticket survival and non-restart on task death, edge safepoints, parking/nonparking interior pollpoints, continuation live sets, indirect calls where representable, and production attribution. Features not yet reached by the consecutive prefix remain later Gate 1 certification work and do not justify invented Gate 0 fixtures.

Oracle transcript fields: every intermediate and final value/fault; demanded versus undemanded input order; primitive request/acceptance/completion; task park/resume/kill/replay/join; ticket survival/non-restart; semantic resource events; safepoints/pollpoints; suspension materialization; cleanup/abandonment; and the exact causal chain `interpreter/native PC -> InstId -> VIR node -> island -> source span -> demand chain` with typed task and primitive-effect links.

### 3.2 Phon/Vox anchor

Freeze the checked-in wire-shape roots:

- `Message`;
- `MessagePayload`;
- `RequestCall`;
- `RequestMessage`;
- `SchemaMessage`;
- `Payload`;
- `DodecaParseResult` as a scale representative unless a traced production call site establishes frequency.

Add selected small/median/large programs spanning encode/decode, borrowed/owned results, scalar/product/sum, option/result, nested collections, `CallBlock`, native-supported forms, and interpreter fallback forms.

Oracle: exact encoded bytes, decoded values, typed malformed-input faults, borrow lifetimes, builder cleanup, per-shape construction/admission/cache events. Report every root independently. Frequency-weight only `traced-production` entries.

### 3.3 Snark

Include:

- minimal deterministic LR grammar;
- production deterministic grammar;
- literal-heavy and regex-heavy lexers;
- zero-width boundary cases;
- GLR conflict grammar;
- recovery grammar;
- full Dibs PostgreSQL grammar by immutable corpus ID, producer revision, semantic digest, and expected state/transition/conflict counts;
- code-heavy, table-heavy, and hybrid representations of that identical semantic parser program.

Oracle: complete trees, spans, errors, recovery decisions, scanner calls, table/automaton event counts, category quotas/resource faults, state/transition/conflict counts, and proof that load does not invoke grammar construction, LR closure/table generation, or lexer-plan construction.

The architecture's 117k-state grammar is a named hard gate. The corpus manifest identifies the exact artifact satisfying it and records expected counts; “current largest” is not an identity. If the frozen Dibs artifact does not satisfy the 117k-state count, Gate 0 adds the immutable artifact that does. All three representations must admit, execute the semantic oracle, and produce resource/size measurements for that exact gate.

### 3.4 Fable

Include read-only scalar predicates, short-circuit branches, strings, nominal products/enums, direct recursion, read-only Facet projection, aggregate builders, mutable adapter operations, imports, typed OOM, and partial cleanup/fault paths.

Oracle: exact values, errors, source spans, adapter call transcript, resource events, integer boundaries, NaNs, Unicode whitespace, allocation, and cleanup order.

### 3.5 facet-json

Include scalar/product/option, nested arrays/sequences, ordered probes, enum replay, cursor checkpoint/rollback, duplicate/unknown policies as separate declared cases, strict skip, truncated hostile inputs, owned strings, borrowed spans, builder partial initialization, and root suffix behavior.

Oracle: exact value, cursor/error offsets, policy result, allocation count, adapter calls, drop order, and borrow lifetime.

### 3.6 facet-hash/equality

Include scalars, products, enums, option/result, acyclic recursive declared shapes, early unequal fields, adapter-heavy projections, floats including NaNs and signed zero, and generic hasher methods.

Oracle: equality result, projection count proving global short-circuit, typed hasher method identity/order, and hash/equality congruence under the declared policy.

### 3.7 Synthetic structural boundaries

Synthetic entries are allowed only for boundaries absent from real corpora:

- maximum instruction length;
- maximum block arguments;
- critical-edge simultaneous copies with cycles;
- largest permitted switch payload;
- branch-relaxation boundaries;
- x86-64 immediate/register constraint boundaries;
- AArch64 immediate, literal, and veneer boundaries;
- admission-count limits and cumulative overflow;
- relation-contract worst cases;
- continuation live-set limits;
- native stack-envelope boundaries.

They are labeled `synthetic-boundary` and never substitute for consumer evidence.

Every retained semantic feature MUST have either at least one real or scale-representative successful corpus use or one explicitly identified mandatory negative oracle. Features intended to execute successfully in experiment cells require the real/scale-representative use; a negative oracle alone may retain only a rejection/security feature whose success path is not part of the runtime profile. Any feature with neither form of coverage is removed from concrete manifests. Synthetic entries may fill only structural boundaries absent from real programs; they do not justify performance claims.

### 4.1 Execution variants

For every semantic module and applicable target profile:

- **A — streaming interpreter:** no predecode required; optional caches disabled for the primary comparison.
- **B — ordinary compact native:** same admitted program; complete ordinary encoder; stencil substitution disabled.
- **C — copy-and-patch emission:** identical to B through finalized physical-form plan; eligible fragments emitted through validated stencils.

A, B, and C share byte-identical admitted semantic programs, `ExecutableId`, admission policy, verifier epoch, semantic/resource/attribution/pollpoint/source-map obligations, target applicability, instrumentation, and consumer oracle.

Only B and C share `LegalProgramV1`, target MIR, instruction selection, physical constraints, allocation/spills/register assignment, native frame/root/fault/safepoint/continuation maps, target profile, build flags, finalized physical-form plan, known fragment lengths, and layout inputs. After normalizing relocation values and placement addresses, B/C instruction bytes and metadata MUST match. A's timed and retained path MUST NOT construct native-only intermediates. Any template that changes selection, fusion, length, layout, or metadata is outside B→C.

### 4.2 Program-encoding variants

Each semantic module is encoded independently as E16, E32, and EV. The same A/B/C execution variants consume each encoding. Admission and execution measurements MUST distinguish image decoding from semantic verification and derived-cache construction.

### 4.3 Table-profile variants

For selectable format-1.0 table views, each module has a canonical sorted list of distinct `TableSchemaClassId`s. `TableProfileAssignmentV1` is a sorted total map from every such class to `Compact | Aligned | DenseAligned`. The primary baseline is all-compact. The study matrix contains that baseline plus exactly one-class-at-a-time substitutions to each other eligible profile; it does not enumerate the exponential unrestricted Cartesian product. A class/profile pair is eligible only when its schema and borrowed-view rules support that profile. A module with no classes uses the canonical empty map, its encoding/execution cell proceeds normally, and only table-specific metric records use `NotApplicable(NoLogicalTable)`.

For each substituted class, compare encoded bytes, validation work, borrowed-access latency, random lookup, sequential scan, memory residency, owned-decompression bytes, and mmap behavior while every other class remains compact. No candidate may require whole-module owned decompression.
Columnar or packed layouts may run only as explicitly nonselectable prototypes until an approved module-format decision assigns a wire discriminator. They cannot contribute to the Gate 0 table-profile selection under format 1.0.

All selectable profiles must decode to byte-identical canonical logical row serialization and matching `ExecutableId`. Report encoded bytes, validation work, borrowed-access latency, random lookup, sequential scan, memory residency, owned-decompression bytes, and mmap behavior. No candidate may require whole-module owned decompression.

### 4.4 Predecode variants

The primary comparison disables optional predecode for all encodings. A secondary experiment enables the same bounded semantic predecode fields for every candidate: internal opcode tag, operand references, block/table index, interpreter slots, and simultaneous-copy schedule. Report construction work, retained bytes, first-result impact, warm throughput, and eviction/rebuild behavior.

An encoding is not selected on a result that requires richer predecode than its peers.

### 4.5 Heavy-tier feasibility

A non-shipping heavy-tier experiment MAY run on representative medium/large Vix and Fable programs. `HeavyTierStudyKeyV1` is `{ corpus_id, semantic_module_digest, heavy_closure_request_id, target_profile_id, measurement_protocol_id }`. Status is `NotRun(OwnerDidNotAuthorize) | NotApplicable(TooSmall | ConsumerExcluded) | Blocked(ProfilePolicyDeferred { policy_key, major } | ProfilePolicyRejected { policy_key, major } | MissingHeavyClosure { heavy_closure_request_id, class = UnsupportedDependency | MissingDescriptor | BuildConfiguration }) | Unavailable(TargetInfrastructure) | Rejected(result_id) | Completed(result_id)`. `HeavyTierResultV1` uses closed profile resolution `Produced { profile_id, manifest_digest } | NotProduced(ProfilePolicyDeferred { policy_key, major } | ProfilePolicyRejected { policy_key, major } | MissingHeavyClosure { heavy_closure_request_id, class })` and build provenance `Produced(digest) | NotProduced(TargetInfrastructure | HardGateBeforeBuild | NoNativeExecution)`, plus shared typed outcomes/metrics/samples/transcripts/provenance and heavy metrics. Heavy evidence cannot promote a shipping tier in Gate 0.

## 5. Target profiles

The Gate 0 decision-required matrix is:

| Architecture | Required direct profile | Required evidence |
|---|---|---|
| x86-64 | at least one available native ABI/platform | build, execute, publish, unwind, applicable security registration, rollback, retire |
| AArch64 | at least one available native ABI/platform | build, execute, publish, unwind, applicable security registration/cache synchronization, rollback, retire |
| portable | no-exec | interpreter-only with no executable-memory dependency |

Candidate additional qualification profiles include Linux System V and Windows Win64 x86-64; Linux AAPCS64, Darwin arm64/arm64e, and Windows ARM64 AArch64; and portable WASM where supported. Their target profiles still model all architecture-v6 ABI/security obligations, but an unavailable additional profile does not block the architecture-wide instruction-encoding decision. Native policy for that profile remains unselected/interpreter-only until its complete direct lifecycle evidence exists.

Target-profile evidence is a closed vector keyed by `build | execute | publish | rollback | unwind | security-register | cache-synchronize | retire`; each entry is `Passed(evidence_digest) | NotApplicable(reason) | Unavailable(reason) | Failed(error)`. Applicability is a total function of ABI/platform/security policy. A native target cell is complete only when every applicable capability passes, including injected rollback and AArch64 instruction-cache synchronization. The required set contains directly executed x86-64, AArch64, and portable no-exec profiles.

### 5.1 Complete cell key and status

The experiment matrix is the reduced cross-product keyed by:

```text
CellKeyV1 {
  authority = Baseline | HelperStudy { study_authority_digest } | Final { final_authority_id },
  corpus_id, semantic_module_digest, runtime_closure_request_id,
  helper_placement = None | Some { candidate_id, alternative = OrdinarySSA | Primitive | Helper },
  encoding = E16 | E32 | EV,
  execution = A | B | C,
  table_profile_assignment: TableProfileAssignmentV1,
  predecode = disabled | equal_information,
  target_profile_id,
  measurement_protocol_id,
}
```

`runtime_closure_request_id = BLAKE3("weavy.runtime-closure-request.v1\0" || CanonicalPhon(RuntimeClosureRequestV1))`, where the request record contains, in order, composition-family ID, execution variant, sorted required feature identities/versions, sorted `PolicyRequirement`s, target profile ID, instrumentation policy, and study/final authority ID when present. It exists before profile generation.

Each fully qualified key has one status: `Required`; `NotApplicable(NoNativeExecution | NoPredecodeStudy | ExecutionVariantUnavailableByDesign)`; `Blocked(ProfilePolicyDeferred { policy_key, major } | ProfilePolicyRejected { policy_key, major } | HelperDecision | FormatAuthority | MissingSemanticDescriptor)`; `Unavailable(TargetInfrastructure | SecurityFacility | UnwindFacility)`; `Rejected(result_id)`; or `Completed(result_id)`. Status/outcome invariants are exact: `Completed(id)` references one result with `Conclusive | Inconclusive | InfrastructureFailure`; `Rejected(id)` references one result with `HardGateRejected`; every other status references no result. `Required` is pre-execution only and cannot remain at review; `Blocked`/`Unavailable`/`NotApplicable` satisfy row presence but never selection. Duplicate keys or result IDs reject the manifest.

`result_id = BLAKE3("weavy.gate0.result.v1\0" || CanonicalPhon(Gate0ResultV1-with-result_id-zeroed))`. `Gate0ResultV1` contains fields in exactly this order: `result_id`; complete `CellKeyV1`; outcome; `corpus_digest`; `semantic_module_digest`; `image_identity = Produced(image_id) | NotProduced(EncodeFailed | ContainerPhase0Rejected | TargetInfrastructure)`; `executable_identity = Produced(executable_id) | NotProduced(NoImage | AdmissionRejected { phase, error_class } | SemanticDecodeUnavailable)`; profile resolution; target profile; experiment manifest digest; build provenance; repository revision digests; ordered measurement-method IDs; ordered metric records; ordered raw samples; ordered `TranscriptRefV1` records; admission/compilation/invocation limits; environment/provenance; and report-generator version. `TranscriptRefV1` is `{ schema_id, transcript_digest, ordered_event_count, storage_object_digest }`; its digest is over canonical PHON transcript bytes under `schema_id`, and list order is observation order. Metric records and closed applicability reasons remain as specified below.

`transcript-schema-registry.styx` versions every transcript schema and records its PHON root. The completeness validator enforces the status/outcome mapping, unique IDs, exact manifest/provenance links, sample/transcript order, resolvable transcript storage, and reproducible report generator.

## 6. Measurement protocol

### 6.1 Reproducibility and frozen protocol

Before Task 9, `decision-policy.styx` and `experiment-manifest.styx` are immutable and schema-validated. Every metric declares units, measurement method, estimator, paired contrast, and paired 95% CI procedure. Default scalar latency/bytes/RSS uses the per-block paired median difference/ratio and BCa bootstrap over complete blocks; throughput/count rates use the per-block paired arithmetic-mean rate contrast and the same BCa procedure. p95 latency uses the Harrell-Davis 0.95 quantile within each cell/block, paired blockwise difference/ratio, then BCa bootstrap over complete blocks. Invocation break-even is recomputed in each paired bootstrap replicate as `compile_cost_delta / max(per_invocation_execution_savings, 0)`; nonpositive savings yield `+infinity`, and the reported selection bound is the 95% upper percentile. Construction-cost break-even is recomputed analogously as `construction_cost / max(per_invocation_predecode_savings, 0)`. The policy records resampling seed/count, zero/negative-denominator rule, warmup termination, initial/maximum complete-block counts, randomized block order, cold-cache procedure, quiescence predicate/timeout, retry trigger/cap, and outlier policy. Default warmup terminates after five consecutive batches whose medians vary by at most 2%, with a declared maximum; default measured count is 30 complete blocks and may increase only to the predeclared maximum when the CI-width trigger fires. Every original/retry sample remains ordered; none is deleted. A quiescence timeout is a typed failed sample.

Each result records repository revisions, compiler version, target triple, CPU model/features, OS version, power/thermal policy when observable, build profile, feature closure, environment, corpus digest, image digest, executable digest, backend/stencil epochs, exact protocol ID, and raw samples. Fresh processes are used for load/initialization/RSS and first-result metrics. Candidate order is randomized within complete blocks. Cold-filesystem and warm-cache observations are separate; any cache-control mechanism is documented. Instrumentation is identical and its disabled overhead is measured.

Retries occur only when a predeclared infrastructure failure or relative CI half-width above the metric threshold fires; the cap and retained failed attempts are fixed in the manifest. An anomalous value alone never triggers deletion or retry. Results exceeding the cap remain `Completed(result_id)` whose `Gate0ResultV1.outcome` is `Inconclusive(reason)` or `InfrastructureFailure(class)` and cannot support selection.

### 6.2 Required measurements

For every applicable corpus/candidate/variant report:

- semantic operation count;
- program bytes, directory bytes, schema bytes, table bytes, metadata bytes, total image bytes;
- producer encode wall/CPU and peak RSS;
- container phase-0 decode/integrity wall/CPU;
- total admission wall/CPU by phase;
- admission allocations, peak scratch, retained `TrustedFacts` bytes;
- optional predecode work and retained bytes;
- baseline RSS before module creation;
- peak construction/admission/compile RSS;
- retained RSS after first result and cache quiescence;
- first uncached interpreted result;
- first native result;
- compilation latency broken into legalization, target lowering, allocation, physical-form selection, layout, ordinary emission, stencil lookup/copy/patch, validation, publication, and registrations;
- generated code, read-only data, writable data, metadata, and embedded stencil bytes;
- semantic operations and program bytes per compile unit;
- warmed end-to-end throughput and steady-state interpreter/native execution;
- break-even invocation count using measured compile and execution deltas;
- random block seek and sequential instruction scan;
- table random lookup and sequential scan;
- source-map, pollpoint, and suspension-site costs;
- malformed/nonminimal fuzz throughput;
- clean-build cost and stripped application text/rodata/data;
- transitive dependency closure and load/initialization latency;
- Vix partition choice impact for Vix corpus entries.

Report distributions and raw samples, not only means. Tail latency is mandatory for compile queues and heavy-tier experiments.

### 6.3 Allocation and resource discipline

Measurements MUST distinguish semantic VM allocation from compiler/admission tooling allocation. All admission and compilation limits used by a run are stored with the result. A candidate that exceeds limits is a typed result, not a crashed or silently omitted sample.

Every result also records success, typed limit rejection, typed unsupported/unavailable state, or oracle mismatch. Missing cells are a manifest error. Compiler/admission peak memory is measured independently from process baseline and module image mappings; retained measurements occur only after declared quiescence and cache state are recorded.

## 7. Correctness and rejection oracles

### 7.1 Determinism

For every candidate/profile:

```text
semantic module
 -> encode
 -> load/admit
 -> canonical semantic re-emit
 -> encode with same physical profile
```

The first and second images MUST be byte-identical. Repacking to another table profile MUST preserve canonical semantic bytes and `ExecutableId` while changing `ImageId` when physical bytes change.

### 7.2 Malformed images

Mutation corpora cover:

- bootstrap truncation and trailing bytes;
- bad magic/version/endian/reserved bytes;
- mismatched file length and payload integrity tag;
- directory offset/length overflow;
- zero/non-power-of-two/mismatched alignment;
- overlap, noncanonical ordering, duplicate/missing singleton sections;
- unknown required and ignored unknown optional sections;
- malformed PHON directory/schema closure;
- missing/unequal schema IDs;
- malformed compact/aligned/dense ranges;
- bad count/stride/profile;
- nonminimal/overlong instruction forms;
- impossible instruction lengths;
- invalid block/value/type/constant/table/callable IDs;
- duplicate definitions and bad dominance;
- ownership double-use/leak;
- invalid borrow/suspension live set;
- relation witness failure;
- helper/import/version mismatch;
- `ClaimedExecutableId` mismatch;
- resource/pollpoint undercoverage.

Every rejection has a stable typed phase and error class. No mutation may panic, read out of bounds, allocate from an unchecked count, or reach execution.

### 7.3 Interpreter/native parity

A, B, and C compare:

- returned values or canonical `TaskFault`;
- exact fault `InstId` and source attribution;
- semantic resource-event transcript;
- external adapter/import call order and partial effects;
- table/automaton events;
- wire/primitive yield transcript;
- safepoint/pollpoint and continuation transcript;
- abandonment and cleanup transcript.

Native publication negative paths inject failure after each registration/protection step and prove no entrypoint exposure, complete rollback, and no leaked mapping/reference. Retirement proves quiescence across execution, unwind, entrypoint, image, and binding leases.

### 7.4 Consumer equivalence

Gate 0 uses frozen semantic modules and differential oracles but does not redirect public entry points. The oracle compares experimental execution with the existing production path for the same input. Any mismatch is a blocker; it is not averaged into performance results.

Consumer differential inputs are immutable and identified by digest. For stateful/incremental cases, the oracle records the complete ordered input/edit/request sequence and compares every intermediate externally observable result, not only the final value. Failure injection covers every declared allocation, adapter partial-write, async acceptance/completion, cleanup, publication, and retirement transition that can affect the transcript. The mandatory lifecycle schedule matrix includes: supply/response racing the parked-state publication gate, proving a yield is either visible-before-resume or atomically suppressed; abandonment against every `ParkedInput`/`ResumingInput`/`AwaitingAsync`/`EarlyResponse`/`ParkedAsync`/`ResumingAsync` state; repeated abandonment during `Cleaning`/`Faulting`/`Abandoning` with no redispatch; already-ready input validation failure; malformed first async response before continuation preparation; and join observation before, during, and after required-affinity cleanup. Interpreter and native transcripts must match for every schedule.

### 7.5 No-exec negative oracle

The portable no-exec closure is built from an interpreter-only manifest whose dependency, linked-symbol, and section denylist rejects native compiler, stencil, executable allocator, protection-changing, instruction-cache publication, unwind-registration, CFG/BTI/PAC publication, and raw-entrypoint APIs. The target profile records the exact forbidden symbol patterns and executable section types.

Run required corpus load/admit/interpret journeys in a fresh process under platform enforcement that denies executable mappings and executable protection transitions where available; otherwise trace every mapping/protection/publication call and fail on a forbidden attempt. Also inspect the stripped binary and dependency closure against the denylist and assert native initialization symbols were neither linked nor run. Pass requires: complete interpreter transcripts; zero executable mappings/protection transitions/publication registrations; zero forbidden symbols/sections/dependencies; and a typed `NotApplicable(NoNativeExecution)` status for B/C cells. A successful interpreter result alone is insufficient.

## 8. Decision rules

### 8.1 Hard conformance gate

A candidate is ineligible if it fails any of:

- deterministic canonical encoding;
- fixed maximum instruction length;
- canonical shortest forms and nonminimal rejection;
- container phase-0 impossible-length detection;
- direct block entry without whole-program decode;
- bounded verifier state and admitted memory/work limits;
- streaming interpreter without mandatory predecode;
- no whole-module owned table decompression;
- complete consumer semantic oracle;
- malformed-input safety;
- interpreter/native transcript parity;
- complete ordinary encoder for every native physical form;
- publication rollback and quiescent retirement;
- x86-64, AArch64, and no-exec conformance evidence.

### 8.2 Quantitative decision policy

`decision-policy.styx` names every load-bearing `CorpusId`: all `traced-production` entries plus the owner-designated production roots required for a consumer/profile claim. Synthetic boundaries never receive performance weight, though they remain hard conformance gates. No cross-consumer weighted average selects a candidate. Comparisons are paired within corpus/block and reported per corpus, frequency class, size class, target, and profile.

A claimed benefit is **material** only when the complete 95% paired CI clears the predeclared minimum effect: 5% for latency/throughput/compile CPU, 3% for image/code bytes, 8% for peak/retained RSS, or the explicit stricter per-metric value frozen in the policy. A regression is disqualifying for a claimed policy when its CI shows more than 5% latency/throughput loss, 5% byte growth, or 10% RSS growth on any load-bearing cell unless `decision.md` records an owner-approved exception with the exact benefit/risk and limits the selection scope. `Dominate` means no material improvement on any load-bearing objective plus at least one disqualifying regression or footprint-budget breach.

Among hard-gate survivors, report the Pareto frontier over image/program/table bytes; admission CPU/wall/allocations/retained facts; first interpreted/native result and compile latency; warmed throughput; random seek; malformed validation; producer cost; metadata; dependencies/build/binary footprint; and reviewed parser/verifier/encoder/unsafe-state counts. There is no hidden aggregate score. If candidate differences remain inside uncertainty on every load-bearing objective, select none and preserve all evidence.

### 8.3 Compact/native/stencil policy

Compute A→B and B→C separately. B is selectable for a `(target, size class, profile)` only when its measured invocation break-even upper 95% bound is below the frozen invocation-distribution p50 for every load-bearing cell in that scope, it has no disqualifying first-result/footprint regression, and interpreter fallback remains complete. C additionally requires normalized byte/metadata equality for every substituted fragment, a material compile-latency or retained-compiler-memory benefit on load-bearing tiny or median cells, stencil binary/manifest growth within the frozen 5% stripped-text/rodata budget, and reliable B fallback. A size threshold is the smallest observed semantic-operation/program-byte boundary whose adjacent strata both satisfy these predicates; interpolation or post-hoc threshold tuning is forbidden.

### 8.4 Table-profile policy

For each logical `TableSchema` class, compact is baseline. Aligned or dense-aligned is selectable only when all semantic/identity/lease gates pass and either random lookup or sequential scan improves materially on every load-bearing table cell using that class, while total image plus retained RSS does not regress beyond 5% and validation does not regress beyond 10%. Otherwise compact remains selected. Decisions are schema-class based, not consumer named.

### 8.5 Predecode policy

Equal-information predecode is retained for a `(encoding,size class,target)` only when warmed throughput or p95 latency improves materially on every load-bearing cell in scope, its construction-cost upper 95% bound is recovered before invocation-distribution p50, retained bytes stay within 5% of admitted program plus `TrustedFacts`, and failure/cancellation/race/eviction/rebuild oracles pass. Otherwise it is disabled. It never participates in correctness or semantic identity.

### 8.6 Heavy tier

Gate 0 records feasibility only. It cannot authorize shipping or automatic promotion. Any later decision requires migrated programs, stable hotness, and positive measured net saved work after all compilation, memory, queue, cache, and code-size costs.

## 9. Sequencing

### Task 1: Freeze executable semantic inputs

**Files:** `weavy/docs/content/weavy-bytecode-specification.md`, `weavy/docs/content/weavy-opcode-catalog-v1.md`, `weavy/docs/content/weavy-relation-contract-v1.md`, `weavy/docs/content/weavy-runtime-profiles-v1.md`

- Extract every frozen clause from the normative specification.
- Expand the exercised opcode/relation/capability set into complete `OperationDescriptorV1` and relation records; generate and independently reproduce canonical semantic, profile, `LegalProgramV1`, and identity vectors.
- Generate stable feature IDs from the approved registry; remove unexercised semantic features from concrete manifests while retaining non-feature machine infrastructure.
- Create `owner-policy-decisions.styx` with one row `{ policy_key: PolicyKeyV1, major: u16, max_minor: u16, descriptors: ordered list<PolicyDescriptorV1>, compatible_minor_digests: list<bytes[32]>, affected_profiles, affected_features, decision = Approved | Rejected | Deferred, approval_reference }`. Descriptor history is complete and append-only through `max_minor`; duplicate key/major with unequal history rejects. `Approved` emits identical `PolicyVersion`; `Deferred` and `Rejected` produce distinct typed blockers and no manifest.
- Review that no consumer engine or open registry crossed the firewall.
- Commit the executable specification separately from experimental implementations.

Before Task 1A, `producer-survey.styx` enumerates every source construct from the pinned producer revisions, its current lowering class, frequency evidence, and whether ordinary core lowering exposes a plausible helper-placement candidate. Its closed sorted candidate output is the sole input to Task 1A; a later discovered candidate requires a new survey version and refreeze.
**Gate:** Task 1A and Task 2A may begin once the canonical-semantic schema framework, producer survey, `LegalProgramV1`, relation complexity, identity-vector procedure, and every required owner-policy row are approved. Task 1A may freeze candidates and provisional semantic descriptors before physical roots exist, but its final study manifests/IDs remain blocked on Task 2A. Tasks 2 (container oracle), 1A, and 2A may proceed independently where inputs permit; Tasks 3+ and all shipping semantic-registry/candidate-image work remain blocked until both Task 1A and Task 2A approve their authorities.

### Task 1A: Authorize helper-placement candidates

For every candidate in the producer survey, freeze identical source workload/input sequence and one exact observable value/fault/effect/resource/suspension/consumer-oracle contract. Approve a closed three-alternative semantic study set: ordinary SSA plus existing facilities, a provisional primitive descriptor, and a provisional bounded-helper descriptor. Task 1A records descriptor semantics and candidate identities first; after Task 2A assigns experimental roots/discriminators, `helper-study-authority.styx` binds each descriptor to its domain-separated `StudyFeatureId`, exact schema root, handlers, versions, and immutable study-only profile manifest/ID. Study admission then accepts them only under matching `Gate0StudyPolicy`.

**Gate:** every surveyed candidate is `Deferred` or has complete semantic descriptors; dependent Tasks 3+ remain blocked until Task 2A supplies physical authority and Task 1A finalizes study manifests/IDs. Once both gates pass, Tasks 3-9 may build and measure alternatives. No shipping artifact may contain an undecided helper.

### Task 2: Freeze the PHON container oracle

**Files:** section 2 of `weavy/docs/content/weavy-bytecode-specification.md`, focused `weavy-phon` tests

- Reconcile implementation constants and terminology with the normative bootstrap table.
- Add canonical layout, overlap/order/cardinality, reserved-byte, byte-order marker, schema-ID, and identity oracles.
- Generate known-good binary fixtures from the writer; never hand-edit generated files.
- Prove owned and borrowed readers enforce the same structural authority.
- Prove alternate table profiles preserve canonical semantics.

**Gate:** no program encoding may redefine the bootstrap, directory, schema closure, section namespace, or physical integrity field.

### Task 2A: Approve the nonshipping experimental semantic profile

**Files:** `weavy/gate0/experimental-module-format.styx`, canonical-semantic schema roots, focused compatibility vectors

- Assign an explicit experimental format/version discriminator that cannot be mistaken for frozen 1.0 required payloads.
- Record exact manifest, schema-closure, program, constants, and table PHON root IDs; closure framing/membership/order; required-section flags/profile/count/stride rules; and upgrade/promotion behavior.
- Use one semantic profile for E16/E32/EV; only the program root/encoding discriminator may differ as explicitly recorded.
- Prove frozen 1.0 readers reject or safely classify experimental images and experimental readers do not reinterpret legacy custom sections as typed semantic roots.
- Obtain explicit module-format owner approval.

**Gate:** Task 3 may build canonical semantic modules, but Task 4 cannot emit candidate images until this authority is approved. Promotion of a selected candidate requires a later owner decision assigning the shipping format/root; Gate 0 experimental IDs do not become shipping authority automatically.

### Task 3: Build semantic module corpus

**Files:** `weavy/gate0/corpus-manifest.styx`, producer-specific corpus adapters

- Record exact repositories/revisions/commands and typed expected transcripts.
- Export canonical semantic modules and `ProducerLoweringManifestV1` before physical instruction encoding.
- Prove every corpus source construct maps to semantic keys/instructions/features or a typed unsupported case; reject hidden consumer engines/callbacks.
- Classify production frequency honestly and freeze semantic digests/provenance.
- Validate each module with the reference semantic oracle and independently derive exact used feature declarations.

**Gate:** all three encodings consume byte-for-byte identical canonical semantic inputs, and every required corpus cell has approved owner policy/helper/format prerequisites.

### Task 4: Implement the three candidate codecs

**Files:** separate candidate modules under `weavy-gate0`, candidate tests, `weavy/docs/content/weavy-program-encoding-candidates.md`

- Implement E16, E32, and EV behind one answer-neutral codec interface.
- Generate block directories and canonical `InstId` maps from the same module.
- Add deterministic round-trip and malformed/nonminimal tests.
- Add bounded container phase-0 scan and direct-block-entry oracles.
- Run the complete corpus through encode/decode/admit/interpret.

**Gate:** a candidate with any semantic mismatch or hard-conformance failure is removed before performance comparison.

### Task 5: Implement answer-neutral admission and interpreter instrumentation

- Ensure admission records per-phase work, allocations, scratch, retained facts, and typed failure.
- Ensure the base plan and streaming interpreter are independent of candidate-specific predecode.
- Emit typed semantic/resource/suspension/adapter transcripts.
- Add mutation/fuzz entry points using the production decoder and admission path.

**Gate:** instrumentation must not change semantic identity or give one candidate extra derived data.

### Task 6: Implement complete ordinary native baselines

- Consume the owner-approved `LegalProgramV1` schema and total per-feature lowering coverage from Task 1.
- Implement complete ordinary encoders for the admitted x86-64 and AArch64 physical forms.
- Compile to `UnboundNativeArtifact`, then link with `BoundProgram`; test separate compilation/link cache keys, binding leases/guards, rollback, publication generations, and retirement.
- Add ABI, branch-relaxation, frame, unwind, security, publication rollback, and retirement oracles.
- Verify native stack envelopes and interpreter fallback.

**Gate:** Task 6 does not start without approved `LegalProgramV1`; copy-and-patch does not start until B passes parity and lifecycle oracles independently.

### Task 7: Add copy-and-patch emission

- Extract immutable build-time stencils with complete manifests.
- Validate manifest operands, locations, defs/uses, fixed registers, flags, clobbers, patches, branch behavior, and security forms.
- Substitute only after finalized physical-form selection.
- Compare normalized bytes and metadata against ordinary emission for every substituted fragment.
- Record fallback causes and coverage.

**Gate:** any selection/length/layout difference is removed from B→C and classified as a separate experiment.

### Task 8: Run table-profile and predecode studies

- Run compact/aligned/dense-aligned on eligible logical tables.
- Validate identical canonical logical bytes and `ExecutableId`.
- Measure access, validation, RSS, and mmap behavior.
- Run equal-information optional predecode across E16/E32/EV.
- Exercise cache failure, cancellation, concurrency, and eviction.

### Task 9: Run controlled measurements

- Validate `decision-policy.styx`, complete cell cross-product/statuses, result/transcript schemas, and immutable experiment manifests before running any sample.
- Run fresh-process cold/warm randomized complete blocks across required target profiles with the exact frozen warmup/sample/retry/quiescence protocol.
- Store every ordered raw sample, typed failure, trace, and transcript in PHON; retries never delete prior attempts.
- Run the no-exec negative oracle and completeness/provenance validator.
- Regenerate the report from typed results only.

**Gate:** no decision review begins with missing required cells, unresolved blocked prerequisites, inconclusive claimed advantages, broken provenance, or a report that does not reproduce from raw data.

### Task 9A: Decide helper placement and refreeze authority

For each authorized helper candidate, compare the ordinary-SSA, provisional-primitive, and provisional-helper alternatives using the frozen result/transcript schema and the same materiality, regression, footprint, boundedness, and security predicates as every other selection. A primitive/helper is selectable only when every load-bearing cell is conclusive, it introduces no disqualifying regression or anti-engine violation, it has a complete portable reference algorithm and final `OperationDescriptorV1`, and the owner approves `helper-placement-decision.md`. Otherwise ordinary SSA is selected or the candidate is `Deferred`.

After the decision, regenerate canonical schemas, feature registries, profile manifests, `LegalProgramV1` roots, and identity vectors using only selected features. Compute `final_authority_id = BLAKE3("weavy.gate0.final-authority.v1\0" || CanonicalPhon(FinalAuthorityProjectionV1))`; the projection contains selected helper decisions and regenerated semantic/profile/root identities but excludes the ID itself, final cell keys/results, and enclosing manifest digest. Freeze `final-authority-experiment-manifest.styx` with that independent ID and a separate full manifest digest; map provisional `HelperStudy` keys/results to distinct `Final { final_authority_id }` keys, name exact reruns, and forbid inference where identity-sensitive work or bytes changed. Provisional IDs are forbidden in final artifacts.

**Gate:** all helper candidates are `SelectedOrdinarySSA`, `SelectedPrimitive`, `SelectedHelper`, or `Deferred`; no selected encoding/profile/policy or production image depends on `Deferred`. Task 10 cannot begin until the final-authority manifest is complete and immutable, every required rerun result is linked, and the regenerated report reproduces the claimed conclusions.

### Task 10: Review and decide

- Run semantic/security review of all surviving candidates.
- Produce hard-gate table and Pareto frontiers.
- State candidate-specific regressions and complexity.
- Select encoding/policies or explicitly record insufficient evidence.
- Obtain owner approval of `decision.md` before Gate 1A or any public consumer cutover.

The owner-approved decision freezes the chosen program schema/version, exact supported target policy, compact/native/stencil threshold policy if any, table-profile classes, and predecode retention policy. Any cell lacking hard-gate evidence remains unselected even if another platform selects the same candidate.

## 10. Stop gates

Stop Gate 0 and request a new architecture/version decision if evidence requires:

- changing bootstrap/header/directory/schema-closure authority;
- adding an open consumer dialect, helper, relation, or validator registry;
- persisting native code or process pointers;
- making predecode mandatory for correctness;
- requiring whole-module owned table decompression;
- changing semantic identities to suit an encoding;
- allowing native availability to change semantics;
- weakening admission limits or malformed-input rejection;
- migrating a public consumer entry point before an encoding decision;
- hiding a consumer evaluator/parser/codec behind an opcode, helper, table predicate, import, or stencil;
- accepting B/C differences in selection, length, layout, normalized bytes, or metadata as copy-and-patch evidence.

A negative result is preserved and reported. Gate 0 may validly end with no selected native tier, compact tables only, no retained predecode, or no heavy tier. It may not end with an instruction encoding selected from incomplete corpus or lifecycle evidence.

## 11. Gate 0 completion checklist

- [ ] Cross-document authority hierarchy approved; no unresolved conflict or delegated descriptor.
- [ ] Canonical semantic/profile/`LegalProgramV1` schemas, roots, enum tags, and independent identity vectors approved.
- [ ] Exercised opcode/relation/capability registry complete; every concrete profile feature has corpus or negative-oracle coverage.
- [ ] Every owner-policy prerequisite has an approved/rejected/deferred row; no selected cell depends on `Deferred`.
- [ ] Helper placement study and owner decision complete for every exercised candidate.
- [ ] Frozen PHON 1.0 container oracle and explicit nonshipping experimental-format authority approved.
- [ ] Corpus and producer-lowering manifests complete with provenance, construct coverage, exact used features, and honest frequency classes.
- [ ] E16, E32, and EV each have complete deterministic/malformed hard-gate status; every failed candidate is recorded.
- [ ] All survivors pass identical consumer, feature-declaration, suspension/abandonment, cleanup, resource, and interpreter/native transcripts.
- [ ] Streaming interpreter works without predecode for every survivor.
- [ ] `LegalProgramV1` lowering coverage is total; ordinary x86-64 and AArch64 compile/link/publication/retirement oracles pass.
- [ ] Copy-and-patch normalized output matches ordinary emission where studied.
- [ ] Portable no-exec negative oracle passes with zero forbidden link/map/protection/publication activity.
- [ ] Table-profile and equal-information predecode studies satisfy frozen predicates or remain baseline-disabled.
- [ ] Complete cell matrix, frozen decision/experiment protocols, typed raw results/transcripts, provenance validation, and generated report are checked in.
- [ ] Pareto, quantitative regression/materiality, complexity, and security review complete.
- [ ] Owner approves `decision.md` or explicit no-selection; only then may Gate 1A planning authorize a Vix production cutover.
