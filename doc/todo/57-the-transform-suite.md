# 57 — The transform suite: what RFC 0002 still owes

Status: **open**.
Priority: 50-band. RFC 0002 §13's first question was answered on 2026-09-03 ("RFC 002 and 003
are approved"), and every verb it proposed is built — the list below. **What is owed is section 1's
four items, section 3's prompt and section 4's tranche**. Two of them the owner answered on
2026-09-06 and neither is built: RFC §13's second question, the DCT encoder that JPEG output and
`optimize --images` wait on ("Use mozjpeg-rs", `doc/questions/A01`), and its third, the
confinement tranche ("Yes, implement it "now"", `doc/questions/A02`).
Corpus witnesses: `issue11124.pdf`, `bug1065245.pdf`, `images_1bit_grayscale.pdf` (inline
images, `--native`); `issue21570.pdf` (a JPEG under an `/SMask`); `issue2177.pdf` (a crop box a
quarter of its media box); `attachment.pdf` (an `/EmbeddedFiles` tree to attach into);
`issue18823.pdf` (an optional-content configuration whose arrays are indirect) and
`issue15096.pdf` (one document stating a fully qualified field name twice with two values); the
suite's own gate runs over ISO 32000-2's PDF.
Clauses: §7.6.4.2 Table 22, asked through `pdf_model::restriction` (ADR 0803), its bit 11 by
`Operation::Assemble` (ADR 0818).
Code: `crates/pdf-transform/`, `crates/pdf-syntax/src/serialize.rs`, ADRs 0800, 0801, 0802,
0803, 0804, 0816, 0817, 0818, 0821, 0830, 0831, 0834, 0835, 0839, 0842, 0843, 0852, 0862, 0863,
0873, 1293.

## What is done

One line per landing; the argument is the ADR's, and what the verbs do is `doc/state-of-play.md`'s
paragraph "And a program can ask it for a *file* derived from a document".

