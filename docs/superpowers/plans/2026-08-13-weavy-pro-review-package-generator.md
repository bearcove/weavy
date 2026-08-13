# Weavy Pro Review Package Generator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a deterministic, manifest-driven `weavy-review-package` CLI and use it to produce the first implementation-planning review release.

**Architecture:** Add one focused Rust workspace binary with typed package/checkpoint configuration, repository snapshot collection, staged entry generation, referential/security validation, canonical ZIP emission, and detached release artifacts. Keep checkpoint policy in a checked-in Styx configuration and generated control documents in disposable staging/output directories. The canonical machine model uses Facet types and `facet-json` for JSON; no handwritten JSON or shell-generated archive metadata.

**Tech Stack:** Rust 2024, Facet reflection/JSON, figue layered CLI configuration, Styx checkpoint configuration, `blake3`, SHA-256, pinned Rust ZIP writer, cargo-nextest, existing workspace and Git CLI.

## Global Constraints

- Follow `docs/superpowers/specs/2026-08-13-weavy-pro-review-packages-design.md` exactly.
- The canonical ZIP contains one top-level directory and no explicit directory entries.
- Never embed the ZIP digest inside the ZIP.
- Never handwrite JSON; derive Facet types and serialize with `facet-json`.
- Never include symlinks, hard links, devices, absolute archive paths, `..`, duplicate paths, or case-folding collisions.
- Package only explicit manifest entries; no recursive directory copying.
- Proprietary inputs require an explicit per-release external-AI upload approval record.
- Active control references may not contain host paths or ephemeral URIs; inert classified evidence may.
- Run Rust tests with `cargo nextest run`, never `cargo test`.
- Do not modify Weavy bytecode/runtime semantics as part of this plan.

---

## File Structure

- Create `review-package/Cargo.toml` — binary crate manifest and pinned packaging dependencies.
- Create `review-package/src/main.rs` — CLI command dispatch only.
- Create `review-package/src/model.rs` — Facet data model for checkpoint, manifest, release, evidence, findings, and security dispositions.
- Create `review-package/src/config.rs` — Styx/figue configuration loading and canonical validation.
- Create `review-package/src/snapshot.rs` — canonical Git/worktree snapshot collection and source-set identity.
- Create `review-package/src/stage.rs` — explicit source collection, generated views, and root-document rendering.
- Create `review-package/src/validate.rs` — referential integrity, budgets, paths, archive-entry, and security gates.
- Create `review-package/src/archive.rs` — deterministic canonical ZIP and detached release artifacts.
- Create `review-package/src/evidence.rs` — reproducible command evidence and raw-output metadata.
- Create `review-package/src/security.rs` — path/content scanning and manual dispositions.
- Create `review-package/tests/model.rs` — canonical identity and serialization contracts.
- Create `review-package/tests/snapshot.rs` — clean/dirty/untracked multi-repository snapshot contracts.
- Create `review-package/tests/archive.rs` — byte-identical ZIP and unsafe-entry rejection contracts.
- Create `review-package/tests/validation.rs` — control-reference, budget, secret, and manifest closure contracts.
- Create `review-package/tests/release.rs` — end-to-end release generation oracle.
- Create `review-packages/implementation-plan-v1.styx` — exact first-checkpoint input and inclusion policy.
- Create `review-packages/security-policy-v1.styx` — scanner rules and required dispositions.
- Create `review-packages/prior-review/package-design-r1/REVIEW-REPORT.md` — exact first Pro report.
- Create `review-packages/prior-review/package-design-r1/OWNER-DISPOSITION.md` — accepted corrections and commits.
- Create `review-packages/prior-review/package-design-r1/APPLIED-CHANGES.md` — line-item correction ledger.
- Create `review-packages/prior-review/package-design-r2/REVIEW-REPORT.md` — exact second Pro report.
- Create `review-packages/prior-review/package-design-r2/OWNER-DISPOSITION.md` — final correction disposition.
- Create `review-packages/prior-review/package-design-r2/APPLIED-CHANGES.md` — final applied changes.
- Modify `Cargo.toml` — add `review-package` workspace member and shared dependencies.
- Modify `.gitignore` — ignore deterministic staging/output directories, not checked-in package policy/history.

