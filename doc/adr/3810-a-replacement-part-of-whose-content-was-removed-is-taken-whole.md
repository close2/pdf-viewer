# 3810 — A replacement part of whose content was removed is taken whole

Session 2277. Status: **accepted**. Context: ADR 3725, ADR 3717, `doc/questions/A834`; ISO
32000-2 §12.5.6.23, §14.3.2, §14.9.4.

## Context

ADR 3725 left one reading open: the codes under a marked-content sequence's `/ActualText` read
back only the replacement (each code's span is the whole value), so where a region takes part of
such a sequence, what the removal "took" is not a substring of the readback. §14.9.4:

> The value of ActualText is a character substitution for the content enclosed by the structure
> element or marked-content sequence.

and the value "shall be used as a replacement, not a description, for the content, providing text
that is equivalent to what a person would see when viewing the content".

## Decision

- **The clause maps the whole value to the whole content and nothing finer.** A part of the
  replacement standing for a part of the codes is a correspondence the file does not state, so
  the removal is taken to have reached the whole value: it is written nowhere on the page (the
  property list's three entries go, `redact/marked.rs`, as ADR 3725 has it), and the whole value
  is the run looked for elsewhere (`traces::removed_runs`), so a place outside the region that
  repeats it is named in a warning.
- **What the partial redaction leaves** is the kept codes, read back by §9.10's own route: with
  the replacement gone they no longer claim to be text equivalent to what a person sees, which
  they would not be once part of what is seen is gone. No replacement for the kept part is
  composed: choosing its words would be this program authoring text.
- The codes' own §9.10 readback under a replacement is not looked for elsewhere: by §14.9.4 the
  value, not the codes, is the content's text.
- **A page's own `/Metadata`** (§14.3.2), which the brief listed as unread, is not written for a
  redacted page since ADR 3725 (`STALE_PAGE_ENTRIES`), and `redaction_leaves_no_trace` holds its
  `page-xmp` copy at zero; a page the region did not reach keeps its packet and, where it repeats
  a removed run, is named by the warning (`is_text_data`). Nothing changed for it here.

## Consequences

`redaction_leaves_no_trace::a_replacement_part_of_whose_content_was_removed_is_written_nowhere`
holds the reading: the kept codes read back alone, the value is in neither the page nor its
readback, and the outline that repeats it is named.
