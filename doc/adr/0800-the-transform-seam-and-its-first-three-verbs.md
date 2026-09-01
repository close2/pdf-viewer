# 0800 — The transform seam and its first three verbs: `render`, `images`, `attachments`

Session 867. Status: **accepted**. The first decision record of RFC 0002's implementation;
this range (0800 upward) is reserved for the transform suite, and main rounds continue from 0790.

## Context

RFC 0002 proposed a document-transform suite — split, merge, page assembly, image extraction,
rasterisation, optimisation, attachments — as one CLI over one library crate whose public API it
called the *transform seam*. On 2026-09-01 the project owner read the RFCs and said:

> Please start the command line features.

That is the word that starts RFC 0002. Its §13 put seven questions to the owner, and **none of
them was individually answered**; the owner's sentence accepts the direction, not the details. So
this ADR treats the RFC's own recommendations as the defaults, records each one as a stated
assumption the owner can overrule, and lands what §14 names as the natural first round: the three
verbs with no writer dependency, which force the seam, the CLI grammar, the range parser and the
report into existence.

## Decision

### 1. A workspace crate under `crates/`, named `pdf-transform`, with the binary inside it

`crates/pdf-transform` — a library whose public API is the seam, and `src/bin/pdf-transform.rs`
as its first consumer. Under `crates/` rather than `tools/` because RFC 0002 §5's rule is that
`tools/` holds instruments and `crates/` holds what ships, and a KIO worker or a FUSE filesystem
depending on a *tool* would be the wrong direction; `tools/pdf-retrieve` stays where it is and
stays a separate binary (RFC §13 question 6's default: it answers questions, this one makes
files). The manner is `pdf-retrieve`'s: no argument-parsing dependency, no serialisation
dependency, typed errors, a hand-rolled JSON module for a fixed shape the crate writes and never
parses. That JSON module is a second copy of `pdf-retrieve`'s 150 lines rather than a dependency
on it, for the same direction-of-dependency reason.

### 2. The seam, as types

`Plan` (data: which verb, which selection, which options — no path inside) · `Source` (bytes plus
an optional `Secret`) · `Sinks` (a `Sync` trait handing out one `Write` per output, keyed by the
pattern-expanded name; the library never opens a path) · `Policy` (the restrictions level) ·
`Budget` (parse limits and a per-page pixel ceiling) · `apply(plan, sources, sinks, policy,
budget) -> Result<Report, Refusal>`. `MemorySinks` is in the crate so a test and an in-memory
consumer need not write one.

Two departures from the RFC's sketch, both stated:

- **`Source` is whole bytes, not a seekable reader.** `pdf_syntax::Document` opens over an
  `Arc<[u8]>` today, and the seekable form is the serializer round's need (copying stream bytes
  by range, RFC §12); the type is where it will be added, and nothing built on the seam has to
  change when it is.
- **`Secret` is `viewer-core`'s, re-exported.** A second password type would be a second buffer
  to clear and a second `Debug` to audit; `viewer-core` costs three crates this one already
  depends on and nothing of its vocabulary crosses. The RFC named this reuse.

### 3. The range grammar and the name patterns, exactly as RFC 0002 §4.2–§4.3

`range.rs` is the grammar's table, one form a row, with unit tests whose expected values are the
rows. The two deliberate departures from prior art are the RFC's and are documented as choices:
parity by **page number**, and `@label` addressing in the grammar with the first match taken where
a document repeats a label. Parsing and resolving are two steps so that a plan can be built before
a file is open and resolved against any source.