### Task 1: Canonical package data model

**Files:**
- Modify: `Cargo.toml`
- Create: `review-package/Cargo.toml`
- Create: `review-package/src/main.rs`
- Create: `review-package/src/model.rs`
- Test: `review-package/tests/model.rs`

**Interfaces:**
- Produces: `Checkpoint`, `Question`, `Decision`, `RepositorySnapshot`, `PackageEntry`, `PackageManifest`, `ReleaseDescriptor`, `EvidenceRecord`, `SecurityDisposition`.
- Produces: `canonical_json_bytes<T: Facet>(value: &T) -> Result<Vec<u8>, ModelError>` and domain-separated identity functions.
- Consumes: no earlier task interfaces.

- [ ] **Step 1: Add the workspace crate and dependencies**

Add `review-package` to workspace members. Add pinned workspace dependencies for `facet`, `facet-json`, `figue`, `styx`, `sha2`, and the chosen ZIP crate. Inspect current crate releases with `cargo info`; do not guess versions. Avoid dependencies that introduce `syn` when an established Facet alternative exists.

- [ ] **Step 2: Write canonical model tests**

Cover:

```rust
#[test]
fn question_text_participates_in_checkpoint_identity() { /* differing text => differing bytes */ }

#[test]
fn source_set_id_is_sorted_and_domain_separated() { /* input order irrelevant */ }

#[test]
fn release_descriptor_does_not_enter_archive_manifest() { /* detached identity */ }
```

Use exact expected digest vectors for one minimal checkpoint and source set.

- [ ] **Step 3: Verify tests fail**

Run:

```text
cargo nextest run -p weavy-review-package --test model
```

Expected: compile failure because model types/functions do not exist.

- [ ] **Step 4: Implement the model and canonical JSON boundary**

Use Facet-derived enums/structs. JSON serialization is centralized in one function. Domain-separate identities:

```text
weavy.review.worktree-snapshot.v1\0
weavy.review.source-set.v1\0
weavy.review.package-entry.v1\0
weavy.review.release.v1\0
```

`PackageManifest` omits itself by type construction; `ReleaseDescriptor` is detached and cannot be added as an archive entry.

- [ ] **Step 5: Run focused tests and check**

```text
cargo nextest run -p weavy-review-package --test model
cargo check -p weavy-review-package --all-targets
```

Expected: all pass.

- [ ] **Step 6: Commit**

```text
git add Cargo.toml review-package
git commit -m "feat: define review package model"
```

### Task 2: Checkpoint configuration and control-document contract

**Files:**
- Create: `review-package/src/config.rs`
- Create: `review-packages/implementation-plan-v1.styx`
- Test: `review-package/tests/validation.rs`

**Interfaces:**
- Consumes: Task 1 model types.
- Produces: `load_checkpoint(path: &Path) -> Result<Checkpoint, ConfigError>`.
- Produces: validation that question/decision/output text is canonical and limits are nonzero.

- [ ] **Step 1: Write configuration tests**

Test a complete checkpoint, duplicate question IDs, inconsistent package revision, missing limits, and active `/Users`, `/tmp`, or `local://` references.

- [ ] **Step 2: Verify failures**

```text
cargo nextest run -p weavy-review-package --test validation -E 'test(config)'
```

Expected: missing loader/validators.

- [ ] **Step 3: Implement Styx/figue loading**

The checked-in configuration contains the exact stable question text, authority set, output sections, severity/confidence schema, prohibited scope, source date epoch, and size/token limits. Do not duplicate those strings in rendering code.

- [ ] **Step 4: Add the first checkpoint configuration**

Record the eight `PLAN-*` questions from the approved design. Set conservative finite review and expanded-archive limits. List the six architecture authority documents explicitly.

- [ ] **Step 5: Run focused tests**

```text
cargo nextest run -p weavy-review-package --test validation -E 'test(config)'
```

- [ ] **Step 6: Commit**

