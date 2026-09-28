# 1367 — The not-owed bucket, re-read: two rows are excluded and two departed

Session 1265. Status: accepted.
Context: `doc/conformance/ledger.toml` rows §12.6.4.9, §12.6.4.10, §12.5.6.11, §12.5.6.12;
`doc/todo/65`'s not-owed bucket.
Applies: `CLAUDE.md` principle 5's clause 13 entry; `doc/questions/A72` and ADR 1299's bound;
ADR 1119's `departed`.

## 1. What `reported` claimed

`doc/PLAN.md` §5a defines `reported` as deliberately not implemented *yet*, and still owed. Five rows
wore it in `doc/todo/65`'s not-owed bucket, which is a contradiction in the map's own terms: a row
that owes nothing cannot be both. Each was re-read against its clause, the titles either side and
Errata Collection 3 (`spec-errata emit` files only typographic issues under these headings: #524
and #608 on §12.5.6.11, #677 on §12.6.4.6).

## 2. Sound and Movie actions are `out-of-scope`

§12.6.4.9 and §12.6.4.10 both open with "The features described in this subclause are deprecated with
PDF 2.0. They are superseded by the general multimedia framework described in 13.2, "Multimedia"."
Every entry of Table 212 is a parameter of playing a sound. The one sentence of §12.6.4.10 that is not
about playing addresses whoever writes the file. So nothing in either clause is owed by a reader that
excludes clause 13, and the right status is the one §12.6.4.14's rendition action and §12.6.4.16's
Go-To-3D-View already carry: `out-of-scope`, exclusion `clause-13-multimedia`. The runtime sentence
(`action.rs`'s refusal, `viewer_core::interact`'s `Event::Reported`) is unchanged. An exclusion that
still says so out loud is better than one that does not.

## 3. The caret and the stamp legends are `departed`

A72's ruling, as ADR 1299 applies it, says no mark is drawn where a clause names none. The ruling puts
§12.5.6.11's caret and §12.5.6.12's legends on the far side of that line. So a caret without an
appearance stream, and the ¶ Table 183's `/Sy` associates with it, are not drawn by decision. Neither
is a stamp without an appearance stream, whose Table 184 `/Name` is a legend, a word rather than a
shape. That decision is not debt, so `reported` overstates it. `implemented` would hide it, which is
ADR 1119's reason for the word. Each row is now `departed`, and its note's first sentence names this
departure.

**The price**, which is what ADR 1299 did not write down: the curated corpus holds 6 carets and 19
stamps, and every one carries an `/AP` (the rows' own counts, `examples/witness_census`). `/Sy` is
stated as a name by none of the curated documents. So the departure takes nothing off any page this
project measures. It costs a producer that relies on a reader's artwork: a blank rectangle, and the
report sentence `appearance::construct` gives each subtype. The ¶ is the one quantity-free mark
either table names. Drawing it without the caret it accompanies ("displayed along with the caret",
Table 183's `/RD`) is the worse answer, as §12.5.6.11's note argues.

## 4. What did not hold

§12.6.4.6 stays `reported`, and it left the bucket for bucket 6: ADR 1368.