- RFC 0002 §14's first landing — the seam, the range grammar, the name patterns, the report, the
  exit statuses, `render`, `images`, `attachments` — with §8.9.7's inline images, `--native` and
  §12.5.6.15's annotations as a third home (ADRs 0800, 0801); `images --no-mask`, `render
  --page-box` and `--no-annotations`, and `attachments --attach` on §7.5.6's incremental update
  (ADR 0802); the restriction policy asked once by `pdf_model::restriction::decide`, `--attach
  --to-page` and `--remove` (ADR 0803); `--format pgm` and `tests/writer_corpus.rs` (ADR 0804).

- `CLAUDE.md`'s authoring exclusion redrawn, `pdf_syntax::serialize` (RFC §10's
  structure-preserving serializer) and `split`, with `tests/split.rs`, `tests/split_corpus.rs` and
  a fuzz target re-reading what the writer wrote (ADRs 0816, 0817, 0818).

- `merge`, one clause per document-level reconciliation, a colliding fully qualified field name
  refused by name across sources; `tests/merge.rs`, `tests/merge_corpus.rs` and RFC §9's
  `split`-then-`merge` property gate (ADR 0821).

- `pages` over `merge::write`: §7.7.3.3's `/Rotate` written, a duplicated page a second page
  object, a page carrying a §12.7 widget refused by name; `tests/pages.rs` and
  `tests/pages_corpus.rs`, whose rotated comparison is still a measurement (section 1; ADRs 0830,
  0831).

- §14.7's structure tree carried by `split`, `merge` and `pages` through one
  `crates/pdf-transform/src/structure.rs` behind a `Host` trait, the parent tree under the output's
  own keys, a cross-source `/ID` collision `Refusal::StructureConflict` at exit 4, and
  `support::check_structure` in the corpus walks (ADRs 0834, 0835).

- `optimize`'s four lossless passes — §7.5.5's reachability, §7.5.7's object streams, §7.5.8's
  cross-reference stream, §7.4.4.1's `FlateDecode` kept only where smaller — with
  `tests/optimize.rs`, `tests/optimize_corpus.rs` (RFC §9's idempotence gate) and
  `support::check_optimized`; lossy image optimisation refused by name (section 1; ADRs 0842,
  0843).

- A piece carries the outline subset that reaches its pages, its labels through
  `merge::page_labels` and its named destinations subsetted; `split --at-bookmarks[=depth]`, with
  `Refusal::NoBookmarks` at exit 2 and `support::check_navigation` (ADRs 0862, 0863).


- Annex F's writer, `optimize --linearize` (`doc/questions/A03`):
  `pdf_syntax::linearize::serialize_linearized` and F.1's reader half `pdf_syntax::linearize::state`,
  read back by `tests/linearize.rs` through `support::linearized` and walked by
  `tests/optimize_corpus.rs`'s linearised arm (ADRs 1293, 1309, 1328, 1337).
- The foreign readback, `tests/foreign_corpus.rs` (ADRs 0839, 0852); section 5 is what it does not
  see.

## 1. What the suite still owes

- **What a piece does not carry.** §14.7's Table 354 `/Namespaces` is
  concatenated without being interpreted, so two sources using one namespace name state it twice,
  and the `/RoleMapNS` construction that would let each source keep its own role map under its own
  namespace is not taken, because a namespace name "should take the form of a uniform resource
  identifier" and this program has no basis for inventing one; and a piece still leaves `/Metadata`,
  `/Threads`, `/SpiderInfo`, `/Collection`, `/Perms`, `/Legal`, `/Requirements` and `/DPartRoot`
  behind, each named in the report where the source states one — with each page's `/DPart`, which
  Table 31 permits only inside a carried hierarchy, and web capture's `/IDS` and `/URLS` trees, which
  neither a piece nor a merge carries (ADR 1461).
- **The aligned rotated comparison — the alignment is done (ADR 0873) and the assertion is not.**
  `render` states the sub-pixel strip of raster the page does not reach (`Origin::Page`'s
  `overrun_across` and `overrun_down`), and `tests/pages_corpus.rs` derives the whole-column shift
  from it; the walk prints the rotated pages' worst mean, least similar tile and byte-identical
  count (its `doc/todo/02` §2 line), and ADR 0873 has the reversed shift that calibrated it.
  **What is still owed is the assertion**, and it needs a tolerance stated against the two terms
  that remain: the grid, which turns with the page and which no integer shift undoes
  (`issue15150.pdf`, at an overrun of zero), and the sub-pixel remainder an integer shift leaves,
  at most half a pixel by construction. Both are
  statements about this renderer's antialiasing rather than about the writer, so a round that picks
  a number has curve-fitted and a round that derives one has finished the item.
- **A per-input password for `merge`.** `viewer_core::Secret` is deliberately not `Clone`, so one
  `--password-fd` opens one document and a merge of several encrypted sources is a usage error,
  by name. A per-input spelling or several `--password-fd`s would lift it; nobody has asked.
- **JPEG output from `render`, and `optimize --images`** (RFC §6.5's lossy half), wait on the DCT
  encoder the owner chose: `mozjpeg-rs` (`doc/questions/A01`), with the owner's condition that
  nothing is re-encoded unless a format is explicitly requested — an extraction with none asked
  for writes the stream's own bytes, as `images --native` does. It is not in the tree, and
  `doc/stack.md` is where it enters. Until then the flag is refused by name, and ADR 0842 §5 is
  why: without an encoder "recompress as DCT where smaller" cannot be done at all and downsampling
  to `FlateDecode`-compressed raw samples makes a photograph larger, so the keep-the-original rule
  would keep every image and the flag would do nothing while claiming to.

## 2. The RFC 0003 hand-off — taken

`pdf-vfs`, RFC 0003's core, consumes the seam — `Plan::Split`, `Plan::Render`, `Plan::Images`,
`Plan::Attachments` and `Plan::Update` — and a test holds what it derives against `apply`'s own
output, so a second implementation fails a gate. The seam gained one thing, `Source::document`,
because a consumer that opened the document itself would need a second `Secret`, which
`viewer_core::secret` makes impossible (ADRs 0840, 0841, `doc/todo/58`).

## 3. One thing still without its dependency
- `--password-prompt`: an interactive prompt that suppresses echo needs a terminal-mode
  dependency (`doc/stack.md` decides), or a host that owns a terminal. `--password-fd` is the
  scripted route and is what exists.

## 4. The confinement tranche — RFC §13 question 3, answered "now" and not built

The owner answered `doc/questions/Q02` on 2026-09-06 — "Yes, implement it "now"" (`A02`) — and
the suite still runs in one process, so the command line parses untrusted bytes unconfined; ADR
0800 §6 states that cost. The worker split is a transport change on the `pdf-view-worker`
pattern — plan in, report out, sources and sinks as descriptors the broker opened — and the seam
was written so that it is one; `viewer-confined` is the precedent, and `pdf-vfs`'s worker already
serves the seam confined into `MemorySinks` (`crates/pdf-vfs/src/serve.rs`). `--attach` adds one
thing to the plan that crosses: the payload's bytes, which are a descriptor like a source's.

## 5. What the gates do not see

The transform gate's floor is a wall-clock number (`PAGES_PER_SECOND_FLOOR` in
`tests/gate.rs`, derived at twenty-four threads in ADR 0801; under `tools/bounded.sh --lock` the
run gets four, ADR 1766), and ADR 0801 §2's defect — a cost that shows only where a CPU-second is
worth what it was, at two or four threads — would pass it. The instrument for that class is the
thread curve in ADR 0801, taken by hand with `RAYON_NUM_THREADS`; a round that touches `render`'s
parallel shape re-takes it, and ADR 0804 has the last one taken.

**The foreign readback** — `tests/foreign_corpus.rs`, ADR 0839 — and what it leaves unseen:

- Five writers' output — `attachments --attach`, `split`, `merge`, `pages` and `optimize` — goes
  through `qpdf --check`, `pdftoppm`, `mutool draw`, `mutool show` and `pdfinfo`, and every foreign
  reading of a derived page is compared with that *same* reader's reading of the source page —
  never with ours, which is the oracle's question and would make a disagreement unattributable
  (traps 3 and 9). **`redact`, `archive` and `optimize --linearize` are not derived by it**, so
  no foreign reader has been shown what they write.
- **It found one defect on its first run** (ADR 0838), in the one thing no other instrument here
  can see: §14.7's parent tree, which only an assistive processor reads (ADR 0835 §5's prediction).
- **What it does not cover, and what a later round owes.** It is a *sample* — every tagged document
  plus every `STRIDE`th — because a document costs nineteen foreign invocations across the five
  writers on an untagged multi-page document; the whole corpus
  is a bigger instrument and would want the reference cache `pdfref` already has. It draws page 1
  only, so a derived document's later pages are unread by anybody but us. It skips a document that
  needs a password, because the foreign readers would have to be given it too. It asks mupdf about
  §14.7 and poppler only about `/MarkInfo` — no installed tool reports a *rendered* structure tree,
  and `mutool draw -F stext -O structure` is not in mupdf 1.28. And it says nothing about the
  outline, the name trees or the form, all of which a foreign reader could be asked about and none
  of which any of them prints.
- **One document is out of the rendering comparison on wall clock alone.** `issue19517.pdf` costs
  poppler and mupdf about their whole budget on the source *and* on the derived file (the figures
  are `foreign_corpus.rs`'s header), so both readers time out on it about as often as not; a
  timeout takes it out of that reader's comparison rather than failing the run (ADR 0839 section
  4). A reference cache, or a budget scaled to what the source render cost, would put it back.
- **qpdf's verdict is split into its two steps** (ADR 0852): a warning where the source had none
  fails the run as an error does.
