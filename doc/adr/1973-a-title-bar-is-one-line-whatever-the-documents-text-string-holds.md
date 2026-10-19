# 1973 — A title bar is one line, whatever the document's text string holds

Session 1568, slot 5 of batch ninety, a host round. Status: **accepted** and **built** in
`viewer-host` and `viewer-ui`, reaching `viewer-gtk` and `viewer-qt` through ADR 1972's shared
`turned`. Context: ADR 1429 (one title rule for the three windows), ADR 1553 (the section read
beside page one), ADR 1962 (a refusal's first line in the title). Code:
`viewer_host::documents::{caption, one_line}`. It moves no ledger row.

## 1. What was wrong

§12.3.3's `/Title` is a text string, and nothing in §7.9.2 keeps a line break out of one.
`PDF20_AN001-BPC.pdf` states an item as `PDF 2.0 Application Note 001:  \n Black Point
Compensation`, and the caption each window builds from `Event::PageChanged`'s section put the break
into the title bar, where a toolkit either cuts the title at it or draws it as a box. Three windows
built that caption in three copies of one `format!`, and the confined window a fourth of its own
shape. Separately, `quorra-confined`'s refusal title carried the adapter's whole refusal, which
`wgpu` writes as a chain of causes on several lines; `quorra`'s title has taken the first line since
ADR 1962.

## 2. The decision

`documents::caption(index, label, of, section)` is the caption `quorra`, `quorra-gtk` and `quorra-qt`
show, written once; `documents::one_line` is the one place a document's text string is put on a
title's one line, and the confined window calls it on its label and section. A line break is one of
Unicode's mandatory breaks (UAX #14's BK, CR, LF and NL: LF, VT, FF, CR, NEL, U+2028, U+2029); each,
with the white space either side of it, becomes one space, and a string with no break is returned
unchanged, so every caption that was right is the same bytes.

**What no clause states, and is a choice**: that the words after the break are kept rather than
dropped. A section named on two lines is named by both, and a title that showed only the first would
be naming a different section. The confined window's refusal title takes the first line of the
refusal, as `quorra`'s does; the terminal keeps the whole of it.

**Gates.** `doc/history/1568`.
