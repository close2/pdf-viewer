# 0542 — The reading that was missing from the run

Status: accepted
Date: 2026-08-23
Session: 694

Returns `function_based_shading_cmyk.pdf` page 2 to `CONTRADICTED_DEVICE_CMYK_CONVERSION`, from
which it was removed in the six-hundred-and-eighty-eighth session on a run that had no
`ghostscript` reading of it. Makes the oracle print, per page and in the summary, when a reference
produced no raster at all — which it had never done on a page still judged by the two that
remained.

## Context

The six-hundred-and-ninety-fourth session's first act was its own baseline oracle run, on a branch
whose tip is the six-hundred-and-ninety-third's merge. It failed:

```text
1 page(s) newly contradicted by the reference consensus: ["function_based_shading_cmyk.pdf page 2"]
```

That page had left the contradicted list six rounds earlier. The note written for it then is
careful and is right about everything it measured: our raster for the page is byte-identical across
that round's change, checked with `cmp` on two renders taken either side of the patch, and the
mechanism the group names — §10.4.2.5's four presses, ours and `poppler` assuming standard process
inks against `mupdf` and `ghostscript` reading Artifex's SWOP profile — was untouched. Its
conclusion was that *the consensus dissolved*, on the evidence that "the closest two references now
miss each other by 29.06 against a page bound of 1.00".

## The measurement

Three facts, each taken from an instrument rather than from a document.

**The references have not moved.** All three cached panels for this page carry an mtime of
2026-07-29 in the shared reference cache at `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`,
which is the cache the six-hundred-and-eighty-eighth read. Re-running today's `pdftoppm`, `mutool`
and `gs` with the gate's own arguments reproduces each of them: ink 80.6357, 77.9291 and 77.7736 of
255, identical to the fourth decimal.

**Our raster has not moved.** The gate prints 5.15 / 19.47 / 29.19% / .9956 for the page, which is
the table ADR 0510 recorded for it in the six-hundred-and-eightieth session, before the round that
removed it.

**So the missing operand is a reference, and the recorded figure names which.** Over the artefacts
the gate writes, at 72 dpi, in `raster_compare`'s own differing fraction:

```text
  poppler   vs mupdf         29.063%
  poppler   vs ghostscript   29.435%
  mupdf     vs ghostscript    0.192%      well inside the page's 1.00% bound
  ours      vs poppler         0.130%
```

**29.063 against a bound of 1.00% is 29.06** — the figure the removal was written on, to the
hundredth, and it is `poppler` against `mupdf`. `mupdf` and `ghostscript` are a consensus by any
bound this gate holds. A run in which no pair of references agreed on this page is therefore a run
in which `ghostscript` produced nothing, leaving the only pair at 29.06.

## The instrument defect, which is the general half

`render_references` renders the three available references in parallel and then:

> A reference that fails on a document is not evidence of anything — many of these files are
> deliberately damaged, and a renderer refusing one is the correct behaviour — so its absence is
> tolerated as long as two remain.

That reasoning is right and the code implemented half of it. Fewer than two remaining is reported
with every failure's own message; **two remaining discarded the failures on the floor**, so a page
judged on two readings printed a line indistinguishable from the same page judged on three. A
verdict computed from a pair is a different measurement from one computed from a trio — the third
reading is exactly what turns two renderers that miss each other into a consensus — and the gate
had no way to say which had happened.

The consequence is the worst shape a false result can take, and it is the same shape
`doc/todo/02` §2 records for a loaded machine: a page moves, every input is unchanged, and the
output is identical to a page that did not move. Six rounds of this tree carried a diagnosis
written about a run rather than about a page, and the note that carried it was written carefully by
a round that checked the two things it could think to check.

Why `ghostscript` produced nothing on that page in that run is not recoverable now and does not
need to be: `Reference::render_within` kills a renderer at 30 seconds, the machine runs four rounds
in parallel, and `doc/todo/02` §2 already records an oracle run whose wall clock quadrupled under
load. What is recoverable is that it did.

## Decision

1. **The page goes back**, into `CONTRADICTED_DEVICE_CMYK_CONVERSION`, with the measurement above
   in the group's note. The section the six-hundred-and-eighty-eighth wrote stays where it is, with
   a sentence in front of it saying which of its claims this supersedes — its evidence is good and
   only its cause was wrong, and deleting it would lose the record of how a note comes to be about
   a run.
2. **`render_references` returns its failures on the success path**, `Examined` carries them, and
   `report` prints them twice: on the page's own line as `[judged without: <reference> did not
   render: <reason>]`, and as a count in the summary block beside the abstention line that ADR 0513
   added for the other way a reading can be missing.

The two lines are deliberately beside each other and say different things. An *abstention* is a
reference that drew a flat sheet and was refused a vote (`pdfref::consensus_abstentions`); an
*absence* is a reference that drew nothing at all. Both shrink the population a verdict is computed
over, and until now only the first one said so.

## Consequences

The gate's ratchets are unchanged in shape: the contradicted list is 64 complete names again rather
than 63, and no other verdict moves. What changes is that the next time a page's verdict rests on
two references instead of three, the run says so on the page's own line — so a round diagnosing it
reads the reason rather than inferring a mechanism.

**A note's citation of a gate figure is a citation of a run.** `doc/todo/02` §4's twentieth sweep
(`--bin quoted`, ADR 0495) already asks whether a figure a note quotes is one the gate still prints;
it would have caught this one, because 29.06 stopped being printed for the page the moment
`ghostscript` came back. The habit that follows is narrower than the sweep and belongs beside it: a
round that removes a page from a ratchet because *the references moved* owes the measurement that
they did — which is one `ls` on the reference cache and one re-render, and is what this round did.
