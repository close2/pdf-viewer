# 3725 — A redaction removes what describes the region, and names what repeats it elsewhere

Session 2256. Status: **accepted**.
Context: `doc/questions/A834` (a redaction removes, never covers), ADR 3717 (the private data and
thumbnail), ADR 2841 (removal proven by extraction), ADR 2868 (the overlay after the removal);
ISO 32000-2 §12.5.6.23, §12.5.2, §12.7.4, §14.3.2, §14.6, §14.7.2, §14.7.5, §14.9.3–§14.9.5.

## Context

`redact` removed the region's marks from the content stream and wrote the rest of the file as the
producer wrote it. A fixture carrying one string in every place a PDF can hold text
(`crates/pdf-transform/tests/redaction_leaves_no_trace.rs`) found it, after the redaction, in:
the `/ActualText` of the `BDC` property list around the removed glyphs; the `/ActualText`, `/Alt`,
`/E` and `/T` of the structure element marking them and the `/ActualText` of its parent; the page's
own `/Metadata` packet; the `/Contents` and `/RC` of a text annotation over the region; the `/V`
and appearance stream of a field whose widget sat in the region; the `/Contents` of the `/Redact`
annotation itself, carried back by its popup's `/Parent`; and the text a reader extracts, through
those `/ActualText` entries.

## Decision

- §12.5.6.23 removes "the content in the area specified by the redact annotations" and requires the
  processor to "remove all traces of the specified content". A description of that content is a
  trace of it. So: an inline property list of a sequence the removal reached loses `/ActualText`,
  `/Alt` and `/E` (`redact/marked.rs`); the structure element whose `/MCID` names that sequence, or
  whose `OBJR` names a removed annotation, loses those three and `/T`, and every ancestor loses the
  three, since §14.9.4's replacement text stands for the whole subtree. A named property list
  holding them refuses the page by name: restating a shared resource for one sequence is a
  construction this verb does not make.
- Where only part of an element's content was removed the clause does not say whether its
  description survives. The processor's choice is taken on the side that leaks nothing — dropped —
  and the report says "part of the content it covers was removed" rather than "every content item".
- An annotation whose `/Rect` (§12.5.2) meets the region draws in the area, so it is removed with
  the `/Redact` annotations, and with each the popup that names it. Every removed annotation is
  written as the null object, so a popup's `/Parent`, a reply's `/IRT`, a field's `/Kids` or an
  `OBJR` cannot copy it back. A field whose widget was removed loses `/V`, `/RV` and `/DV`.
- A redacted page's and an edited form's `/Metadata` join ADR 3717's stale entries.
- **Not bound**: an outline title, a named destination, an embedded file, an optional content
  group's name, the document's XMP and information dictionary, an annotation outside the region.
  None is content in the area, and editing what a producer wrote elsewhere because it repeats the
  region's words would be this program guessing what else the reader meant to remove. Diligence
  ("shall also be diligent in their consideration of all content that can exist in a PDF
  document") is taken as the duty to look: the text the removal took (the readback of each run of
  removed codes, one run per line) is searched for, as whole words, in every object written
  unchanged, and each place is a `Warning` naming the object and its path from the catalog —
  never the text, since a report quoting it would be one more copy. Redacting it there is the
  reader's choice.
- Each removal and restatement is a `Departure`; no public type changed.

## Consequences

The three tests of `redaction_leaves_no_trace.rs` hold the bound places at zero hits, the unbound
ones carried and warned, and the report. Left: a structure element written directly inside its
parent's `/K` is not restated (only an indirect object has a slot of its own to restate it in);
codes under an `/ActualText` read back only the replacement, so a run is that replacement's text;
a form `XObject` copied per placement is matched to the tree by the original's number.
