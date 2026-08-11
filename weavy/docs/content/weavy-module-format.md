# Durable Weavy modules encoded with PHON

## Status

Design input for an independent implementation session. This document records the architectural invariants established while moving Snark parser generation out of Dibs process startup. It is intentionally a module-format requirement, not a Snark artifact proposal.

## Problem

Weavy currently exposes lowered programs as Rust-owned vectors: an entry program and dense callable blocks. It has no durable module format and no module-local constant pool.

Snark currently executes a Weavy parser plan while separately borrowing parser grammar, parse-table, lexer-program, and tree metadata objects. A rejected precompilation experiment serialized those separate Snark objects with Facet Postcard behind a `SNARKPAR` envelope. For the full Dibs PostgreSQL grammar, it produced an approximately 974 MB artifact that took approximately 170 seconds to load. The payload contained parser-generator workspace and duplicated per-state lexer topology; the failure was architectural, not merely a codec choice.

The full Dibs grammar currently produces 91,267 parse states, 1,861,394 transitions, and 5,908 conflicts. Runtime facts for a parser of this size must be able to ship as ordinary Weavy constants without a side lookup mechanism.

## Required invariant

A durable `.weavy` file represents one self-contained Weavy module.

The module contains:

- executable Weavy instructions and callable blocks;
- module-local typed constants;
- the PHON schemas needed to interpret those constants;
- module identity, format compatibility, and required-dialect metadata.

Instructions and dialect intrinsics reference constants through ordinary module-local `ConstantId` values. An ID is an index or range reference into the module constant space. It is never a handle into a separately loaded Snark object.

For a Snark-produced parser module, Weavy constants include:

- symbol and public-node tables;
- productions and production metadata;
- compact LR action and goto rows;
- GLR runtime rows and deterministic ranking metadata;
- lexical modes and state-to-mode mappings;
- reserved-word contexts;
- unique regex and literal matcher specifications;
- external-scanner runtime metadata;
- tree-construction metadata;
- source, trace, and debugging metadata when retained.

Snark intrinsics give these constants parser semantics, but the storage, addressing, loading, verification, and lifetime of the constants belong to the Weavy module.

## Excluded construction workspace

Parser-generator workspace does not belong in the executable module:

- LR item sets and closure state;
- lookahead propagation workspace;
- state-discovery transitions used only during table construction;
- conflict derivation workspace;
- temporary interning tables and builder caches;
- duplicated lexer structures that can be represented by interned constants and state-to-mode IDs.

The generator consumes this workspace and emits compact runtime constants, then discards it.

## Semantic module model

The smallest useful semantic model is equivalent to:

```rust
struct WeavyModule<Intrinsic, Constant> {
    manifest: ModuleManifest,
    program: DenseWeavyLowered<Intrinsic>,
    constants: ConstantPool<Constant>,
}

struct ConstantId(u32);
```

The concrete Rust generic shape is not part of the wire contract. The requirements are:

1. a single module-local constant address space;
2. typed constant entries or homogeneous typed constant ranges;
3. stable IDs independent of process addresses;
4. no native pointers, `usize`, or Rust ABI-dependent enum layout on disk;
5. verification of every instruction-to-constant reference before execution;
6. support for constants large enough to hold generated parser tables without forcing dialect-specific side storage.

A constant range is preferable to one directory entry per small homogeneous row. For example, all LR rows may occupy one typed range addressed as `first_id + state_id`. Variable-sized constants may use a checked offset index internal to their section.

## PHON is the durable encoding

PHON is the intended storage substrate. Postcard is not the module format.

PHON is appropriate because it already provides:

- a portable schema vocabulary;
- compact schema-driven encoding;
- self-describing values and schemas when required;
- content-derived schema identity;
- compatibility machinery;
- typed Rust encode/decode without a dynamic-value bounce;
- implementations and ecosystem work across Rust, Swift, and TypeScript;
- an architecture intended for durable storage rather than ephemeral Rust object snapshots.

The format must not encode the entire module as one accidental Rust object graph and call the result complete. PHON supplies the schemas and field encodings; the Weavy module format supplies executable organization, addressing, admission, and discoverability.

