# 3765 — The ninth pass reads twenty-four held files and logs seven readings

Session 2266, the silent-repair campaign's census burn-down, ninth pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3565, ADR 3689, ADR 3729, ADR 3757; ISO 32000-2 §7.3.8.1,
§9.6.5.4, §9.7.5.1, §9.7.5.3, §10.7.5, §11.6.5.1, §12.3.2.3, §12.3.3, §12.4.2, §12.5.4,
§12.5.6.22, §12.7.

## Context

ADR 3757 left 122 sites held. Twenty-four files held two each; each site was read against the text
its module cites. The launch gate had two `peak_anon_mib` figures below their floors on main since
`3b5320ea`.

## Decision

- **New classes**, each section 5.1 alone and outside content, so the whole-file rewrite carries the
  entry as stated and the output's own check refuses by the class's name (as ADR 3757's):
  - `model/outline-entry-unusable` (§12.3.3): Table 151's `/F` that is no integer (read as its
    default 0), and a `/C` that is not three numbers from 0.0 to 1.0 — black where it is no array,
    a component of another type 0, a number past the range its nearer end. Logged at the item.
  - `model/page-label-entry-unusable` (§12.4.2): Table 161's `/St` that is no integer of at least 1,
    read as its default 1. The reading was already the clause's; the log is new.
  - `model/soft-mask-backdrop-unusable` (§11.6.5.1): Table 142's `/BC` that is not n numbers for
    its group's `/CS`, read as absent. `table_142_faults` already said it (ADR 3391); now it logs.
- **On existing classes**: a non-integer field `/Ff` (`field-entry-unusable`), Table 194's `/H` or
  `/V` and Table 169's `/I` of another type or past the range (`annotation-entry-unusable`), and a
  `CMap` file's own `/WMode` other than 0 or 1 (`font/wmode-unusable`). The last was read as 0 when
  it did not fit a byte and kept otherwise, so a 2 refused the font even beside a dictionary
  stating 1; it is now read as the nearer mode, as the loader already read the dictionary's, and
  the class's text names both places.
- **A defect**: an unreadable selector inside a `cidrange`, `bfrange` or a range's array was
  dropped without marking the map damaged, so `font/cmap-damaged` missed it; both arms now mark it.
- **Not repairs**, each said beside the code: §12.3.2.3's first page for a structure destination
  whose element states no content (the default is the clause's own sentence, not a guess);
  §9.6.5.4's "(if one is present)" for a `post` table; Table 165's optional `/Next`; the catalog
  declines (`CATALOG_UNREADABLE` where the page tree reads it); a `/Length` that is no non-negative
  integer (`syntax/stream-extent-from-endstream`, logged below it); and the declines whose
  refusal is named where the decode or the draw meets it.

## The launch floors

The two rows fell when `8f54e0483` (ADR 3729) fitted every `/SA` stroke, not only those under 1.5
device pixels. Both page ones draw the PDF Association's coloured boxes with `/SA true` strokes
about 1.6 pixels wide: on the processor those are now exactly two pixels (a row across the red
box's edge reads three part-covered pixels before and two whole ones after), every box present.
On the device a fitted stroke is expanded at its placement and drawn at once rather than queued as
a parallel `Job::sheet` with a shared expansion slot, which is the allocation that went as the
encoder reads; it was not taken apart by function. The launch logs either side of the commit
bracket it: both rows fall about three mebibytes from round 2255's run to 2259's, and the other
four stay inside their bands. That is a saving the clause produced, so
the floors are re-taken by the file's allocation rule over the runs since.

## Consequences

- `HELD` falls from 122 to 74 sites; the twenty-four files hold none.
- A negative `/DA` `Tf` size is still auto-sized without a class, and is not the file's departure:
  Table 103's `size` "shall be a number representing a scale factor" and says no more about its
  sign, so the auto-sizing is this reader's choice and the question is the layouts', not a log's.
