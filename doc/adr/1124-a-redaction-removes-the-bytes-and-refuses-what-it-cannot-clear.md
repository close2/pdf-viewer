# 1124 — A redaction removes the bytes, and refuses what it cannot clear

Status: accepted. Session 1112.
Context: `crates/pdf-transform/src/redact.rs` (the `redact` verb), `crates/pdf-transform/src/lib.rs`
(`Plan::Redact`, `Origin::Redacted`, `Report::departures`, `Departure`, `Plan::operation`),
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-model/src/content.rs`
(`base_transform`, made public), `crates/pdf-transform/tests/redact.rs`.
Builds: RFC 0002 section 10's serializer (`pdf_syntax::serialize`), the owner's ratification in
`doc/questions/A64`, and A65's provenance fence (ADR 1120). `doc/todo/64` is the design this round
executed.
Clauses: ISO 32000-2 §12.5.6.23 (Table 195), §9.4.3, §9.4.4, §9.6.5, §9.7.5, §8.5, §8.7.4.2,
§8.9.7, §11.6.4.3, §7.5.6, §7.5.7, Table 22.

## 1. The removal is a new file, and it deletes the bytes

`doc/questions/A64` put content removal in scope. The exclusion it lifted rested on one false
sentence — "applying a redaction is the one edit an incremental update cannot express" — and
§7.5.6's update is exactly the wrong tool: it appends and leaves the producer's bytes in the file
byte for byte, which is the opposite of "remove all traces of the specified content". So `redact`
writes a **new file** through RFC 0002 section 10's serializer. Every non-redacted page and its
content stream crosses byte for byte (a copied slot); a redacted page is *replaced* — its
`/Contents` a new stream with the region's content spliced out, its `/Redact` annotations dropped
(§12.5.6.23: "the redact annotations are removed") — and, because the assembly is copied by
reachability, the original content stream and the removed annotations are referenced by nothing and
copied by nothing. That is what makes the removal a removal and not an orphan the file still holds;
`text_inside_a_quadpoints_region_is_removed_and_outside_it_is_byte_identical` greps the output for
the secret and for the surviving line to prove both halves.

## 2. No font metrics: the advance is the placed quad's own

The region is Table 195's `/QuadPoints`, else `/Rect`; "within" is bounding-box intersection, the
choice the clause leaves open, recorded as a choice (centre-in-region leaves half-glyphs,
containment leaks). The page is interpreted **without `/Annots`** (`render::page_to_draw(page, None,
false)`), because `text_layer` otherwise carries §12.5.3's annotation pass and the removal reads
only `/Contents`. Each placed code carries a §9.4.4 quadrilateral in the display list's space;
`base_transform` (now public) maps the region into that same space. A deleted glyph's advance is
restored as a §9.4.3 `TJ` adjustment whose magnitude is the quad's own advance vector, projected
onto the text-rendering x-axis the walk tracks and divided into thousandths of text space — so the
walk needs no font's width table. A second walk cross-checks the first: its code count must equal
the interpreter's, and a disagreement refuses the page rather than cut it against a walk the
interpreter does not confirm (trap 13). `one_glyph_is_removed_from_the_middle_of_a_show_operator`
is the witness that the arithmetic is right — B goes, A stays where it was.

## 3. What it refuses, and what it departs from — each priced, never silent

Two things the clause asks for this build does not do, and neither is done quietly (trap 5,
principle 1).

**Refused by name**, its content and annotation left as the file wrote them: a page whose region
holds content the removal cannot prove it clears without trace — a Type 3 font (§9.6.5's glyph
procedures draw outside the advance box this walk measures), a composite font not `Identity-H`
(§9.7.5's codespace decides code width), the `sh` operator (§8.7.4.2), a soft-mask group
(§11.6.4.3), or an image, painted path or form meeting the region (the clause's own "that portion
of the image data shall be destroyed"). Over-refusal is the safe direction: a refused page leaks
nothing. An encrypted document is refused outright, because the serializer emits no `/Encrypt`.

**A reported departure**, not a refusal: Table 195's overlay (`/OverlayText`, `/IC`, `/RO`) is not
composed. Those marks are partly this program's invention (A65), the far side of `CLAUDE.md`'s
authoring line, so the removal happens and a per-page `Report::departures` entry says the overlay
was not drawn — the same shape as `Origin::Optimized`'s savings, given a home that survives into
RFC 0002 section 4.5's JSON. `Origin::Redacted` carries the counts beside it. The ledger row stays `partial`: the overlay is a decided departure but destroying image data in the region is an owed capability the clause requires, so the row is not a pure departure.

## 4. Whether to apply is a policy, asked once

Removal is Table 22 bit 4's residual content modification (`Operation::Modify`), so `Plan::operation`
returns `Some` and the existing seam asks the policy once per document before the verb runs (ADR
1076) — never a refusal hard-coded at the point of removal, which is what lets a host later supply
the four levels `CLAUDE.md` principle 3 names.