### Dependency layering

`phon-engine` currently depends on Weavy. A durable module codec must not create `weavy -> phon-engine -> weavy`.

The implementation must choose a deliberate acyclic layer. Plausible shapes include:

- a sibling `weavy-phon` crate depending on Weavy plus lower PHON schema/codec crates;
- extracting or using a lower PHON storage layer that does not depend on Weavy;
- keeping the semantic `WeavyModule` model in `weavy` and the `.weavy` file reader/writer in a codec crate.

Whichever crate owns encoding, decoding must yield one self-contained Weavy module. The codec boundary is not a runtime sidecar.

## ELF-inspired physical organization

ELF is useful precedent for separating bootstrap, semantic sections, and runtime loading. The first implementation should adopt only the pieces justified by current requirements.

A `.weavy` file should have:

1. a small fixed bootstrap header;
2. a PHON-encoded module directory;
3. PHON schema data;
4. one or more program and constant sections;
5. optional non-executable metadata sections.

Conceptually:

```text
fixed bootstrap header
PHON module directory
PHON schema bundle
Weavy program section
constant directory or range table
constant sections
optional source/debug/trace sections
```

### Bootstrap header

The fixed header exists only so a reader can identify the file and locate the PHON directory without decoding arbitrary bytes first. It should contain no dialect-specific semantics.

Required facts:

- `.weavy` magic;
- format major and minor versions;
- byte-order marker where fixed-width bootstrap fields require one;
- header size;
- module-directory offset and length;
- total file length;
- executable module identity or enough information to validate it.

Do not add dual directories, append journals, signatures, compression negotiation, or recovery records without a demonstrated consumer.

### PHON directory

The directory describes each section with at least:

- stable section ID and kind;
- file offset and encoded length;
- decoded length where meaningful;
- required alignment;
- PHON schema ID or explicitly declared raw-byte schema;
- executable, required, optional, or debug flags;
- integrity hash when the module identity does not already make per-section corruption unambiguous.

Unknown optional sections may be skipped. Unknown required executable sections must reject admission.

### Sections and segments

ELF distinguishes semantic sections from loadable segments. Preserve that distinction only if it solves an observed loading requirement.

Sections are immediately useful because code, constants, schemas, and debug metadata have different schemas and retention policies. A separate segment table is optional for the first cut. Add it when one runtime mapping decision must cover several sections or when mmap evidence requires it.

## Compact and aligned PHON storage

PHON compact encoding should be the first measured representation for manifests, directories, schemas, sparse metadata, and variable-shaped constants.

Large hot homogeneous tables may require an aligned storage profile for direct borrowed access from an mmap. If measurements demonstrate that decoding them into a second allocation is material, extend PHON rather than adding a Snark binary codec.

An aligned PHON profile must describe a stable storage layout, not dump Rust memory. It requires explicit:

- scalar widths and byte order;
- field offsets;
- aggregate size and alignment;
- array stride and count;
- tagged-union tag representation and variant layouts;
- relative offsets or IDs for references;
- bounds and alignment validation before borrowed access.

It must prohibit process pointers, platform `usize`, and compiler-selected Rust enum layouts.

Compact and aligned representations are storage profiles for the same PHON schema and the same semantic Weavy constant. Choosing one must not change the module model or intrinsic APIs.

## Program representation

The Weavy program is part of the same module and references constants by `ConstantId`.

The first implementation may encode typed Weavy operations with PHON if its size and load measurements are acceptable. A denser opcode stream may be introduced later without changing module semantics: it remains a typed section whose schema defines instruction and operand encoding.

The durable format must not persist JIT machine code or process addresses. Native copy-and-patch output is an ephemeral derivative of the admitted module.

## Admission and execution

Loading a `.weavy` file proceeds as follows:

1. validate the fixed header and all file bounds;
2. decode and validate the PHON directory and schema closure;
3. validate section bounds, alignment, identities, and required encodings;
4. construct borrowed or owned views of code and constant sections;
5. verify instruction operands, block references, constant IDs, dialect requirements, and constant schemas;
6. compile process-local accelerators such as unique regex engines or JIT code;
7. expose an admitted module to the interpreter or JIT.