```text
git add review-package/src/config.rs review-package/tests/validation.rs review-packages/implementation-plan-v1.styx
git commit -m "feat: load review checkpoint policy"
```

### Task 3: Exact repository and source-set snapshots

**Files:**
- Create: `review-package/src/snapshot.rs`
- Test: `review-package/tests/snapshot.rs`

**Interfaces:**
- Consumes: `RepositorySpec`, `PackageEntry` from Tasks 1–2.
- Produces: `snapshot_repository(spec, entries) -> Result<RepositorySnapshot, SnapshotError>`.
- Produces: `source_set_id(repositories, non_repository_inputs) -> [u8; 32]`.

- [ ] **Step 1: Write repository fixture tests**

Create temporary Git repositories through the test harness and assert identity changes for tracked edits, untracked included files, modes, submodule state, and source bytes; assert excluded untracked files do not change identity.

- [ ] **Step 2: Verify failures**

```text
cargo nextest run -p weavy-review-package --test snapshot
```

- [ ] **Step 3: Implement canonical Git collection**

Invoke Git with fixed configuration and NUL-delimited machine forms. Record canonical remote, object format, HEAD, status bytes/hash, included diff bytes/hash, submodules, LFS identities when present, and per-file source kind/blob identity. Never parse human-colored output.

- [ ] **Step 4: Implement snapshot/source-set hashes**

Hash canonical Facet model bytes, not raw platform structs. Sort repositories by `repository_id` and files by normalized repository-relative path.

- [ ] **Step 5: Run tests**

```text
cargo nextest run -p weavy-review-package --test snapshot
```

- [ ] **Step 6: Commit**

```text
git add review-package/src/snapshot.rs review-package/tests/snapshot.rs
git commit -m "feat: identify review source snapshots"
```

### Task 4: Explicit staging and generated control documents

**Files:**
- Create: `review-package/src/stage.rs`
- Create: `review-package/src/evidence.rs`
- Test: `review-package/tests/validation.rs`

**Interfaces:**
- Consumes: checkpoint and repository snapshots.
- Produces: `stage_release(input, destination) -> Result<Vec<StagedEntry>, StageError>`.
- Produces: root documents `00` through `09`, generated line-numbered views, and evidence metadata.

- [ ] **Step 1: Write staging tests**

Assert one explicit source file maps to one normalized archive path; no recursive copy exists; generated views cite source digest; question text is identical in `checkpoint.json`, `01`, and `06`; active references resolve.

- [ ] **Step 2: Verify failures**

```text
cargo nextest run -p weavy-review-package --test validation -E 'test(stage)'
```

- [ ] **Step 3: Implement explicit staging**

Copy only configured files. Reject source symlinks and normalized path collisions. Render root documents solely from checkpoint/model data and source annotations.

- [ ] **Step 4: Implement evidence capture records**

Evidence commands execute without shell interpolation, save full stdout/stderr separately, and record exact environment/toolchain/snapshot metadata. The first package needs bare `cargo metadata`, feature-tree, check, nextest, and formatting evidence.

- [ ] **Step 5: Implement coverage budgeting**

Count required-review bytes/lines and estimate tokens with the schema-defined method/margin. Fail when limits are exceeded.

- [ ] **Step 6: Run tests and commit**

```text
cargo nextest run -p weavy-review-package --test validation -E 'test(stage) | test(evidence) | test(budget)'
git add review-package/src/stage.rs review-package/src/evidence.rs review-package/tests/validation.rs
git commit -m "feat: stage self-contained review releases"
```

### Task 5: Security and referential-integrity validation

**Files:**
- Create: `review-package/src/security.rs`
- Create: `review-package/src/validate.rs`
- Create: `review-packages/security-policy-v1.styx`
- Test: `review-package/tests/validation.rs`

**Interfaces:**
- Consumes: staged entries and checkpoint policy.
- Produces: `validate_stage(...) -> Result<ValidationReport, ValidationError>`.
- Produces: `SecurityReport` and required `SecurityDisposition` records.

- [ ] **Step 1: Write rejection tests**

