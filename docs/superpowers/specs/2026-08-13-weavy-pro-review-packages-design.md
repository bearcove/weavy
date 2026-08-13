# Weavy Pro review packages

## Purpose

Each ChatGPT Pro review receives an immutable, self-contained review release. The release, rather than chat history, carries project orientation, normative authority, factual evidence, prior-review decisions, the exact review target, and the paste-ready prompt.

The canonical archival artifact is a deterministic ZIP. The release may also contain detached transport derivatives for review environments that cannot enumerate ZIPs directly. A fresh Pro session must be able to identify the checkpoint, authority, state, questions, files, and unsupported claims without supplementary project context.

## Session policy

Use a fresh Pro session for every major checkpoint:

1. implementation plan;
2. Gate 0 schemas, roots, identity vectors, and study authority;
3. admission and interpreter implementation;
4. native compilation, publication, and retirement;
5. final Gate 0 evidence and selection.

Reuse a session only for bounded follow-up on the same package release and review round. A corrected release may remain in the same session only when authority, checkpoint, target, and question set are unchanged and the follow-up is explicitly limited to named findings. The reviewer must verify the new release identity, treat the earlier release as superseded, and identify conclusions carried forward from the prior round.

The current long-running Pro session may receive the first planning package as a transition preflight and may perform a comparison review, but it cannot certify self-containment. Before the planning package is accepted, a separate fresh Pro session must complete a cold-start orientation check using only the released package. The formal review record identifies which session supplied the substantive review and which supplied the cold-start check.

The cold-start reviewer must identify:

- the exact checkpoint and review target;
- both authority planes and their precedence;
- settled and open decisions;
- the complete question set and requested output;
- current implementation state;
- every required package entry;
- facts it cannot establish from the package.

Any project-specific conclusion that depends on prior chat history is a package defect.

## Release identity

Archive names use:

```text
weavy-pro-review-<checkpoint>-r<revision>-<weavy-commit>-<sourceset8>.zip
```

A correction increments `r`; releases are never overwritten. Every release has three detached canonical artifacts:

```text
weavy-pro-review-….zip
weavy-pro-review-….zip.sha256
weavy-pro-review-….release.json
```

The archive SHA-256 is never embedded in the bytes it identifies. The detached release descriptor records:

- package schema version;
- checkpoint ID and package revision;
- exact archive SHA-256;
- frozen `SOURCE_DATE_EPOCH`;
- primary Weavy revision;
- complete source-set identity;
- generator and ZIP implementation versions;
- superseded release identity, when applicable;
- hashes of detached transport derivatives.

The primary commit in the filename is convenient provenance. The source-set identity is the identity of every packaged repository/worktree input.

## Canonical machine-readable control files

The archive contains:

```text
checkpoint.json
package-manifest.json
```

### `checkpoint.json`

This is the source of truth for control-document consistency. It contains:

```text
package_schema_version
checkpoint_id
package_revision
review_target
normative_authority_set
factual_authority_set
settled_decision_ids
open_decision_ids
question_ids
required_output_sections
severity_schema
confidence_schema
prohibited_scope
repository_snapshots
source_date_epoch
```

`00-START-HERE.md`, `01-REVIEW-PROMPT.md`, `03-AUTHORITY-AND-SCOPE.md`, `06-QUESTIONS-FOR-PRO.md`, and `08-FILE-MANIFEST.md` are generated from or mechanically validated against this descriptor.

Every question has a stable checkpoint-scoped ID, for example:

```text
PLAN-DEP-001
PLAN-ORDER-002
PLAN-DURABILITY-003
```

### `package-manifest.json`

This file lists every archive entry except itself. Validation requires:

```text
archive entries == package-manifest entries + package-manifest.json
```

For each repository it records:

```text
repository_id
canonical_remote
object_format
head_commit
submodule_revisions
lfs_object_ids
worktree_dirty
git_status_sha256
included_diff_sha256
worktree_snapshot_id
```

For each file it records:

```text
archive_path
repository_id
repository_relative_source_path
source_kind               # tracked | modified | untracked | generated
git_blob_id               # when applicable
sha256
size
mode
media_type
authority_class
review_requirement        # required | consultable | evidence-only
generated_from
inclusion_reason
confidentiality
license_or_owner
external_ai_upload_approved
redaction_status
contains_personal_data
```

