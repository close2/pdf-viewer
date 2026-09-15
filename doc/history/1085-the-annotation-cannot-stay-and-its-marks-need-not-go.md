# 1085 — The annotation cannot stay, and its marks need not go

Date: 2026-09-15. Branch: `batch-1080-1085`, worktree `/home/AI/pdf-viewer-rounds`, five siblings.
ADR: [1099](../adr/1099-the-annotation-cannot-stay-and-its-marks-need-not-go.md).
Question: [Q65](../questions/Q65-may-a-preserve-remedy-write-onto-a-page-the-producer-wrote.md).

## The census, re-run, and the row it named

`tests/archive_corpus.rs` re-ranked after 1073's rewrite. Its top PDF/A-2b row is 1073's own, now
refused for the fault ADR 1087 section 3 argues; the two colour rows above it at PDF/A-4 are
`WRONG_FAMILY`. The largest that is neither is `annotations/subtype-defined-in-iso-32000-1`.
Reading all fourteen of its witnesses is what shaped the answer, and a count could not have: each
is one annotation — `Sound`, `Screen`, `Movie`, `3D`, `RichMedia` — and **ten carry a normal
appearance stream while four carry no `/AP` at all.** Two questions, not one.

## What was built

ISO 19005-2 section 6.3.1 and ISO 19005-4 section 6.3.1 state a prohibition and no alternative, so
removal is the only rewrite that meets either: `Loss::ForbiddenAnnotation`, the limits document's
section 3.2 *Ask*, built as one. The reference leaves the page's `/Annots` and the walk does the
rest — the media stream and §12.5.6.14's popup go because nothing reaches them. The marks need
not.An appearance is a form `XObject` the **producer** wrote, and ISO 19005-2
section 6.2.2's NOTE 2 puts a page description and an annotation appearance under the same
restrictions, so a page may carry it directly. `remedy = "preserve"` appends a page stating the
source page's boxes and turn, invoking the same stream under §12.5.5's own matrix — **no placement
choice at all**, where ADR 1025's metadata page needed ten. An annotation that drew nothing refuses
by name rather than having an appearance constructed for it to keep.

**Flattening back onto the annotation's own page was not taken**: `CLAUDE.md`'s third amendment
permits *appending* and keeps "composes new content over pages" shut. Q65 puts that sentence to the
owner with the two costs a yes carries — §8.4.4's q/Q balance, and annotation drawing order.

## Calibration, and the two witnesses

Trap 13 at both levels: `the_predicate_answers_what_the_two_rows_report` derives its population from
the table and holds the new predicate to each row's verdict, subtype by subtype, and
`an_annotation_of_a_subtype_the_part_admits_is_not_touched` is the same fixture wearing `Square`.
Through `quorra-transform archive --to 2b`, re-opened and re-validated: `6-3-1-t01-fail-c.pdf`
**conforms**, gaining one page whose content invokes the producer's stream byte for byte;
`6-3-3-t01-fail-q.pdf` is **refused by name** under `preserve` and **conforms** under `--authorise
forbidden-annotation`. Trap 1: both rendered and looked at — the appended page shows the invader
where the source page had it, and page one is blank.

## Measured

Authorised run: PDF/A-2b 437 → **447** converted, 173 → **163** refused; PDF/A-4 171 → **176** and
188 → **183**. Both subtype rows leave both rankings; every default run is unchanged at every
target, which is what an *Ask* means and is the difference from 1073's `Mechanical`.
