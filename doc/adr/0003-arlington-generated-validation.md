# ADR 0003 — Generate the object-model validation layer from the Arlington PDF Model

Status: accepted, 2026-07-26.

## Context

ISO 32000-2 specifies several hundred object types, each with required and optional
keys, permitted value types, default values, and the PDF version in which each key
appeared or was deprecated. A viewer must validate documents against this to reject
malformed input and to apply defaults correctly.

The Arlington PDF Model — an open PDF Association project, cloned at
`doc/arlington-pdf-model` — encodes exactly that as tab-separated data: 3468 files,
with `tsv/2.0/` covering PDF 2.0. Columns are `Key, Type, SinceVersion,
DeprecatedIn, Required, IndirectReference, Inheritable, DefaultValue, PossibleValues,
SpecialCase, Link, Note`.

## Decision

Generate the validation layer from the Arlington TSVs with a `build.rs` step in
`pdf-spec`, rather than hand-writing conformance checks.

## Consequences

Positive:

- Conformance becomes reviewable **data**. Hand-writing these checks would produce
  thousands of conditionals that no reviewer could meaningfully audit against the
  specification text; a table can be diffed against it.
- Version-awareness is free. `SinceVersion` and `DeprecatedIn` become properties of
  the generated table rather than something each check must remember independently.
- `Link` encodes the object graph, giving typed traversal between object types.
- Upstream corrections arrive as a data update rather than a code change.

Negative and open:

- A build-time code generator is machinery a reader must understand before reading the
  validation layer. It must be simple and its output legible, or it undermines the
  goal of an exemplary codebase.
- `SpecialCase` holds a small predicate language. How much of it to handle in codegen
  versus by hand is unresolved — see Phase 5C.
- **The Arlington revision must be pinned**, or generated code changes without a commit
  in this repository. The clone is currently a plain nested checkout; it should become
  a git submodule so the exact model revision is recorded. Unresolved — the nested
  `.git` also means the model is not currently tracked by this repository at all.

## Alternatives considered

Hand-written validation was rejected on reviewability grounds above. Runtime parsing of
the TSVs was rejected because it would move a large data-loading cost into
time-to-first-page, which `CLAUDE.md` principle 2 protects.
