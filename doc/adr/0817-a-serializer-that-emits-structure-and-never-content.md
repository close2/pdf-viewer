# 0817 — A serializer that emits structure and never content

Session 886. Status: **accepted**. The seventh decision record of RFC 0002's implementation, on
the long-lived branch `round-867`. ADR 0816 is the amendment that made it legal; ADR 0818 is the
first verb on it.

## Context

RFC 0002 §10 argues three options for a writer and recommends the third: a **structure-preserving
serializer**, the qpdf shape, against Option A (incremental update only — merge impossible, split
ships the whole source inside every piece) and Option B (re-distillation, the Ghostscript shape —
"a fidelity project cannot ship a writer whose output is its own dialect of the input"). The
option was recommended, not designed; what this record holds is the design, the choices the RFC
left open, and the two defects the corpus found in it on its first pass.

## Decision

### 1. Two halves: an assembly, then bytes

`pdf_syntax::serialize` is `Assembly` — the output's object table being built — and `serialize`,
which turns a finished one into a file. RFC §10's "(&[&Document], object-selection, replacements)"
is exactly the three ways into an assembly:

- **`copy(from, id)`** takes a source object *by reference*: nothing is read, cloned or decoded
  until the write. It is **idempotent**, which is what lets a transform walk a closure without
  keeping a visited set of its own.
- **`add` / `reserve` + `place`** are for objects the caller built. `reserve` exists because a
  catalog naming a page tree that names the catalog needs one of the two numbers before either
  object can be built.
- **`replace(from, id)`** is the third input: a slot that *stands in for* a source object, so that
  every reference to that object — from anywhere in the assembly — maps to the replacement. A
  page whose `/Parent` must name the new tree is the case that needs it, and without it a
  transform would have to rewrite every reference to every page it changed.

Numbering is **total and sequential from 1**, in the order the caller assembled, every generation
0. Not a preference: two sources may use one object number, so preserving them is impossible in a
merge, and RFC §9's byte determinism needs the numbering to be a function of the plan.

### 2. What it refuses, and what it repairs

Refuses, by name, in `SerializeError`: an assembly with no `/Root` (§7.5.5 requires one, and a
file this reader would refuse is not a file this writer will produce); a reserved slot never
filled (a caller's mistake, and the number was promised to whatever referred to it); an offset
past §7.5.4's ten digits; a sink that failed.

Repairs, counted rather than hidden — and both are the standard's own answer rather than a
tolerance:

- **A reference the assembly does not hold becomes `null`**, §7.3.10: "[a]n indirect reference to
  an undefined object shall not be considered an error by a PDF processor; it shall be treated as
  a reference to the null object." A transform makes these deliberately, at the edge of a piece,
  and `Written::dangling` is what lets it name every one.
- **`/Length` is re-derived from the bytes written**, §7.3.8.2. A source that lied does not get
  the lie copied into a file this program's name is on, and `Written::relengthed` counts the
  disagreements.

### 3. The sub-decisions RFC §10 left open

- **Form follows the sources**, ADR 0121's argument promoted from a section to a whole file:
  `Form::of` reads a document's own last section, `Form::of_all` takes a classic table unless
  **every** source uses a stream. Conservative on purpose — §7.5.8.1 makes a cross-reference
  stream a PDF 1.5 construct, and a merge of a 1.4 file that came out as a stream would have
  raised the output's version for a reason nobody asked for. A stream form raises the header to
  1.5 where it is lower, because a file whose header disowned its own cross-reference section
  would be one no §7.5.8.4 reader could recover.
- **No object streams.** §7.5.7's producer half is not implemented and its ledger row is now
  `partial` with the debt named. Generating them means deciding which objects may share a stream,
  how large one may grow and what `/Extends` says — each a decision RFC §6.5 puts under
  `optimize`. The cost is stated: a piece of a 1.5 document is larger than the pages it holds.
- **No encryption on the way out.** The serializer emits no `/Encrypt`, so a derivative of an
  encrypted source is unencrypted; `Assembly::has_encrypted_source` is the question a caller asks
  in order to say so, and `split` warns on every such piece. §7.6.4.4.7 to §7.6.4.4.9 stay
  `writer-side` for exactly as long as that holds.
- **No linearisation**, per `CLAUDE.md`'s amended entry.
- **§14.4's two identifiers are one value**, because the clause says so for a file being created:
  "[w]hen a PDF file is first written, both identifiers shall be set to the same value." Neither
  is the source's — two pieces of one document are two files, and a shared permanent identifier
  would assert they were versions of each other, which is what the clause's next paragraph uses
  the pair to decide. The value is a digest of the piece's own bytes, taken **as they go past the
  sink**, which is how RFC §12's memory rule survives a clause that wants a function of the file.
- **Nothing is buffered whole.** Each object goes to the writer as it is built and a stream's data
  crosses from the source's `Arc<[u8]>` straight to the sink, still encoded — never decoded, never
  examined. The one buffer is a stream's dictionary, which is small.

### 4. The evidence, in RFC §9's four layers

Layer 1, determinism: asserted in `tests/serialize.rs` and again per document in
`tests/split_corpus.rs`. Layer 2, self read-back: eleven tests in `tests/serialize.rs`, three of
them the hostile constructions this record's context asks for — an object referring to one the
output does not hold, a stream whose `/Length` lies, and a two-way cycle walked six times.
Layer 3 is `split`'s and is ADR 0818's. Layer 4 is `qpdf --check`, in `tests/split.rs`, in
principle 5's register.

And `fuzz/fuzz_targets/serialize.rs`: any bytes → a document → every object copied → written in
both forms → **re-opened with this tree's own reader**, which must find the catalog. RFC §11.3's
cost — "this project starts *producing* files other parsers read" — is why the round trip rather
than the write alone is the target.

## Consequences, and the two defects the corpus found

**`write::real` wrote six decimal places, and six is not enough.** The fractional branch was
`{:.6}` with the zeros trimmed, and it had been correct for as long as the only writer was
§7.5.6's update: a form field's `/Rect` survives that rounding. A serializer rewrites *every*
dictionary of every object it carries, and a Type 3 `/FontMatrix`, a shading's `/Coords` and a
function's `/C0` do not — `0.0009765625` becomes `0.000977`. **Seven corpus documents drew
differently after `split` because of it**, every one of them off by one antialiasing level on a
glyph's edge, and the walk found all seven on its first run. What is written now is the shortest
decimal that reads back as the same double, which Rust's `Display` for `f64` produces and which
never uses an exponent — so §7.3.3's prohibition is met by the same formatting that fixes the
precision, and Annex C's Table C.1 ("IEEE ... single or double precision") is the warrant.

The defect is worth its paragraph for a reason beyond the numbers: **it was a latent bug in code
that had been read and gated for seven hundred sessions**, and nothing could have found it until
a writer existed that rewrote dictionaries it had not authored. A new consumer of old code is an
instrument, and this is what it measured.

The second defect is `split`'s and is in ADR 0818.

- `pdf-syntax` gains one module and one fuzz target; `#![forbid(unsafe_code)]` is untouched and
  `Document` is still immutable — the serializer takes `&Document`s and owns only the output, so
  §3's oracle-purity argument survives verbatim.
- The ledger rows §7.5.2, §7.5.3, §7.5.4, §7.5.5, §7.5.8, §7.3.8.2, §7.3.10 and §14.4 all now
  describe a writer as well as a reader; §7.5.7 is `partial`. ADR 0816 has the table.
