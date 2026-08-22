# 661 — An identifier two content streams may share

**A number unique within one content stream was being used as if it were unique within a page, and
two features had rested on it for hundreds of sessions.** 2026-08-22. ADR 0488. Parallel round,
branched beside 660, 662 and 663.

## What the clause says the scope is

The content stream, and it says so three times over. §14.7.5.2 makes the `/MCID` "an integer
marked-content identifier that uniquely identifies the marked-content sequence within its content
stream"; the same clause permits a form `XObject`'s stream to hold sequences of its own and writes
the collision out in Example 5, page and form both numbering from zero; Table 357's `/Stm` names
which stream, and its absence is itself a `shall` — the sequence "shall be contained in the content
stream of the page identified by Pg". §14.7.5.4 makes the route back per stream by construction, one
parent tree entry "for each content stream containing at least one marked-content sequence that is a
content item", keyed from the page object *or* "the stream dictionary of a form or image XObject".

Errata Collection 3's Issue #308 adds the consequence as a NOTE under §14.7.5.4 (`spec-errata emit`,
p. 750): identifiers are scoped by content stream and start at zero, so the same one may reappear
across pages or `XObject`s.

**So the clause answers cleanly and the defect was ours.** ADR 0486's guess — that this might be an
ambiguity — was wrong in the right direction.

## The population

`pdf-model --example mcid_stream_census`, built this round and calibrated against a planted
collision before it was believed. Over the SafeDocs crawl: **65 703 of 65 944 documents opened,
23 447 with a structure tree, 701 with a page marked by two or more content streams, 42 with a page
where two of them share an identifier** — 163 pages, 7334 identifiers — and 545 stating a Table 357
`/Stm` at all. Over pdf.js + `doc/corpora` + `doc/`: 1245 opened, 153 tagged, **one** document with
two marking streams and one collision (`issue15372.pdf`, whose form's `/MCID 0` drew nothing, so the
collision cost that file nothing visible).

17 of the crawl's 42 also state `/Stm` — conforming files this tree read wrong outright.

## What was built

`content::ContentStream` on every `MarkedSpan`; Table 357's `/Stm` on
`structure::Child::MarkedContent`; `content::named_sequences` as the one place the match is made,
read by `Tree::logical_text`, `Tree::logical_range` and `viewer_core::accessibility`'s `ranges` and
`marked_extent` — ADR 0134's text range and ADR 0486's rectangle, which are the two things the round
was about. Two more the round found: `Interpreter::enter_stream_structure` gives §14.9's `/Alt` the
*stream's* own parent tree, and `Tree::stream_owners` adds the fourth route `elements_on_page` did
not have, so an element whose sequences live inside a form stops being pruned as another page's.

**One recovery, with its cost written down.** Read strictly, two corpus documents that put every
sequence in one form and name each with a bare integer say nothing at all — 61 elements lost their
place and the accessibility ratchet failed. So where the page's own stream holds no such identifier
and exactly one other stream does, that one is answered; two carrying it answers nothing rather than
both. An appearance stream is `Unnameable`, which fixes the half that misleads and leaves the other
direction unreachable; no document in either population exercises it.

## Gates

Full sequence, because `pdf-model` changed. `fmt`, `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` and the fuzz `check` all silent · `nextest` **2412 passed, 17 skipped** ·
doctests · conformance **163 + 5 + 1**, 1006 quotations verbatim · corpus green · oracle **908
agrees, 65 contradicted, 786 ambiguous** — unchanged, so no pixel moved · `render-quorra` **957
pages, 933 agree, 22 differ, 2 refused** · `fixed_documents` **40 checked, 0 absent** · text
extraction **99.2% over 974 documents, 99.8% over 40, 98.26% of matched word boxes in bounds** ·
selection census 0 disagreements with the readback, 0 offsets misnamed · dates, XMP, JPEG 2000 ·
accessibility census **102 853 elements, 93 267 placed by their own marks, 1336 with no place, 876
of 876 untagged pages honest, 0 invented** — every figure equal to 658's.

## Files

`crates/pdf-model/src/content.rs` and `content/{report,marked,run,xobject,annotations}.rs` ·
`crates/pdf-model/src/structure.rs` · `crates/pdf-model/tests/marked_content_scope.rs` (new, 6
tests) · `crates/pdf-model/tests/logical_structure_example.rs` ·
`crates/pdf-model/examples/{mcid_stream_census,element_bounds_census}.rs` ·
`crates/viewer-core/src/accessibility.rs` and `tests/headless.rs` · `doc/conformance/ledger.toml`
(§14.7.5.2 and §14.7.5.4) · `doc/todo/31-accessibility-host.md` · `doc/todo/README.md` ·
`doc/adr/0488-…`.
