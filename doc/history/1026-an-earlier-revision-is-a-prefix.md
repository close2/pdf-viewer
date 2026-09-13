# 1026 — An earlier revision is a prefix, and the comparison it makes possible

Ledger slot of batch 1026–1031, on `batch-1026-1031`, five siblings live in the same worktree.
Theme: §12.8.2.2.2's second step, whose obstacle turned out not to be one.

**1. The reading.** §12.8.2.2.2's row has said since session 545 that comparing the signed and
current revisions is blocked because "`Document` holds one view of the file". The clause asks for a
*second* document, not a mutated one, and §7.5.6 hands it over: "changes shall be appended to the
end of the file, leaving its original contents intact", with "[e]ach trailer shall be terminated by
its own end-of-file (%%EOF) marker". An earlier revision is a **prefix**, and `FileBytes::prefix`
opens one — on disk a duplicated descriptor and no byte read, in memory one allocation, because
neither `Arc<Vec<u8>>` nor `Arc<[u8]>` sub-slices. Immutability makes the comparison exact rather
than forbidding it. ADR 1043.

**2. What landed.** `pdf_signature::revision::Comparison::of` builds both states off round 1021's
boundaries (`signed_end`, `excluded`) and diffs the two cross-reference tables. Identity is by
*placement* — same offset, or same index of the same object stream, below the signed boundary —
which the digest warrants: those bytes are "known to correspond to the state of the document at the
time of signing". `/Root` is compared separately: an update can re-point Table 15's entry at an
object the signed revision already held and move nothing else.

**3. What is refused, by name.** `NotComparable` — not a revision boundary, a hole
holding more than the signature value, a table recovered by scanning, a prefix that will not open.
`Changes::unplaceable`, per object. And `Judgement::NotClassified`, which is Table 257's ranking and is **not
done**: it names the level, counts the objects, and says what deciding needs, including the table's
carve-out for a DSS or document-timestamp update — a judgement about a whole revision, which is why
`updates_after` is counted first. The only assertion is `NoChangeToRank`.

**4. The corpus.** Four of ten signatures refuse, including the corpus's one certification
(`xfa_filled_imm1344e.pdf`, 32 578 unsigned bytes in its hole) — the answer that row wanted, not the
one it feared. Six compare clean, and `prefilled_f1040.pdf` is the live case: **three incremental
updates, seven objects added and eight redefined**, every number named and none ranked.

**5. Rows moved.** §12.8.2, §12.8.2.1, §12.8.2.2, §12.8.2.2.2 and §12.8.2.4 rewritten, all still
`partial` on narrower debts; §7.5.6 gains the prefix reading. **The debt §12.8.2.2.2 now carries is
that no host reports any of this** — the capability reached the crate and not the program, named
here rather than left to be rediscovered.

**6. Gates.** Tier 1 and the whole of tier 2 (rule 2). `raster_golden` 974 held, 0 moved. The
workspace lines are red on **siblings' in-flight edits** (`viewer-core`, `viewer-ffi`, `pdf-model`'s
`silent_fonts`) and were run with those excluded; `launch_path`'s `open_kinstructions` band is
missed by a sibling's `viewer-core/src/open.rs`, calibrated by reverting this round's whole diff —
the figure is 4945.254 without it and 4943.133 with it, against the band's 4996 floor.
