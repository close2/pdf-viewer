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

Negative:

- A build-time code generator is machinery a reader must understand before reading the
  validation layer. It must stay simple and its output legible.
- The pin is load-bearing: without it, generated code would change without a commit
  here. Resolved — the model is a submodule pinned at `ba7d4d61`.

## Implemented (Spike C, 2026-07-26)

611 object definitions and 3973 key rows generate into 1.8 MB of `static` tables in
about 0.5 s. Being `static`, they live in read-only data and cost nothing at startup.

### What the data turned out to be

Measured across all 3973 rows, so the split between "structure" and "predicate" is known
rather than guessed:

| Column | Populated | Contains a predicate |
|---|---|---|
| `Type` | 3973 | 2 |
| `SinceVersion` | 3973 | 645 |
| `DeprecatedIn` | 373 | 0 |
| `Required` | 3973 | 192 |
| `IndirectReference` | 3973 | 113 |
| `Inheritable` | 3973 | 0 |
| `PossibleValues` | 1146 | 315 |
| `SpecialCase` | 593 | 593 |
| `Link` | 1694 | 1 |

So the structural majority is exact, and the predicate language is concentrated in
`SpecialCase` plus a minority of four other columns.

### How much of the predicate language is handled

`SinceVersion` predicates turned out to be a **closed set of two shapes** —
`fn:Extension(Name[,version])` and `fn:Eval(fn:Extension(Name,version) || 2.0)` — so all
645 are modelled exactly, as `Availability::Extension` and
`Availability::SinceOrExtension`. `Required` is uniformly `fn:IsRequired(...)`, and the
two predicate-bearing `Type` cells are `fn:Extension` and `fn:Deprecated` wrapping a
single type. Those are modelled too.

Everything else — `SpecialCase`, predicate-bearing `PossibleValues`, conditional
`IndirectReference` — is **carried verbatim and left unevaluated**. `Requirement` is a
three-state enum for exactly this reason: collapsing `Conditional` to `Never` would
accept invalid files, and to `Always` would reject valid ones. A caller that cannot
evaluate the predicate must say so.

Deferred, therefore: a `SpecialCase` evaluator. It needs `fn:Eval`, `fn:ArrayLength`,
`fn:BitsClear`, `fn:IsPresent`, `fn:IsPDFVersion`, `fn:IsMeaningful`, `fn:Ignore` and
`fn:SinceVersion`, and it needs a document to evaluate against — so it belongs after
`pdf-syntax`, not before.

### Structural invariants found and asserted

`Type`, `Link`, `PossibleValues` and bracketed `IndirectReference` are aligned
positionally by `;`: the third type alternative takes the third group of each column.
Verified across all 1694 link-bearing and 1146 value-bearing rows with zero mismatches,
and asserted per row at generation time, since a future model update that broke it would
otherwise pair a type with another type's constraints. `TypeAlternative` groups them so a
consumer cannot mismatch them by hand.

The `Key` column carries three different things — a dictionary name, an array index, and
`*`/`<digit>*` wildcards — modelled as distinct `KeyPattern` variants rather than
strings, so a dictionary key cannot be accepted where an array index belongs.

### The rule that makes this safe

**An unrecognised token fails the build**, naming file, line and value. A generator that
skipped what it did not understand would produce a smaller table that still appeared to
work, having quietly stopped checking part of the specification. The object and key counts
are pinned by a test for the same reason.

## Alternatives considered

Hand-written validation was rejected on reviewability grounds above. Runtime parsing of
the TSVs was rejected because it would move a large data-loading cost into
time-to-first-page, which `CLAUDE.md` principle 2 protects.
