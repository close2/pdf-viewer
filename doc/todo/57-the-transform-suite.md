# 57 — The transform suite: what RFC 0002 still owes after its first three landings

Status: **open**, on the long-lived branch the transform rounds share (`round-867` onward).
Priority: 50-band — RFC 0002 §13's first question was answered on 2026-09-03 ("RFC 002 and 003
are approved"), so the serializer, four verbs and §14.7's structure tree are done; what is left is
**`optimize`**, `split --at-bookmarks`, the RFC 0003 hand-off, and the §13 questions the owner has
not been asked again. **The foreign readback is no longer among them**: session 898 built it
(ADR 0839), and §5 below records what it found rather than what it is for.
Corpus witnesses: `issue11124.pdf`, `bug1065245.pdf`, `images_1bit_grayscale.pdf` (inline
images, `--native`); `issue21570.pdf` (a JPEG under an `/SMask`); `issue2177.pdf` (a crop box a
quarter of its media box); `attachment.pdf` (an `/EmbeddedFiles` tree to attach into);
`issue18823.pdf` (an optional-content configuration whose arrays are indirect) and
`issue15096.pdf` (one document stating a fully qualified field name twice with two values); the
suite's own gate runs over ISO 32000-2's PDF.
Clauses: §7.5.7's producer half, which the serializer does not emit (item 1); §7.6.4.2 Table 22
(item 3).
Code: `crates/pdf-transform/`, `crates/pdf-syntax/src/serialize.rs`, ADRs 0800, 0801, 0802,
0803, 0804, 0816, 0817, 0818, 0821.

## What is done

