# 1036 — The report that blamed a font program

Slot: corpus. Branch `batch-1032-1037`.
## The instruments, verified rather than inherited
The oracle over the whole corpus, then `pdfref --bin undrawn` over its log. The *ambiguous,
undiagnosed* ranking prints **an empty list**, all 62 contradicted pages are held by a
`CONTRADICTED_*` group, and the sweep is **byte-identical to the thousand-and-thirtieth's** — 839
listed, 839 measured, 18 at or past −1.00, 15 reported, head and complete tail to the thousandth,
cache 100% of 6747 renders. Third round running in which both rankings name nobody, so the slot's
third option was taken: `corpus.rs`'s incomplete list, ten of whose 61 documents are mentioned in
no `.rs` and no document of this tree — six ADR 0433's §9.7.5.2 class, four held by nothing, and
`issue12823.pdf` the one of those four whose report is false of the file.
## The clause
Its `/FT19` (`NUKKLY+NotoColorEmoji`, `/Encoding /Identity-H`) states `/DescendantFonts [ null ]`.
§9.7.6.1's Table 119 makes that "A one-element array specifying the CIDFont dictionary that is the
descendant of this Type 0 font" — the array is there and the CIDFont is not — and §9.7.6.2's
mapping has nothing to index to. §7.3.9's null rule is about a *dictionary entry* and this null is
an array element. **We are right to draw nothing**: `poppler` draws nothing either, `mupdf` logs
*unknown cid font type* twice, `ghostscript` draws a small substitute, inks 1.998 / 2.091 / 2.038 /
3.155, and the `agrees` page loses two emoji.
## The defect, which is in what we said
The refusal read *font /FT19 could not be parsed* — a claim that bytes a `/FontFile` supplied were
read and rejected, when no program was reached at all — and `corpus.rs`'s classification read that
phrase into its *an embedded font program that would not parse* row, whose population was **4 where
its clause's is 3**. Three more raise sites carried the same false prefix (`/CIDToGIDMap` twice,
the `/Encoding` CMap stream). `FontError::MalformedDictionary` names the table instead; three
narrow rows in `whose_defect`; two tests in `silent_fonts.rs` (the sentence, and a witness reading
the one-element array out of the file), calibrated by putting the raise site back — first assertion
red, program row four again. §9.7.6.1's ledger row gains the witness, stays `implemented`.
**And `MAX_INCOMPLETE` was 91 against a counted 61**, thirty documents of slack in the corpus's own
headline ratchet, invisible because the run prints the population and the constant does not. Now 61.
## Gates
`rustfmt --check` on the four files touched 0; `clippy -p pdf-font --all-targets` under
`RUSTFLAGS="-D warnings"` 0; `nextest --workspace` 4595 passed, 36 skipped; `--doc` 0; `fuzz` fmt 0,
clippy 0; `corpus` ok; `raster_golden` **failed naming `issue12823.pdf p1: reports only (a change in
the diagnosis, trap 37)`**, raster and display-list digests unchanged, regenerated, re-run ok (held
974, moved 0); `oracle` ok, seven verdict counts identical, undiagnosed head still empty. `clippy -p
pdf-model` and `cargo test -p conformance` fail **only** in a sibling's untracked
`crates/pdf-model/src/annotation_state.rs`.