Cover private keys, representative token formats, high-entropy candidates without disposition, nested archives, traversal, absolute active links, stale source revision, absent authority entry, duplicate/case-folded paths, over-budget review set, and proprietary file lacking upload approval.

- [ ] **Step 2: Verify failures**

```text
cargo nextest run -p weavy-review-package --test validation -E 'test(security) | test(references)'
```

- [ ] **Step 3: Implement the scanner**

Use a versioned Styx policy. Scan file content and active Markdown/control fields separately. Preserve inert historical paths in classified evidence. Every candidate is fail-closed until an explicit disposition record exists.

- [ ] **Step 4: Implement manifest closure and referential integrity**

Prove exact entry equality, authority digests, control-text agreement, current release identity, generated-view source linkage, active path resolution, and required external-AI approval.

- [ ] **Step 5: Run tests and commit**

```text
cargo nextest run -p weavy-review-package --test validation
git add review-package/src/security.rs review-package/src/validate.rs review-packages/security-policy-v1.styx review-package/tests/validation.rs
git commit -m "feat: validate review release security"
```

### Task 6: Canonical ZIP and detached release artifacts

**Files:**
- Create: `review-package/src/archive.rs`
- Test: `review-package/tests/archive.rs`

**Interfaces:**
- Consumes: validated staged entries and manifest.
- Produces: canonical ZIP, `.zip.sha256`, `.release.json`, detached `00`/`01`, optional review bundle.

- [ ] **Step 1: Write archive tests**

Test byte-identical generation in two separate output directories, lexical entry order, no directory entries, fixed modes/timestamps, no extra metadata, one top-level directory, and detached digest correctness.

- [ ] **Step 2: Verify failures**

```text
cargo nextest run -p weavy-review-package --test archive
```

- [ ] **Step 3: Implement deterministic archive writing**

Pin ZIP compression settings and timestamp conversion from `SOURCE_DATE_EPOCH`. Explicitly set modes and reject unsupported timestamps rather than substituting current time.

- [ ] **Step 4: Implement detached artifacts**

Serialize the release descriptor with Facet JSON after the archive digest exists. Copy detached `00`/`01` from exact staged bytes. Run final security validation over ZIP-adjacent detached artifacts and put its digest in the descriptor through a nonrecursive projection: the security report excludes the descriptor being finalized.

- [ ] **Step 5: Run tests and commit**

```text
cargo nextest run -p weavy-review-package --test archive
git add review-package/src/archive.rs review-package/tests/archive.rs
git commit -m "feat: emit deterministic review archives"
```

### Task 7: CLI and end-to-end release oracle

**Files:**
- Modify: `review-package/src/main.rs`
- Create: `review-package/tests/release.rs`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: all earlier modules.
- Produces CLI commands:

```text
weavy-review-package inspect --config <path>
weavy-review-package build --config <path> --output <dir>
weavy-review-package verify --release <release.json>
```

- [ ] **Step 1: Write the end-to-end test**

Build a fixture release twice, verify both, compare ZIP bytes, inspect the archive entry set, and assert detached identities and root-document consistency.

- [ ] **Step 2: Verify failure**

```text
cargo nextest run -p weavy-review-package --test release
```

- [ ] **Step 3: Implement command dispatch and diagnostics**

Use typed errors with phase and offending path/field. `inspect` performs no writes. `build` stages to a fresh output directory and never deletes arbitrary paths. `verify` reconstructs every detached and archive assertion.

- [ ] **Step 4: Ignore only generated outputs**

Add `review-packages/.stage/` and `review-packages/out/`; retain policies and prior-review records.

- [ ] **Step 5: Run the production-path smoke test**

```text
cargo run -p weavy-review-package -- inspect --config review-packages/implementation-plan-v1.styx
```

Expected: exact checkpoint/source inputs and budget summary without mutation.

- [ ] **Step 6: Run focused/full verification and commit**

```text
cargo nextest run -p weavy-review-package
cargo check --workspace --all-targets
cargo fmt --all -- --check
git add review-package .gitignore
git commit -m "feat: add review package CLI"
```

### Task 8: Preserve prior Pro review provenance