Packaged file bytes are authoritative for the review release. Commit IDs are provenance and do not override dirty, generated, modified, or untracked bytes recorded in the manifest.

`08-FILE-MANIFEST.md` is a generated human-readable view. It is generated from a projection that does not contain its own hash; `package-manifest.json` then records the final Markdown file hash. No file contains or claims its own digest.

## Archive layout

```text
weavy-pro-review-<checkpoint>-r<revision>-<weavy-commit>-<sourceset8>/
├── checkpoint.json
├── package-manifest.json
├── 00-START-HERE.md
├── 01-REVIEW-PROMPT.md
├── 02-PROJECT-ORIENTATION.md
├── 03-AUTHORITY-AND-SCOPE.md
├── 04-CURRENT-STATE.md
├── 05-DECISIONS-AND-NON-GOALS.md
├── 06-QUESTIONS-FOR-PRO.md
├── 07-PRIOR-REVIEW-HISTORY.md
├── 08-FILE-MANIFEST.md
├── 09-COVERAGE-AND-OMISSIONS.md
├── authoritative/
├── implementation/
├── evidence/
├── generated-views/
└── prior-review/
```

## Root documents

### `00-START-HERE.md`

States the release identity, checkpoint, purpose, reading order, both authority planes, output contract, accessibility preflight, and the distinction between authoritative, proposed, generated, and evidence-only files.

The first review action is:

> Confirm that every required package entry is accessible. If the archive cannot be enumerated or a required file cannot be opened, stop and report a package-transport defect. Do not compensate by searching public repositories or relying on chat history.

### `01-REVIEW-PROMPT.md`

Contains a prompt Amos can paste verbatim. It defines:

- Pro's role for the round;
- exact release, target, and stable question IDs;
- settled decisions not open absent a demonstrated contradiction;
- severity and confidence schema;
- required citations and correction shape;
- prohibited scope expansion;
- required response sections;
- outside-context policy;
- final coverage declaration.

Project-specific conclusions must come from the package. Missing project information is a package defect. External research is permitted only when the checkpoint explicitly asks for prior art or current external facts, and it must be labelled separately from package authority.

### `02-PROJECT-ORIENTATION.md`

Provides stable context:

- Vixen, Vix, Weavy, Snark, and Dibs relationships;
- producer and consumer boundaries;
- semantic program versus physical encoding;
- PHON's container role;
- interpreter/native equivalence;
- Gate 0's purpose;
- relevant repository layout.

Generated orientation claims cite exact packaged authority entries and digests.

### `03-AUTHORITY-AND-SCOPE.md`

Defines two independent authority planes.

**Normative precedence — what ought to be true**

1. package-specific owner instructions;
2. approved architecture;
3. normative bytecode specification;
4. approved opcode, relation, and runtime-profile specifications;
5. approved Gate 0 or checkpoint plan;
6. implementation choices where higher authority leaves freedom.

**Factual precedence — what is currently true**

1. exact packaged repository/worktree snapshots;
2. reproducible raw command, test, benchmark, and inspection evidence tied to those snapshots;
3. recorded but not package-reproducible evidence;
4. generated summaries and orientation prose;
5. historical drafts and superseded proposals.

A factual artifact cannot silently override normative authority. It may demonstrate that authority is unimplemented, inconsistent, or unconstructible. Historical drafts are context only.

A settled decision may be reopened only by a demonstrated contradiction: an exact higher-authority conflict, constructibility or dependency proof, minimal counterexample, exact current-source evidence, or reproducible experiment. Reopening is limited to the smallest affected decision.

The document also lists allowed conclusions, excluded subsystems, and decisions requiring owner approval.

### `04-CURRENT-STATE.md`

Records exact repository/worktree snapshots, implemented and unimplemented components, verification evidence, known limitations, unresolved gates, relevant concurrent work, and artifact classifications.

Every verification evidence item records:

```text
command
working_directory
repository_snapshot
OS and architecture
toolchain versions
feature/profile selection
relevant environment
start_time
end_time
exit_status
stdout_path and sha256
stderr_path and sha256
raw_result_path
reproducible_from_package
reason_if_false
```

Generated prose summaries cite raw evidence.

### `05-DECISIONS-AND-NON-GOALS.md`