Runtime caches are allowed. They are derived from the immutable module and are not a second semantic source of parser data.

Snark execution must no longer require separately passed `ParserGrammar`, `ParseTable`, or lexer-plan objects once migration is complete.

## Identity and compatibility

A module has a content-derived executable identity covering all code and constants that affect behavior. Debug paths, source maps, and other strippable metadata should not change executable identity; the complete file may additionally have a package identity.

The manifest declares:

- Weavy module-format compatibility;
- required dialect names and compatible intrinsic-set versions;
- root entry points;
- executable identity;
- optional producer information that does not participate in semantic identity unless explicitly required.

PHON schema IDs identify constant and directory schemas. Module verification must reject a constant whose declared schema is incompatible with the intrinsic that references it.

## Discoverability

The durable extension is `.weavy`, not `.bin`.

The repository should provide an inspection command or library surface that can report, without linking the producing frontend:

- manifest and identities;
- dialect requirements;
- section names, kinds, sizes, schemas, and alignments;
- program and constant counts;
- optional decoded constants when their PHON schemas are available.

A hex blob hidden behind `include_bytes!` is not sufficient tooling for a durable executable format.

## Snark migration

The migration sequence is:

1. add the semantic module and constant-pool model to canonical Weavy;
2. add PHON-backed `.weavy` encode/decode in an acyclic crate layer;
3. prove deterministic round-trip and large-constant behavior in Weavy/PHON without Snark;
4. define Snark constant schemas and lower generated runtime facts into them;
5. change Snark intrinsics to carry ordinary `ConstantId` references;
6. change the Snark runtime to obtain all parser facts from the admitted Weavy module;
7. compare parsing behavior against live parser construction;
8. benchmark the real Dibs grammar for file size, encode time, load/admission time, memory residency, and first parse;
9. remove the rejected `ParserArtifact`, `SNARKPAR`, Facet-Postcard payload, and separate runtime parser-table path;
10. update Snark and Dibs to pinned reviewed Weavy/PHON revisions.

There is no compatibility requirement for the rejected Snark artifact API.

## Required evidence

The implementation is not accepted without:

- deterministic `.weavy` byte round-trip;
- corruption, truncation, invalid-offset, invalid-alignment, wrong-schema, unknown-required-section, and bad-constant-ID rejection;
- a large-constant test in Weavy/PHON independent of Snark;
- behavioral equivalence between live-built and module-loaded Snark parsers;
- unchanged Dibs parser states/conflicts and accepted syntax behavior;
- measurements on the same Dibs grammar and build profile for:
  - live construction;
  - module generation;
  - file size by section;
  - module load and admission;
  - resident/allocated memory;
  - first and warm parse;
- proof that module load does not call LR table construction or Weavy lexer-plan construction;
- proof that identical regex specifications compile once per admitted module;
- inspection output demonstrating format discoverability.

## Existing measured baseline

Before module-format work:

- full Dibs parser topology: 91,267 states, 1,861,394 transitions, 5,908 conflicts;
- profiling-build live preparation after Snark regex deduplication: approximately 7.0 seconds total;
- `WeavyParsePlan::new`: approximately 3.9 seconds, reduced from approximately 31.5 seconds;
- rejected Facet-Postcard Snark artifact: approximately 974 MB and 169–173 seconds to load.

The rejected artifact is evidence of what not to serialize. It is not a target to optimize incrementally.

## Explicit non-goals for the first implementation

Unless measurements or an existing consumer require them, do not add:

- Snark-owned envelopes or sidecars;
- Postcard as the module format;
- persisted JIT code;
- native pointers or relocations for ordinary module references;
- compression;
- lazy section paging;
- signatures or package trust policy;
- dynamic linking between independently produced Weavy modules;
- a full ELF feature clone;
- backward compatibility for the rejected `SNARKPAR` experiment.

The design must leave room for aligned/mmap PHON storage, but the first implementation should prove the semantic module model and compact PHON format before adding unmeasured loader machinery.