**Files:**
- Create: `review-packages/prior-review/package-design-r1/REVIEW-REPORT.md`
- Create: `review-packages/prior-review/package-design-r1/OWNER-DISPOSITION.md`
- Create: `review-packages/prior-review/package-design-r1/APPLIED-CHANGES.md`
- Create: `review-packages/prior-review/package-design-r2/REVIEW-REPORT.md`
- Create: `review-packages/prior-review/package-design-r2/OWNER-DISPOSITION.md`
- Create: `review-packages/prior-review/package-design-r2/APPLIED-CHANGES.md`

**Interfaces:**
- Consumes: exact Pro reports supplied in this conversation and commits `aba181b`, `278aae0`.
- Produces: immutable review inputs for the first package and generated `07` ledger.

- [ ] **Step 1: Save exact final reports**

Preserve the reports verbatim as review artifacts. Historical host/ephemeral references remain inert classified data.

- [ ] **Step 2: Write owner disposition ledgers**

Assign stable finding IDs, accepted status, technical rationale, and implementing commit/section for every named finding.

- [ ] **Step 3: Validate through the CLI**

```text
cargo run -p weavy-review-package -- inspect --config review-packages/implementation-plan-v1.styx
```

Expected: prior-review artifacts classified as evidence and no active-reference violation.

- [ ] **Step 4: Commit**

```text
git add review-packages/prior-review
git commit -m "docs: preserve Pro package reviews"
```

### Task 9: Generate the first implementation-planning release

**Files:**
- Modify: `review-packages/implementation-plan-v1.styx`
- Generate (ignored): `review-packages/out/weavy-pro-review-implementation-plan-r1-<commit>-<sourceset8>.*`

**Interfaces:**
- Consumes: this implementation plan, architecture authority, current source set, prior-review artifacts, and production evidence.
- Produces: first canonical review release and detached transport files.

- [ ] **Step 1: Freeze exact source inputs**

After all prior tasks are committed, update the checkpoint config to the exact current commit and explicit source entries. Record owner upload approval for each proprietary/private entry actually included.

- [ ] **Step 2: Capture dependency and verification evidence**

Run through the package evidence mechanism:

```text
cargo metadata --format-version 1
cargo tree --workspace -e features
cargo check --workspace --all-targets
cargo nextest run --workspace --all-targets
cargo fmt --all -- --check
```

Include full raw outputs, toolchain facts, workspace manifests, lockfile state, build scripts, current dependency graph, PHON/Weavy wire sources/tests, task graph, and caller inventory.

- [ ] **Step 3: Build twice and verify**

```text
cargo run -p weavy-review-package -- build --config review-packages/implementation-plan-v1.styx --output review-packages/out/a
cargo run -p weavy-review-package -- build --config review-packages/implementation-plan-v1.styx --output review-packages/out/b
cmp <a.zip> <b.zip>
cargo run -p weavy-review-package -- verify --release <a.release.json>
```

Expected: byte-identical ZIP and successful verification.

- [ ] **Step 4: Inspect release contents and budgets**

Confirm all first-package release gates from the design: exact target, six authorities, plan identity, dependency/current/container evidence, prior reviews, determinism, security, prompt integrity, and enforced review limits. Transport and clean-room session gates remain explicitly pending until Amos uploads the artifacts.

- [ ] **Step 5: Deliver artifacts**

Report absolute paths and SHA-256 for ZIP, `.release.json`, `.zip.sha256`, detached start/prompt, and optional review bundle. Do not upload or publish them.

- [ ] **Step 6: Commit only source policy and provenance**

The generated package remains ignored. Commit any final checkpoint-policy correction needed to reproduce it.

```text
git add review-packages/implementation-plan-v1.styx
git commit -m "docs: freeze implementation review checkpoint"
```

## Final Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo nextest run --workspace --all-targets`
- [ ] `cargo run -p weavy-review-package -- inspect --config review-packages/implementation-plan-v1.styx`
- [ ] Two production builds yield byte-identical ZIPs.
- [ ] `verify` passes against the delivered release descriptor.
- [ ] Working tree contains no uncommitted source/policy/provenance changes.
