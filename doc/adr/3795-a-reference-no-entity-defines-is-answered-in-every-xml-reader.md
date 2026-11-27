# 3795 — A reference no entity defines is answered in every XML reader

Session 2273. Status: **accepted** and built. Context: ADR 3621 (rich text repairs returned),
ADR 3752 (`model/xmp-reference-undefined`); ISO 32000-2 §12.7.4.3, §12.7.3, §12.5.6.2 Table 172;
XML 1.0 sections 2.7 and 4.1 (Entity Declared); ISO 19444-1 section 5.4.1.

## Context

`xmp::unescape_noting` keeps a reference XML does not define as the characters it is written in
and answers that it did. The XMP reader logged that answer (round 2262); the rich text, XFDF and
XFA readers went through `xmp::unescape`, which dropped it.

## Decision

1. **Rich text** (`rich_text::markup::parse_noting`) returns `model/rich-text-malformed` with the
   reading when a reference was kept, and the popup's `/RC` walk logs the same class where its
   markup did not parse whole or kept a reference — the class's description now says both.
2. **XFDF** is a file a person imports, not the document, so the reader owes it on `owed` with the
   other things it does not apply, once per file.
3. **XFA** packets carry the answer out of `xfa::xml::parse` (`Tree::kept_reference`) and the
   packet's reader logs the new class `model/xfa-reference-undefined` at the packet's stream, or at
   the file for packets read as one XDP text. It maps to section 5.1 as the XMP class does.
4. `xmp::unescape` is removed; the XMP reader's two extension-schema walks discard the answer by
   name, the packet's own reading having logged it.
5. **CDATA is characters** (XML 1.0 section 2.7): the rich text walk and the popup's walk no longer
   expand a reference inside a CDATA section, as XFDF and XFA already did not.
