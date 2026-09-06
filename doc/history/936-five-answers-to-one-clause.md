# 936 — Five answers to one clause, and a rule wrong on its own second witness

Argued in [ADR 0912](../adr/0912-five-answers-to-one-clause.md) (the sweep, the four families, the
measurement) and [ADR 0913](../adr/0913-one-place-for-the-rule.md) (the module, and the sixth call
site). [`Q39`](../questions/Q39-a-population-that-does-not-discriminate.md) is the part the
measurement could not settle.

Branched from `round-932`'s tip, merged `main` (rounds 930, 931, 933, 934) before the gates; the one
conflict was `doc/traps/`'s tail, where main extends trap 34 and this round adds trap 35 after it.

Touched: `crates/pdf-model/src/integer_entry.rs` (new), `crates/pdf-model/src/image.rs`,
`crates/pdf-model/src/inline_image.rs`, `crates/pdf-model/src/lib.rs`,
`crates/pdf-model/tests/inline_images.rs`, `crates/pdf-model/examples/integer_entry_census.rs`
(new), `crates/pdf-model/examples/display_list_digest.rs`, `doc/conformance/ledger.toml` (§7.3.3,
§8.9.7), `doc/traps/instruments-and-reports.md` (trap 35), `doc/HANDOVER.md`, `doc/verify.md`,
`doc/todo/03-more-corpora.md` (§49), two ADRs, one `Q` file, this file. **This round moves pixels
by reach**, so `doc/todo/02` §2 ran whole.

## 1. The sweep, and what a hundred and twelve sites say

Session 932's question was one entry's; this round's was the class's. The greppable half —
`as_integer` under `crates/pdf-model/src` and `crates/pdf-syntax/src` — is 133 lines, of which 112
are reads of a document. What came out of reading them is not a list of defects but a **taxonomy**,
and the headline is that ADR 0904's opening sentence was wrong about its own tree: "one answer to
one clause rather than two" was written where there were **five**, three of them nobody's decision
in particular.

Two of the five are right, and their reasons are what make the other three legible: `signature`
refuses a malformed `/DocMDP` `/P` because `CLAUDE.md` principle 3 makes a restriction the reader's
to set, and `pdf_font` reads `/FontWeight` as a number because nothing there has to become an
integer at all. From those the four families follow, each with an argument rather than a habit:
read a magnitude the file corroborates, refuse an identifier wearing a number, refuse a claim
another reading settles, refuse a restriction in the permissive direction. ADR 0912 has each
argument; the module comment carries them where a future round will be standing.

## 2. The world's half, derived from the standard's own typing

`examples/integer_entry_census` is the population instrument, and its key set is **Arlington's**
rather than anybody's list (trap 25): every name typed `integer` or `bitmask` in some table and
`number` in none. The fifteen names typed both ways — `/Width` and `/Height` among them, because
§14.8.5.4's layout attribute shares the name — are counted in a table of their own with witnesses
instead of being judged, because deciding which table a dictionary belongs to needs the link graph
and guessing would be worse than not asking.

It has two halves because **an inline image is not an object**, and a census that walked only the
cross-reference table would have reported zero on the one document this project had. Its second
half reads its own tokens between `BI` and `ID` rather than asking `pdf_model::inline_image` (trap
8), and it was run against the known witness before it was believed.

What it says is in ADR 0912. The shape worth repeating here: the departure exists, it is **rare**,
and it is **concentrated** — 443 reals against 29 066 528 integers, in 212 of 90 128 documents, and
nine key names carry all but four of them.

## 3. The finding: a rule wrong on the second witness its own round measured

Session 932 measured its reach correctly — two documents of 89 256 changed — and read the second
result incompletely. `GHOSTSCRIPT-695872-0.pdf`'s new report was recorded as "ADR 0799's reading
arriving one clause later"; it is in fact this reader accusing the file of a §7.4.8 disagreement
that was our own arithmetic. The file says `/W 737.999999999715 /H 49.999999999` over a JPEG frame
of 738 × 50. Truncation reads the grid one short in **both** axes.

So the rule for a dimension is the **nearest** integer, and ADR 0904's truncating paragraph is
superseded one session after it was written — on evidence it did not have, not on a change of
taste. The fixture that pinned truncation is rewritten with the argument that a hand-made `/W 2.9`
cannot say which rule is right (trap 4); it can only record which was chosen.

**"A count that improves is not a picture" has a sibling**, and this is it: a *list* of what
changed is not a reading of whether what changed is right.

## 4. The sixth call site

ADR 0904 unified five readers of `/Width` in `image.rs` and called that consolidation most of its
value. There is a sixth in `inline_image.rs`, and it is the one that matters most in the world:
both documents that write a Table 87 dimension as a real write it inside a `BI`. With a real `/W`
the decoder had a grid and `unfiltered_length` had none, so §8.9.3's arithmetic could not predict
`EI` and the extent fell back to the forward search — the reading that loses the rest of the
content stream in silence when sample data spells a delimited ` EI `. It was invisible to session
932 because its witness is *filtered*, and a filtered image's end comes from the filter's own
marker.

Both new tests were run against their defect first (trap 13). Planting `as_integer` back in
`unfiltered_length` fails `a_real_grid_still_predicts_where_the_data_ends` saying "its samples stop
at 1 bytes where 6x1 … needs 6" and reporting a stray `EI` operator; changing `round()` back to
`trunc()` fails the fraction test and `integer_entry`'s own unit test on the GHOSTSCRIPT values.

## 5. The reach, and the instrument that could not have seen it

`display_list_digest` over **1 353 documents** — all of `doc/pdf.js` plus every document in the
world the census found a real `/Width` or `/Height` in, judged or not — before and after in one
sitting with one worker on disk. **One line differs and only its report half**: 673 commands and
the same list hash on both arms, one report before and none after. That also settles the 712
unjudged documents without inspecting them: none of their `/Width`s was an image's.

And it is why the digest gained a column. The change moved *only what the program said*, which the
old digest could not represent at all — it would have printed an empty diff for a fix, and a round
would have read that as "nothing moved". Trap 35.

## 6. What this round did not take

**`/Rotate`.** Nine entries in three documents write it as a real and not one of them draws
differently: two write `.00`, which is the entry's default, and the third's `/Pages` parent states
`270` as an integer, so §7.7.3.4's inheritance supplies what the page's own entry could not. So "a
measured population" has two readings — a document that writes the value, or a document whose page
moves — and this is the first entry where they part. `Q39`, with the recommendation to refuse and
to write the rule down in those words, because `/FormType`, `/Order`, `/OPM` and `/Count` are
queued behind it.

**Every other family's refusal stays a refusal**, and three of them are now refusals with an
argument rather than by omission.