RFC 0002 §14's first landing — the seam, the range grammar, the name patterns, the report, the
exit statuses, and `render`, `images`, `attachments` (read) — ADR 0800, session 867. Session 868
(ADR 0801): the CPU-time question answered by measurement and the font cache per rayon split
fixed; the transform gate with RFC §12's perf floor; §8.9.7's inline images and `--native`;
§12.5.6.15's annotations as the third home `attachments` reads. Session 870 (ADR 0802): `images
--no-mask` and the mask beside every native JPEG; `render --page-box` over Table 31's five boxes
and `--no-annotations`; and **`attachments --attach`**, the suite's first writer consumer, on
§7.5.6's incremental update alone — the source's bytes intact, three objects and a rewritten
holder after them, deterministic unless `--date` is given. Session 872 (ADR 0803): the
restriction policy done once in `pdf_model::restriction` — every Table 22 bit named, the
transform's three operations beside the viewer's two, the four levels and an exhaustive verdict
asked once by `decide`, §12.8.2.2's certification read against all five; **`--attach
--to-page`**, §12.5.6.15's annotation on a page with this tree's own icon; and **`--remove`**,
the tree without the entry and the objects marked free by §7.5.4's second mechanism. Session
875 (ADR 0804): **`--format pgm`** for `render` and `images`, §10.4.2.2's grey through the one
statement of the NTSC weights the tree has, over the RGB the interpreter's own conversion
already produced, with the mask beside a netpbm image; the `/Names`-indirect holder fixture,
and the walk's census of every holder shape the corpus has; and **the writer over the corpus**
— `tests/writer_corpus.rs`, every corpus document the suite opens attached into, read back,
the file removed and filed on page 1, on `doc/todo/02` §2's sequence with its refusals counted
by reason.

**Session 886 (ADRs 0816, 0817, 0818): the amendment, the serializer, and `split`.** The owner
ratified RFC §11.1 on 2026-09-03 — "RFC 002 and 003 are approved" — so `CLAUDE.md`'s authoring
exclusion is redrawn, the ledger's `writer-side` status is redefined by the serializer's
boundary, and §7.5.7 moved to `partial` for the one producer half no writer here emits.
`pdf_syntax::serialize` is RFC §10's structure-preserving serializer: an `Assembly` of copied,
synthesised and replaced objects, renumbered totally and sequentially, written as §7.5.2's
header, a body of indirect objects, §7.5.4's table or §7.5.8's stream in the sources' own form,
§7.5.5's trailer and §14.4's two identifiers — with stream bytes crossing encoded and only
`/Length` re-derived, and a reference the output does not hold written as §7.3.10's null and
counted. `split` is the first verb on it, with `tests/split.rs`, `tests/split_corpus.rs` and a
fuzz target that re-reads what the writer wrote. Everything below is what the RFC proposed and
no round has taken, in the order the next round should.

**Session 888 (ADR 0821): `merge`, and one clause per reconciliation.** RFC §6.2's other half, on
the serializer session 886 built. The machinery was `Assembly`'s already, so the round's substance
is the document-level reconciliations, each derived from a sentence: §8.11's groups unioned and
their initial states rewritten as one default configuration on §8.11.4.3's own parenthesis about
`/BaseState`; §7.9.6's name trees merged with a colliding key renamed deterministically and every
`/Dest` and `/GoTo` naming it rewritten, because §12.3.2.4's two homes are one namespace to this
tree's reader; §12.3.3's outlines spliced into one chain rather than parented under a synthesised
item Table 151 would make this program invent a `/Title` for; §12.4.2's labels one entry per page;
§14.11.5's array on each source's own pages where several contribute — the clause's *second* home,
which `pdf_model`'s colour path now reads for the same clause's sake; §12.7's Table 224 entry by
entry with §12.7.4.2's fully qualified field name **refused by name** across sources and carried
with a warning within one; and §12.8.1's signature crossing without its `/V`. `apply` opens every
source the plan names and asks the restriction policy per document. `tests/merge.rs`,
`tests/merge_corpus.rs` and RFC §9's `split`-then-`merge` property gate; the walk found two defects
on its first run, both real.

**Session 893 (ADRs 0830, 0831): `pages`, and what a rotated raster cannot be asked.** The other
half of §6.2 and the RFC's own open question about the two verbs, answered by the count of files:
`pages` reads one document, `merge` reads several, so `--insert` takes a range of *this* document
and a path in it is a usage refusal naming `merge`. The engine is `merge::write` over a list of
`Placement`s, so every reconciliation session 888 derived applies to a page *leaving* with no
second construction. What is this verb's own: §7.7.3.3's `/Rotate` written — an unsigned angle
absolute, a signed one relative to the value §7.7.3.4 gives the page, reduced modulo a whole turn,
a zero written as no entry, an angle that is not a multiple of 90 refused by name; §12.5.3 read and
found to bear on the viewer rather than the file, so no annotation is touched; Table 31's one
`/Parent` making a duplicated page a second page object with its own annotations, and a page
carrying a §12.7 widget refused by name because §12.7.4.2 makes a field's fully qualified name its
identity. `tests/pages.rs` and `tests/pages_corpus.rs`, whose rotated-page comparison is a
*measurement* rather than an assertion for the reason ADR 0831 records.

**Session 897 (ADRs 0834, 0835): §14.7's structure tree, carried by all three verbs.** The debt
sessions 888, 891 and 893 each named as the suite's largest. `crates/pdf-transform/src/structure.rs`
reads every contributing document's `/StructTreeRoot`, keeps the elements whose content is on a
carried page together with the ancestors that hold them, prunes the content items that name a page
the output does not hold, and writes Table 354's root whole — `/K`, §14.7.5.4's parent tree with the
output's **own** keys, `/ParentTreeNextKey` restated, the merged `/RoleMap` and `/ClassMap`, an
`/IDTree` over the kept elements, the three list-valued entries concatenated, and Table 353's
`/MarkInfo` with `/Marked` a conjunction over the sources. One implementation for `split`, `merge`
and `pages`, behind a `Host` trait, so §14.7 is read once. The marked-content identifiers inside the
carried content streams are **not** rewritten and the assumption was checked rather than made:
§14.7.5.2 scopes an `/MCID` to its own content stream and §14.7.5.4 makes it an index into the array
its key names, so carrying the array at its own length moves both ends of the index together. Three
namespaces two sources can collide in get three answers from three clauses — §14.7.3's role map is
an *approximation* (NOTE 1), so the first source's wins with a warning; §14.7.6.2 closes the set of
things that name a class, so a colliding class is renamed and every `/C` follows; Table 355 makes an
`/ID` unique and the set of things that name one is open (§14.8.5, Annex E), so a cross-source
collision is `Refusal::StructureConflict` at exit 4. ADR 0831 §2's dangling key is superseded on its
own terms: where the output states a tree, every §14.7.5.4 key in it is the output's own or absent.
The three corpus walks gained `support::check_structure`, four clause-derived properties asked of
every output.

## 1. The verb left, on the serializer that exists

- **`optimize`** (RFC §6.5) is where §7.5.7's producer half is owed: the serializer generates no
  object stream, so a piece of a 1.5 document is larger than the pages it holds, and
  `--object-streams=generate|disable` is the knob the RFC names. Reachability pruning belongs here
  too — `split` deliberately over-copies rather than inventing a pruning policy of its own (ADR
  0818).
- **`split --at-bookmarks`**, the one mode of `split` that did not land: it wants
  `pdf_model::retrieval::sections`, which exists, and an outline subset for the piece, which does
  not.
- **What a piece does not carry** is now the shorter list, because `merge` built four of the five
  and session 897 built the fifth: the outline subset whose destinations survive, §12.4.2 page
  labels per piece, and name-tree entries still referenced are each `merge.rs`'s construction
  pointed the other way, and a round taking them should reuse it rather than write a second one.
  §14.7's structure tree is **carried** since session 897 (ADR 0834), which is what this bullet
  used to name as the largest single thing the suite owed. What is left of it is small and named
  there: Table 354's `/Namespaces` is concatenated without being interpreted, so two sources using
  one namespace name state it twice; and the `/RoleMapNS` construction that would let each source
  keep its own role map under its own namespace is not taken, because a namespace name "should take
  the form of a uniform resource identifier" and this program has no basis for inventing one.
- **The aligned rotated comparison.** `tests/pages_corpus.rs` measures the turned-raster
  comparison and does not assert on it, because a page `W` units wide at scale `s` is
  `ceil(W × s)` pixels wide and the leftover sliver sits on a different edge after the page turns
  than after the raster does — worth a whole pixel, measured exactly on `issue2761.pdf` (mean
  absolute difference 0.000 once one column is allowed for, 19.4 without). What would make it an
  assertion is `render` reporting the sub-pixel offset it placed the page at, so the walk can
  *derive* the whole-pixel shift instead of searching for one. That is a change to the renderer's
  report, not to the walk. ADR 0831 §1 has the measurement.
- **A per-input password for `merge`.** `viewer_core::Secret` is deliberately not `Clone`, so one
  `--password-fd` opens one document and a merge of several encrypted sources is a usage error
  today, by name. A per-input spelling or several `--password-fd`s would lift it; nobody has asked.
- JPEG output from `render` waits on §13 question 2, the DCT encoder.

## 2. The RFC 0003 hand-off

The owner's sequencing, 2026-09-03: RFC 0003 (the file-system faces) follows *after* this
stream's writing verbs land, because running them in parallel would have both implementing the
same things. What RFC 0003 consumes is the seam — a `Plan`, `Sources`, `Sinks`, a `Policy`, a
`Budget` and a `Report`, with no path, clock or process inside any of them — and the seam has now
survived six verbs' worth of contact, one of which writes whole files.

**Taken in session 899** (ADRs 0840, 0841, `doc/todo/58`): `pdf-vfs` is RFC 0003's core, read side
only, and it consumes the seam for six of its eight generators — `Plan::Split` for a page taken
out, `Plan::Render` for a page drawn, `Plan::Images` for a page's pictures, `Plan::Attachments` for
an embedded file — with a test holding the first byte for byte against `apply`'s own output, so a
second implementation of any of them fails a gate rather than going unnoticed. **The seam gained
exactly one thing**, and it is three lines: `Source::document`, because three of that crate's
generators are `pdf-model` readers no verb covers and a consumer that opened the document itself
would need a second `Secret` — which `viewer_core::secret` deliberately makes impossible. That is
the whole cost of the hand-off, and it is the strongest evidence the seam's shape was right.

## 3. One thing still without its dependency

- `pdf_transform::Operation` moved into `pdf_model::restriction` in session 872 (ADR 0803);
  nothing of the policy is in this crate any more but the words on stderr. Table 22's bit 11 is
  consumed since session 886, by `Operation::Assemble` (ADR 0818).
- `--password-prompt`: an interactive prompt that suppresses echo needs a terminal-mode
  dependency (`doc/stack.md` decides), or a host that owns a terminal. `--password-fd` is the
  scripted route and is what exists.

## 4. The confinement tranche — RFC §13 question 3, defaulted to in-process

ADR 0800 §6 states the cost. The worker split is a transport change on the `pdf-view-worker`
pattern — plan in, report out, sources and sinks as descriptors the broker opened — and the seam
was written so that it is one; `viewer-confined` is the precedent. Taken when the verbs settle,
or earlier if the owner requires it before the first release. `--attach` adds one thing to the
plan that crosses: the payload's bytes, which are a descriptor like a source's.

## 5. What the gates do not see

The transform gate's floor is a wall-clock number over 24 threads, and ADR 0801 §2's defect —
a cost that shows only where a CPU-second is worth what it was, at two or four threads — would
pass it. The instrument for that class is the thread curve in ADR 0801, taken by hand with
`RAYON_NUM_THREADS`; a round that touches `render`'s parallel shape re-takes it, and ADR 0804
has the one taken in session 875.

**The foreign readback is taken** — `tests/foreign_corpus.rs`, ADR 0839 — and this section used to
be five paragraphs arguing for it. What it is worth recording now is what remained after it:

- All four writers' output goes through `qpdf --check`, `pdftoppm`, `mutool draw`, `mutool show`
  and `pdfinfo`, and every foreign reading of a derived page is compared with that *same* reader's
  reading of the source page — never with ours, which is the oracle's question and would make a
  disagreement unattributable (traps 3 and 9).
- **It found one defect immediately** (ADR 0838), and it was in the one thing no other instrument
  here can see: §14.7's parent tree, which only an assistive processor reads. That is ADR 0835 §5's
  prediction, confirmed on the first run.
- **What it does not cover, and what a later round owes.** It is a *sample* — every tagged document
  plus every eighth — because a document costs up to fourteen foreign invocations; the whole corpus
  is a bigger instrument and would want the reference cache `pdfref` already has. It draws page 1
  only, so a derived document's later pages are unread by anybody but us. It skips a document that
  needs a password, because the foreign readers would have to be given it too. It asks mupdf about
  §14.7 and poppler only about `/MarkInfo` — no installed tool reports a *rendered* structure tree,
  and `mutool draw -F stext -O structure` is not in mupdf 1.28. And it says nothing about the
  outline, the name trees or the form, all of which a foreign reader could be asked about and none
  of which any of them prints.
- **One document is out of the rendering comparison on wall clock alone.** `issue19517.pdf` costs
  poppler 23.9 s and mupdf 17.6 s on the source *and* on the derived file, against a 20 s budget,
  so both readers time out on it about as often as not; a timeout takes it out of that reader's
  comparison rather than failing the run (ADR 0839 section 4). A reference cache, or a budget
  scaled to what the source render cost, would put it back.