Summarizes settled decisions with stable IDs, rejected alternatives and reasons, explicit non-goals, open decisions, and the demonstrated-contradiction rule.

### `06-QUESTIONS-FOR-PRO.md`

Contains the finite stable-ID checklist for the checkpoint. The first planning review asks whether:

1. `PLAN-DEP-001`: the implementation dependency graph is faithful to architecture v6 and current repository layering;
2. `PLAN-ORDER-002`: any task depends on an artifact ordered after it;
3. `PLAN-PARALLEL-003`: independent work is parallelized without violating authority dependencies;
4. `PLAN-DURABILITY-004`: early choices leak accidentally into durable bytes or semantic identity;
5. `PLAN-ORACLE-005`: every task has a production-path oracle;
6. `PLAN-AUTHORITY-006`: experimental authority can leak into shipping authority;
7. `PLAN-CONSTRUCT-007`: any mandatory artifact is unconstructible from its prerequisites;
8. `PLAN-BLOCKER-008`: any high-confidence blocking or important issue remains.

Every question appears identically in the paste-ready prompt and checkpoint descriptor.

### `07-PRIOR-REVIEW-HISTORY.md`

A generated compact ledger of immutable prior-review artifacts. For each review it records package digest, purpose, accepted/rejected/unresolved findings, owner rationale, applied commits, superseding findings, and remaining questions.

Raw conversational transcripts remain excluded unless needed to resolve a specific dispute. Exact final review reports are retained.

### `08-FILE-MANIFEST.md`

Human-readable manifest view showing source provenance, hashes, authority class, confidentiality, upload approval, inclusion reason, and whether Pro must read, consult, or merely treat each file as evidence.

### `09-COVERAGE-AND-OMISSIONS.md`

Makes review coverage falsifiable. It records:

- claims the package is intended to support;
- represented source roots and call paths;
- nearby files deliberately omitted;
- excluded repositories or subsystems;
- known blind spots;
- why selected evidence is sufficient;
- total package size;
- required-review byte, line, and estimated token budget;
- consultable/evidence-only size;
- files not expected to be read completely.

The reviewer must end with:

```text
Fully reviewed:
Searched/consulted:
Not inspected:
Commands independently run:
Claims not supportable from package:
```

## Prior-review artifacts

Each completed review round retains:

```text
prior-review/<review-id>/
├── REVIEW-REPORT.md
├── OWNER-DISPOSITION.md
└── APPLIED-CHANGES.md
```

Every finding has a stable ID and records:

- originating package/release digest;
- exact report location;
- severity and confidence;
- accepted, rejected, or unresolved disposition;
- owner rationale;
- implementing commit or document change;
- superseding finding, when applicable.

## Inclusion and security policy

Include exact authority, relevant complete small crates or focused source material, the exact implementation plan or changed implementation, dependency evidence, raw verification evidence, and prior-review artifacts.

Exclude unrelated repositories, build output, caches, credentials, secrets, irrelevant proprietary material, obsolete drafts, and raw chats. Packages never silently depend on files outside the archive.

Relevant proprietary material enters a release only with explicit owner approval for upload to the external AI review service. Labelling a file proprietary is not approval.

Validation includes:

- path allowlist and denylist;
- content-based secret scanning;
- private-key and token-pattern detection;
- high-entropy candidate reporting;
- recursive scanning or rejection of nested archives;
- environment/log redaction checks;
- explicit manual disposition for every finding;
- rejection of symlinks, hard links, devices, extraction traversal, absolute paths, and `..`;
- compressed and expanded size limits;
- per-file confidentiality, ownership, upload approval, redaction, and personal-data classification.

## Deterministic archive contract

Canonical ZIP generation freezes:

- lexical UTF-8 archive-path order;
- `/` separators;
- NFC Unicode normalization;
- fixed file and directory modes;
- timestamp from frozen `SOURCE_DATE_EPOCH`;
- no UID/GID or host-specific metadata;
- no ZIP comments;
- no uncontrolled extra fields;
- one pinned ZIP implementation and compression configuration;
- explicit directory-entry policy;
- one top-level directory;
- package schema version.

The generator rejects symlinks, hard links, devices, absolute paths, `..`, duplicate paths, case-folding collisions, and unsafe nested archives. The generation date is a frozen input, never `now()`.

