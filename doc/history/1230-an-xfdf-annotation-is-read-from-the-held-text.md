# 1230 — An XFDF annotation is read from the held text

## What moved

§12.7.8.3.4 `departed` → `implemented` (A97, ADR 1297). §12.7.6.2's row is round 1227's and is
untouched; its note's XFDF sentence was handed over.

## Findings

**The text was extracted and checked before anything was built on it.** `doc/md/XFDF_Spec_3.0.md`
(145 pages, the structure tree's order) carries chapter 2's element reference, `annots`, every
annotation element, the attribute groups and both mapping tables, pages 37 to 99, in order.

**The FDF import had never placed an annotation with a popup.** Table 172's `/Popup` and Table
186's `/Parent` name each other; `forms_data::carry` followed the pair round until its depth bound
refused the whole annotation. Both now cross as positions and are written as references, which is
also what XFDF's nested `<popup>` needed — one mechanism for both formats.

**Table 172 had already settled `/IRT` for FDF**: a text string of the replied-to `/NM`. XFDF's
`inreplyto` is the same string, so the import resolves one form on the same page for both.

**Streams other than `/AP` were written as direct objects** for a carried FileAttachment or Sound;
§7.3.8.1 allows no such thing. Every stream in a placed annotation is promoted now.

**Ten places ISO 32000-2 overrides the text**, tabled in ADR 1297 section 4: `PolyLine`, the
dimension intents, `/State` as a text string, `rotation`, a caret's `/DA`, `/BS` on text, `/BE` on
a polyline, `redact`'s missing `page`/`rect`, `alaw`, and `<link>` (Table 246).

## Left

`<appearance>`, `<overlayappearance>`, `<resource>` and `<ex_data>` are refused by name: the text
does not state the decoded format of the first two, and the last two are deprecated or clause 13.
A link's destinations and actions are named, not built, since Table 246 keeps a Link out of the
array.

## Gates

Tier 1 on `pdf-model`; `cargo test -p conformance` fails only on neighbours' files (`linearize.rs`
quotations, a deleted `render-raster` test); oracle filtered to the new PDF agrees; the
accessibility census's twelve whole-population floors moved for its 145 tagged pages (trap 43).
