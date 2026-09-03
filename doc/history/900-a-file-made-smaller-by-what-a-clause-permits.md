# 900 — A file made smaller by what a clause permits

2026-09-03. Argued in [ADR 0842](../adr/0842-a-file-made-smaller-by-four-passes-each-derived-from-a-clause-and-none-of-them-touching-a-mark.md)
and [ADR 0843](../adr/0843-what-a-corpus-walk-can-ask-a-file-that-is-supposed-to-say-the-same-thing.md).
The tenth implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`. `main` had moved — session 894's merge and session 895's PDF/A
RFC — so it was merged in first, as a fast-forward; nothing it carried touched `pdf-transform`.

The round's subject is `optimize`, the verb that completes RFC 0002's set, and the producer half
of §7.5.7 it finally forces. That debt has a date: session 886 landed the serializer, wrote down
that it "generates no object stream at all", moved §7.5.7's ledger row to `partial` and named the
three decisions a generator owes — which objects may share a stream, how large one may grow, what
`/Extends` says. All three are now answered in the clause's own words, and the row is
`implemented`.

Touched: **`crates/pdf-transform/src/optimize.rs`** (new, the verb), **`crates/pdf-transform/tests/optimize.rs`**
and **`crates/pdf-transform/tests/optimize_corpus.rs`** (new, the fixture tests and the fifth
corpus walk), `crates/pdf-syntax/src/serialize.rs` (`Options`, `ObjectStreams`, `Streams`, the
§7.5.7 writer, the recompressor, Table 18's type 2 entries), `crates/pdf-syntax/src/document.rs`
(three helpers widened to `pub(crate)`), `crates/pdf-syntax/src/lib.rs`,
`crates/pdf-syntax/tests/serialize.rs` (three tests), `crates/pdf-transform/src/lib.rs`
(`Plan::Optimize`, `Origin::Optimized`, the report), `src/bin/pdf-transform.rs` (the verb, its
flags, two refusals by name), `src/split.rs`, `src/merge.rs`, `src/pages.rs` (the call sites, and
three stale claims), `tests/support/mod.rs` (`check_optimized`), `fuzz/fuzz_targets/serialize.rs`;
`doc/conformance/ledger.toml` (seven rows), `doc/todo/02-every-round.md` (a gate line and a
stale row), `doc/todo/57-…`; ADRs 0842 and 0843, this file.

## 1. The exclusion decided the design before any argument about quality did

RFC 0002 §6.5 surveys two schools and calls one of them the proposal. It did not have to argue
the point on quality grounds, because `CLAUDE.md`'s amended exclusion had already decided it.
Ghostscript's `-sDEVICE=pdfwrite` interprets a document down to marks and writes a new one; the
exclusion's boundary is **"does the operation invent marks?"**, and its sentence about this suite
is that "every content stream in their output is a producer's, carried byte for byte **or
recompressed without reinterpretation**". A re-distiller's content streams are its own. So the
structure-preserving school is not the better answer here, it is the only one in scope, and the
whole verb is four passes that touch the *file* and nothing on a page.

The same sentence's second arm is what makes recompression legitimate at all, and it is the one
place this round had to be careful: what may change is §7.4's encoding, and what may not change
is what the encoding decodes to.

## 2. Four passes, and the one measurement that is worth reading twice

The passes and their clauses are ADR 0842 §1. The corpus walk attributes them, over the 961
documents the verb rewrites:

| the file written with | saved |
|---|---|
| the serializer copying what it was given | **−11.21%** |
| + §7.5.5 reachability pruning | −0.63% |
| + §7.4 recompression | 14.06% |
| + §7.5.7 object streams | **26.71%** |

The first row is what changes how the verb is understood. A serializer copy is **larger than its
source**, by more than a tenth. That is not a defect and it is not news — ADR 0817 wrote it down
as a stated cost, because a 1.5 document's compressed objects come out at the outermost level —
but until this round it was a sentence rather than a figure, and the figure says that pruning
spends its whole yield undoing it before any of the saving starts.

## 3. §7.5.7's three decisions, and the two rules met by construction

The clause's prohibition list is walked bullet by bullet in `serialize::packable`, and **the two
bullets this writer satisfies without testing anything are named rather than omitted**: every
output object has generation 0, and there is no encryption dictionary because the serializer
emits no `/Encrypt`. That second one is also how Errata Collection 3's Issue #439 bullet — an
*encrypted* document's catalog — is met, which is worth knowing rather than assuming, because in
an unencrypted file the catalog may be compressed and here it is. The linearized-file bullet is
conditional on a construct `CLAUDE.md` excludes until Annex F is ratified, so it does not bind,
and it is written down so that whoever ratifies Annex F finds it. **A rule met by accident is a
rule waiting to be broken** is the reason for all of that prose.

NOTE 4 asks for a limit and gives no number, so 200 objects or 64 KiB is a measured choice: 50/16
KiB saves 13.52% of the sample, 500/256 KiB 13.64%, these two 13.62%. Two hundredths of a point
separates the chosen pair from four times the ceiling, so the curve is flat where it sits and the
smaller pair is the one that honours NOTE 4's own reason for asking.

`/Extends` is written, and the argument is NOTE 4's next sentence: a group of streams cut only
because "the number of objects in an individual object stream needs to be limited" is exactly the
collection the entry describes. A chain is a directed acyclic graph. This tree's own reader
ignores `/Extends` entirely, which was the argument for omitting it and is not a good one — the
file is written for readers other than this one.

## 4. Idempotence is a design constraint, not a test that came later

RFC §9 asks for it as a property gate. It turned out to be the thing that *decided* two rules
before either was ever run.

A stream's `/Length` may be an indirect reference, and the serializer always re-derives `/Length`
as a direct integer. So the object the source stated it in is referred to by nothing in the
output — and a *second* `optimize` would prune it, and the two files would differ. The walk
therefore does not follow `/Length` at all, which is §7.3.8.2's own reading ("[t]he number of
bytes from the beginning of the line following the keyword stream" is a statement about the file
being written) arrived at from the other end.

The same shape a second time: a recompressed stream states `/Filter` and `/DecodeParms` directly,
so a source that stated either *indirectly* would leave the same kind of orphan. The recompressor
refuses such a stream outright rather than the walk chasing the consequence, because refusing is
the cheaper of the two fixes and changes nothing a reader sees.

**Neither was found by a gate.** Both were found by asking, before writing the walk, what the
second run would see that the first had left behind.

## 5. Refusing is the design, and where each refusal comes from

`recompressed` answers `None` for seven conditions, and the list is in ADR 0842 §3. The one worth
repeating here is **damage**: a decode that came back short has bytes that stop before what the
file says the stream is, and re-encoding those would write a *whole* stream over a truncated one
and lose the fact. `Decoded::damage` has existed since ADR 0343 for readers to report with; this
is the first writer that has to refuse on it.

An image codec is the one thing that stops the walk instead of refusing it, so
`[/ASCII85Decode /DCTDecode]` becomes `[/FlateDecode /DCTDecode]` with the producer's JPEG bytes
untouched inside — `Document::image_stream`'s reading of the same chains, "[o]nly the last entry
can be a codec", used to decide where to stop rather than what an image is.

And qpdf's rule for `--optimize-images` is adopted for *every* stream: one that fails to shrink
keeps what its producer wrote. It is also half of why the verb is idempotent — a second pass over
this writer's own output finds nothing left to save and counts nothing, which
`tests/optimize.rs::a_stream_that_does_not_shrink_is_carried_and_the_report_says_so` asserts on
both sides.

## 6. Lossy is not here, and the absence is a decision rather than a gap

RFC §6.5 proposes `--images downsample=…,quality=…`. §13's second question makes it conditional
on a DCT encoder this tree does not have, and the owner has not been asked. What settles it is
not "later": **without an encoder the feature cannot be honest.** "Recompress as DCT where
smaller" cannot be done at all, and downsampling to `FlateDecode`-compressed raw samples makes a
photograph larger — so the keep-the-original rule this verb adopts everywhere else would keep
every image, and the flag would be a switch that does nothing while claiming to do something. So
there is no flag, `--images` anything is a usage refusal naming the dependency and the RFC
question, and `doc/todo/57` carries it beside `render`'s JPEG output, which waits on the same
answer. One question, two features.

`--linearize` is refused the same way, printing `CLAUDE.md`'s own sentence: "Annex F stays
excluded until linearisation is separately ratified."

## 7. The walk earned its round four times over

Every one of the four is in ADR 0842 §7, and none was found by reading. Two are worth the
retelling here because of *how* they hid.

**A recompressed image's `/DecodeParms` was rebuilt in the source's numbering.** The tail of a
filter chain stopped at an image codec was reconstructed from `Document::decode_parms`, so
`bitmap-p32-eof.pdf`'s `<< /JBIG2Globals 3 0 R >>` came out of the rewrite naming whatever object
3 had become, with the globals it actually needed written and referred to by nothing. **Every
raster in that run was bit-identical**, including this document's, and the two gates that saw it
were the output's own closure — an object no path from `/Root` reaches — and a second `optimize`
producing a smaller file. That is ADR 0843 §2 and §3 being exactly what they were built to be.

**Four documents rewrote into files with no page.** All four are files this tree opens only by
recovering what their trailer misstates: `/Root` naming an object that is not there, `/Root`
naming an information dictionary because every object is misfiled, a catalog whose `/Pages` names
nothing. `pdf_model::Pages` finds their pages by looking for what Table 31 describes, which is
right for a *reader*. It is wrong for a writer, and the clauses say so in the same breath —
Table 15's `/Root` and Table 29's `/Pages` are each "( Required; shall be an indirect reference )"
— so a rewrite of a reconstruction would be a file stating a structure no producer wrote. Refused
by name, which is trap 5, and the refusal prints the clause.

## 8. Three claims about this tree had decayed, and the round found them by needing them

Session 897 carried §14.7's structure tree in all three verbs. Four places went on saying it did
not, for three rounds:

- `pages.rs`'s module comment had a whole section headed "§14.7's structure tree, said plainly",
  whose first sentence was "**No verb of this suite carries it**, `pages` included";
- `split.rs`'s not-carried list named "the structure tree" among the entries a piece leaves
  behind;
- `doc/todo/02`'s change-map row described `pages_corpus` as holding "§14.7's *absent* structure
  tree to what the clauses say";
- three paragraphs of the CLI's `--help`, including one telling the user that "a tagged document
  loses its tagging" to every verb of the suite.

None was found by a sweep. They were found by reading the neighbouring verbs in order to write a
sixth one, which is `doc/habits.md`'s ledger section wearing a different hat: **a claim about
this tree decays exactly the way a ledger row's does**, and the round that touches the code is
the round placed to notice.

## 9. What was looked at

- Every run of the corpus walk — six of them, four of which failed — and `qpdf --check` on the
  rewritten `PDF20_AN001-BPC.pdf` and on ISO 32000-2 itself: 1023 pages, 19 206 210 bytes to
  18 184 101, 111 244 objects to 100 741, 95 164 of them compressed into 498 carriers, and the
  second rewrite byte-identical to the first.
- `bitmap-p32-eof.pdf`'s six objects and `issue5280.pdf`'s eleven, read out of the raw bytes,
  which is how the two `/DecodeParms` defects were diagnosed rather than guessed at.
- A throwaway `pdf-model` example asking three documents what their `/Root` resolves to and how
  many objects are misfiled, deleted once it had answered.
- The measurement driver over every fifth corpus document, twice — once for the pass attribution
  and once for the three object-stream ceilings, with the constant patched and the binary rebuilt
  between runs, because a ceiling is a constant rather than a flag.
- §7.5.7 read whole, including its five NOTEs and both examples, and §7.5.8.3's Table 18.
- The gate results are in the round's report and not here.

## 10. What is left

`doc/todo/57` has the order. After this verb: `split --at-bookmarks`, the aligned rotated
comparison ADR 0831 §1 priced, a per-input password for `merge`, the confinement tranche, and the
RFC 0003 hand-off — which is now unblocked, because the owner's sequencing was that the
file-system faces follow the writing verbs, and there are no writing verbs left. The suite's five
writers still have exactly one gap in common, and session 898 is closing it from the other end: a
corpus-wide *foreign* readback, which `optimize_corpus.rs` now inherits for a fifth time.