Two generations from unchanged manifest inputs must be byte-identical.

## Control-document referential integrity

Validation proves:

- every control-document path resolves to a manifested entry;
- every authority reference resolves to an exact file and digest;
- question IDs and text agree across `checkpoint.json`, `01`, and `06`;
- required response sections, severity, and confidence schemas agree;
- repository revisions and source-set identity agree everywhere;
- no document names a superseded release as current;
- generated orientation claims cite packaged authority;
- no released root document contains an absolute host path unless explicitly classified as evidence;
- no `local://paste-*`, `/tmp/*`, `/Users/*`, stale revision, or absent-file reference survives;
- generated line-numbered views link to the exact source digest while originals remain present.

## Transport derivatives

ZIP is canonical but is not assumed to be directly inspectable in every review environment. Each release also provides detached exact copies of:

```text
00-START-HERE.md
01-REVIEW-PROMPT.md
```

It may provide a deterministic `REVIEW-BUNDLE.md` or extracted upload set. Transport derivatives are not separate authority; the release descriptor records their hashes, and validation proves their contents match archive entries or a documented deterministic projection.

A transport preflight must prove that the target session can enumerate and open every required entry before substantive review begins.

## Review-round lifecycle

1. Freeze checkpoint, exact source set, package revision, and `SOURCE_DATE_EPOCH`.
2. Record explicit external-AI upload approval for proprietary inputs.
3. Generate `checkpoint.json` and root documents from current authority.
4. Assemble only manifest-listed files.
5. Generate line-numbered views, evidence metadata, coverage ledger, and human manifest.
6. Generate `package-manifest.json` and validate exact entry equality.
7. Run path/content security scanning and record manual dispositions.
8. Generate the canonical ZIP twice and require byte identity.
9. Generate detached release descriptor, SHA-256, and transport derivatives.
10. Run transport accessibility preflight.
11. Give the ZIP and paste-ready prompt to Pro.
12. Run the fresh-session cold-start orientation check.
13. Perform substantive review and classify findings.
14. Preserve exact final report, owner disposition, and applied changes.
15. Produce `r2` only for warranted same-checkpoint follow-up.
16. Start the next major checkpoint in a fresh session and release.

## First implementation-planning package

The generator freezes one exact Weavy source set; the released package never says “commit or successor.” It includes:

- the six architecture-v6/Gate 0 authority documents, individually named and hashed;
- one exact implementation-plan file and digest;
- workspace and relevant crate `Cargo.toml` files;
- `Cargo.lock`;
- relevant `build.rs` files;
- generated-code ownership notes;
- `cargo metadata` output;
- `cargo tree -e features` or equivalent feature graphs for proposed closures;
- current crate/dependency graph;
- source proving current PHON/Weavy layering and wire facts;
- implementation task graph in machine-readable form;
- production-path and caller inventory for named routes;
- exact commands, raw logs, toolchain, environment, and exit statuses;
- exact prior final review reports and owner dispositions;
- coverage and omissions ledger.

Pro acts as an adversarial implementation-plan reviewer, not a replacement architect. It identifies concrete dependency, authority, durability, oracle, coverage, or constructibility defects and does not reopen settled design without demonstrated contradictory evidence.

## First-package release gate

The first planning release is ready only when:

| Gate | Required result |
|---|---|
| Exact target | One frozen Weavy source/worktree snapshot |
| Complete authority | Six authority documents individually named and hashed |
| Plan identity | One exact implementation plan and digest |
| Dependency evidence | Workspace manifests, lockfile, metadata, feature graph, relevant build scripts |
| Current-state evidence | Exact commands, raw logs, toolchain, environment, and exit status |
| Container evidence | Current PHON/Weavy wire sources and tests |
| Prior-review provenance | Exact final reports and owner disposition ledger |
| Determinism | Two canonical builds produce identical ZIP bytes |
| Security | Path/content scans pass; dispositions and proprietary upload approval recorded |
| Transport | Target review environment can enumerate and open every required entry |
| Cold-start orientation | Fresh session identifies scope, authority, questions, state, and missing facts using only the release |
| Prompt integrity | No stale revision, absent file, host path, or ephemeral URI remains |
| Reviewability | Required-reading budget and omissions ledger are present |