`pattern.rs` is `%d` / `%0Nd` / `%p` / `%l` / `%t` / `%%`. pdfseparate's rule holds: more than one
output with no `%d` is a usage error (exit 1). `%l` and `%t` are the *document's* text, so they
are sanitised — `/`, `\`, control bytes and the characters Windows forbids replaced by `_`, an
empty result becoming `_` — and the report says which names were changed. `../../.bashrc` as a
title is a test.

### 4. Exit statuses and the report

RFC §4.4's four: 0, 2 (the file defeated us), 3 (written, with warnings; `--strict` makes it 2,
`--quiet-warnings` 0), 4 (refused by name — policy, or an unsupported construct on the path),
and 1 left to argument parsing. **Where the line between 3 and 4 falls for `render` is this
round's choice**: a page whose interpretation reports marks it could not draw is *written and
warned about* — the output is usable and every missing mark is named per page, which is what
trap 5 asks — while a page the rasteriser will not draw at all, or that the budget will not admit,
is *refused by name* and the other pages are still written. A sink that fails is the machine's
and ends the run with 2. The report (`--report=json`) carries every output with its provenance
(page and label, or image object and first page, or attachment name), the inventory for
`--list`, every warning and every refusal.

### 5. The three verbs

- **`render`** is `interpret` → `render-cpu` → the in-tree PNG encoder, at `dpi / 72` per
  §8.3.2.3's user-space unit, with `TargetSpec::for_page` deciding the whole-pixel extent exactly
  as the viewer does. CPU only, rayon across pages, one font cache and one rasteriser per worker
  thread. The integration test holds a rendered page **byte for byte** to the oracle backend's
  raster produced independently in the test, through the seam and through the program, so the
  tool cannot become a fourth rasteriser. `--format ppm` drops the alpha, which loses nothing
  because §11.4.7's page group is composited onto 𝑊 white before the raster leaves the backend.
- **`images`** enumerates image `XObject`s reachable from each selected page's resources,
  descending into form `XObject`s, each object once on the first page that reaches it; decodes
  through `pdf_model::image::decode` — so JBIG2, JPX and CCITT go through the confined worker
  exactly as in the viewer, and a build without the worker refuses those images *by name* (seen
  in this round's own test run, which is why the restriction test extracts attachments instead);
  writes PNG with the soft mask in the alpha. `--list` is the inventory; `--min-pixels` a floor.
- **`attachments`** lists, saves all (`-o dir/` is `dir/%t`, `%t` the file's own name sanitised)
  or saves one by name, over `pdf_model::attachment::attachments` — the name tree and the
  catalog's `/AF`, deduplicated, exactly as the viewer's files panel lists them. A damaged stream
  or a failed `/CheckSum` is a warning; a stream this reader refuses is a refusal by name.

### 6. The defaults assumed for RFC §13's open questions

Each is the RFC's own recommendation, taken because the owner's word accepted the direction and
answered none of them individually. **Any one of them the owner can overrule in a sentence.**

| § 13 | assumed | what it cost this round |
|---|---|---|
| 1 — the redrawn authoring exclusion | **not needed yet, and `CLAUDE.md` is not touched.** No writer landed; the three verbs derive files that are not PDFs. | Nothing. The amendment is the serializer round's, and it is the owner's to ratify then. |
| 2 — the DCT encoder | **absent.** No JPEG output from `render`, no lossy optimise; `--help` says so. | Nothing. |
| 3 — confinement tranche | **in-process, tranche one**, the worker split named as a follow-up (`doc/todo/57`). | **This is the tranche's known cost, stated plainly: the CLI parses and decodes untrusted documents in its own process.** The parse path is memory-safe, budgeted and fuzzed — the posture `pdf-viewer` ships with outside `pdf-viewer-confined` — and the three codecs the tree confines *are* confined here too, because `images` uses `pdf-model`'s decode path rather than reaching around it. What is not confined is the interpreter and the Flate/LZW/DCT decoders, which run in the caller's process. |
| 4 — restrictions for a non-interactive tool | **default `off`**, with `--restrictions=on\|warn` — three of `CLAUDE.md`'s four levels, `ask` needing a host that can ask. Asked once, in `apply`, never at the point of an operation. `render` is mapped to Table 22 bit 3 (print) and `images` / `attachments` saving to bit 5 (extract), as choices; listing is never restricted. | The two operations are an enum in this crate rather than in `pdf_model::restriction::Operation`, where they belong, because that is a first-row crate and this round did not run the whole sequence. `doc/todo/57` carries the move. |
| 5 — linearisation | untouched. | Nothing. |
| 6 — naming | **`pdf-transform`** for crate and binary; `pdf-retrieve` stays separate. | Nothing. |
| 7 — metadata stance | **deterministic by construction**: no clock is reachable from the crate, the PNG encoder writes no time or text chunk, and there is no `/Producer` to write because nothing written is a PDF. The test asserts the seam's bytes equal the program's equal `-o -`'s. | Nothing yet; the serializer round inherits the rule. |

### 7. Two smaller choices

- **Passwords: `--password-fd <n>` only.** There is no `--password` (argv is public, RFC §4.3)
  and no `--password-prompt` yet: suppressing echo on a terminal needs a terminal-mode dependency
  the tree has not argued in `doc/stack.md`. `doc/todo/57`.
- **`--page-box` and `--no-annotations` are not flags this round.** `interpret` draws the crop
  box with annotations, as §14.11.2.1 and §6.3.2.2 say a viewer does, and offers no knob for
  either; adding one is an interpreter change, first-row, and the next round's if wanted.

## Consequences

- A program in the tree now derives files from documents. The seam has survived three verbs'
  worth of contact, which is what RFC §14 wanted before the serializer and 786's consumers meet
  it.
- The throughput baseline is in `doc/history/867-*.md`; §12's perf floor is not gated yet
  because no transform gate exists to carry it and the tree's perf gates are the viewer's
  launch-path ones. What the baseline shows and the next round should look at first: rendering
  200 pages of the standard on 24 threads spends about 2.4× the CPU time of one thread doing the
  same work — the per-thread font caches re-parse what one cache would parse once, and
  `interpret` already bands colour conversion across the global pool. Measured before it is
  chosen, per principle 2.
- `doc/todo/57` is the suite's remaining work: the other verbs, the serializer, the worker split,
  inline images and `--native`, file-attachment annotations, the prompt, the `Operation` move,
  and a transform gate with the floor.
