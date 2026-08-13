# Weavy Pro review packages

## Purpose

Each ChatGPT Pro review receives an immutable, self-contained ZIP archive. The archive, rather than chat history, carries project orientation, authority, current state, evidence, prior-review decisions, and the exact review prompt.

The package must let a fresh Pro session produce a useful review without supplementary explanation while remaining useful in a continuing session.

## Session policy

Use a fresh Pro session for every major checkpoint:

1. implementation plan;
2. Gate 0 schemas, roots, identity vectors, and study authority;
3. admission and interpreter implementation;
4. native compilation, publication, and retirement;
5. final Gate 0 evidence and selection.

Reuse a session only for bounded follow-up on the same package and review round. A corrected package may remain in the same session when it addresses a small set of findings without changing the authority or review phase. A new authority revision, implementation phase, or evidence set requires a fresh package and fresh session.

The current long-running Pro session gets the first fully oriented planning package. Its dependence on information absent from the archive is itself an orientation defect to correct. Retire that session after the planning round.

## Archive identity

Archive names use:

```text
weavy-pro-review-<checkpoint>-r<revision>-<source-commit>.zip
```

A correction increments `r`; archives are never overwritten. Each release records:

- archive SHA-256;
- source repository revisions;
- checkpoint and package revision;
- generation date;
- superseded package, when applicable.

## Layout

```text
weavy-pro-review-<checkpoint>-r<revision>-<source-commit>/
├── 00-START-HERE.md
├── 01-REVIEW-PROMPT.md
├── 02-PROJECT-ORIENTATION.md
├── 03-AUTHORITY-AND-SCOPE.md
├── 04-CURRENT-STATE.md
├── 05-DECISIONS-AND-NON-GOALS.md
├── 06-QUESTIONS-FOR-PRO.md
├── 07-PRIOR-REVIEW-HISTORY.md
├── 08-FILE-MANIFEST.md
├── authoritative/
├── implementation/
├── evidence/
└── prior-review/
```

### 00-START-HERE.md

States the checkpoint, purpose, reading order, authority precedence, output contract, and the distinction between authoritative, proposed, generated, and evidence-only files.

### 01-REVIEW-PROMPT.md

Contains a prompt Amos can paste verbatim. It defines:

- Pro's role for the round;
- exact target and questions;
- settled decisions that are not open absent a demonstrated contradiction;
- severity and confidence threshold;
- required citations and correction shape;
- prohibited scope expansion;
- requested output schema.

### 02-PROJECT-ORIENTATION.md

Provides stable context:

- Vixen, Vix, Weavy, Snark, and Dibs relationships;
- producer and consumer boundaries;
- semantic program versus physical encoding;
- PHON's container role;
- interpreter/native equivalence;
- Gate 0's purpose;
- relevant repository layout.

### 03-AUTHORITY-AND-SCOPE.md

Records the exact precedence for the checkpoint:

1. owner instructions included in the package;
2. approved Weavy architecture;
3. normative bytecode specification;
4. opcode/relation catalogs and runtime profiles;
5. Gate 0 plan;
6. implementation;
7. experiment evidence and historical drafts.

It lists allowed conclusions, excluded subsystems, and decisions requiring owner approval.

### 04-CURRENT-STATE.md

Records repository revisions, implemented and unimplemented components, verification commands and observed results, known limitations, unresolved gates, relevant concurrent work, and the classification of included artifacts.

### 05-DECISIONS-AND-NON-GOALS.md

Summarizes settled decisions, rejected alternatives and reasons, explicit non-goals, and constraints that may be reopened only with an executable contradiction.

### 06-QUESTIONS-FOR-PRO.md

Contains a finite checkpoint-specific checklist. The first planning review asks whether:

1. the dependency graph is faithful to architecture v6;
2. any task depends on an artifact it is ordered before;
3. independent work is parallelized safely;
4. early choices leak into durable bytes or semantic identity;
5. every task has a production-path oracle;
6. experimental authority can leak into shipping authority;
7. any mandatory artifact is unconstructible from its prerequisites;
8. any high-confidence blocking or important issue remains.

### 07-PRIOR-REVIEW-HISTORY.md

Contains a compact ledger per review: package digest, purpose, accepted findings, rejected findings with technical reasons, changes made, and remaining questions. Raw chat transcripts are excluded unless a specific disputed finding requires one.

### 08-FILE-MANIFEST.md

Records for every included file:

- archive path and source path;
- repository and revision;
- SHA-256;
- authority classification;
- generated status;
- inclusion reason;
- whether Pro must review it or use it only as evidence.

## Inclusion policy

Include exact authoritative documents, relevant complete small crates or focused source material, the proposed implementation plan or changed implementation, verification evidence, and the prior-review ledger.

Exclude unrelated repositories, build output, caches, credentials, secrets, irrelevant proprietary material, obsolete drafts, and raw session transcripts. Relevant proprietary code may be included intentionally and must be labeled in the manifest.

Packages must not silently depend on files outside the archive.

## Generation and validation

Package construction must be deterministic from an explicit manifest. Validation must prove:

- every manifest entry exists and matches its SHA-256;
- no unmanifested file enters the archive;
- required orientation files exist;
- referenced repository revisions and source paths are recorded;
- the ZIP expands to one top-level directory;
- a second generation from unchanged inputs is byte-identical;
- the package contains no credential or secret files selected by path;
- the ready-to-paste prompt names the same checkpoint and authority as the manifest.

The generated archive and its SHA-256 are deliverables. Generated staging directories are disposable and are not committed unless a later repository convention explicitly requires them.

## Review-round lifecycle

1. Freeze checkpoint scope and source revisions.
2. Generate orientation and checkpoint-specific prompt from current authority.
3. Assemble only manifest-listed files.
4. Validate hashes, references, exclusions, and deterministic regeneration.
5. Give the ZIP and verbatim prompt to Pro.
6. Classify findings as accepted, rejected with evidence, or unresolved.
7. Apply accepted corrections through the normal repository workflow.
8. Record the outcome in the prior-review ledger.
9. Produce `r2` only when another Pro pass on the same checkpoint is warranted.
10. Start the next major checkpoint in a fresh session and package.

## First package

The first archive targets implementation planning at Weavy commit `d358092` or its planning successor. It includes the six architecture-v6/Gate 0 documents, the implementation plan, only the repository source needed to validate current wire/container facts, current verification evidence, and a concise history of the architecture audit and accepted corrections.

Pro acts as an adversarial implementation-plan reviewer, not a replacement architect. It must identify concrete dependency, authority, durability, oracle, or constructibility defects and avoid reopening settled design without exact contradictory evidence.
