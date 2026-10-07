# 1622 — The forty departures read against the tree, and a departure is from a `shall`

Session 1393. Status: **accepted**. Amends how ADR 1119's word was applied, not the word; ADR 1623
is the rows whose premise had expired.
Context: `doc/PLAN.md` §5a; `doc/questions/Q63`, `A63`, `A72`; ADRs 1119, 1165, 1166, 1535, 1560,
1572, 1574; ISO 32000-2 §3.15, §10.4.2.1, §10.7.1, §10.7.4, §12.3.5.1, Table 184.

## 1. The rule, decided once

Q63 asked for a word for "one `shall` addressed to this program that the project decided against".
In practice the word also went to rows whose unexecuted sentence was a `should`, a `may`, a NOTE, or
a method the standard offers a kind of processor this one is not — and `implemented` already means
"every normative requirement is executed", which such a row meets. So: **`departed` is kept for a
`shall` addressed to this program that it does not carry out, decided and priced.** A recommendation
declined, a permission not taken, or an alternative the standard ranks below the route taken departs
from no requirement; the row is `implemented`, its note's first sentence naming the choice, quoting
the sentence that leaves it to the processor, and naming the ADR that priced it — ADR 1535's shape
for a permission exercised, applied to one declined. A conflict between two `shall`s (§12.5.5) and a
`shall` met on one reading of an undefined word only (§10.7.5, ADR 1189 section 5) stay `departed`.
`doc/PLAN.md` §5a's `departed` row now says so. A heading over such rows takes their status.

## 2. The rows (`tools/state.sh departures` is each one's reading list)

| row | outcome | the ground, read in `doc/md/` and the tree |
|---|---|---|
| §6.3.2.1 | `implemented`, choice | Annex F is a `should`; ADR 1225's price stands, and nothing reads a document off a network |
| §7.4.2 | stands | the stray byte is still skipped; ADR 0036's recovery names nothing that can expire |
| §7.4.8 | stands | Table 13's default of 1 is a `shall`; `issue11931.pdf` is still the one page (ADR 1183) |
| §7.5.5 | stands | Table 15's `/Size` sentence unchanged; 66 documents still the price |
| §7.10.2 | `partial` (ADR 1623) | ADR 0098's ground, that choosing a spline would invent an algorithm, is answered by A72's bound |
| §8.5.3.3.1 | stands | the clause's own hedge; `Extents::collapse` still declines the point |
| §8.6.5.7 | `implemented`, choice | "should avoid converting"; every raster, the simulated press's included, is a screen's |
| §8.7.4.5.7 | stands | §10.7.2's tolerance is in device pixels; no backend holds a colour space |
| §8.7.4.5.8 | stands | §8.7.4.5.7's |
| §10.4.2.3 | `implemented`, choice | §10.4.2.1 offers the family to "a less-capable PDF processor" |
| §10.4.2.5 | `implemented`, choice | the same sentence; ADR 0263's price |
| §10.7.4 | stands | four `shall`s, and the first sentence now names all four; section 3 |
| §10.7.5 | stands | ADR 1189 section 5; the note's licence-by-NOTE wording corrected to ADR 1560's reading |
| §11.3.4 | `implemented`, choice | the clause names no conversion, and §10.4.2.1's `should` decides the route |
| §11.5.3 | stands | `MAX_PRESSES` (ADR 1254) |
| §11.6.6 | stands | the same bound, ADR 1254 now in the first sentence |
| §11.7.2 | stands | the same bound, ADR 1254 now in the first sentence |
| §11.7.5.3 | stands | a document's own `B2A` is still the conversion in |
| §12.2 | stands | `/HideMenubar` over the reader's levels; GTK 4 still cannot place a window |
| §12.3 | `implemented` | both children are |
| §12.3.5 | `implemented`, choice | the splitter is a `may`; `/Colors` states no `shall` |
| §12.3.5.1 | `implemented`, choice | the same sentence |
| §12.5 | `partial`, aggregate | §12.5.6.2's and §12.5.6.6's rich text (ADR 1623) |
| §12.5.2 | stands | a writer's `shall`; no appearance it may write, `Written::unappeared` |
| §12.5.5 | stands | three `shall`s in conflict, all still the standard's |
| §12.5.6 | `partial`, aggregate | the same two rows |
| §12.5.6.11 | `implemented`, choice | no caret artwork anywhere; A72 forbids composing one |
| §12.5.6.12 | `implemented`, choice | Table 184's icons are a `should`; A72's bound |
| §12.5.6.23 | stands, re-stated | departed on two refusals (ADRs 1363, 1371); the overlay "should be drawn" |
| §12.6.4.6 | stands | principle 3 unamended; the half naming a document is built |
| §12.7 | `partial`, aggregate | rich text formatting (ADR 1623) |
| §12.7.4 | `partial`, aggregate | the same |
| §12.7.4.1 | stands | `MAX_FIELD_ANCESTRY` 256 against a deepest chain of 32 |
| §12.7.4.3 | `partial` (ADR 1623) | the exclusion does not reach it, and XFA 3.3 is held |
| §12.7.5 | `partial`, aggregate | the same |
| §12.7.5.3 | `partial` (ADR 1623) | the same |
| §12.7.8.3.1 | `implemented`, choice | §3.15: a deprecated part "should be ignored by a PDF processor" |
| §12.7.8.3.2 | `partial` (ADR 1623) | `/RV` crosses once its formatting is applied |
| §12.7.8.3.3 | stands | a carried page's `/Annots` do not cross, so a renamed field has no object |
| §13.4 | stands | the clause 13 exclusion is unamended and Q254 is open |

## 3. §10.7.4's departure (1), by ADRs 1560's and 1572's method

The four — the one pixel, (1) a partly covered pixel painted at its area, (2) the area sentence that
falls with it, (3) a reduced image averaged — are each against a `shall`, and §10.7.1's NOTE
licenses none (ADR 1560). **No contradicted verdict the oracle holds rests on any of them**: of the 47 held contradicted pages `tools/state.sh oracle-held` holds 0 as ours, 14 as the
references' and 33 as choices, and the two held pages (1) is drawn on, `issue7891_bc1.pdf` and `issue4436r.pdf`, are each the
references' because the clause's own form in our place is contradicted by the same measure. Where (1)
still shows is the ambiguous pool, which convicts nobody.

## 4. Consequences

`departed` 40 → 20, `implemented` 699 → 705, `partial` 11 → 25, by `grep -c` rather than a write.
