# ADR 0610 — The rule that was paid, and the row that never heard

Status: accepted, 2026-08-25. Session the seven-hundred-and-twenty-fifth, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other as well as against the code —
ADR 0538's method in its eighth round (0551, 0560, 0567, 0579, 0593, 0600, this), with 710's two
rules for reading the ranking and 0593's third. Amends §10.7.5 in the ledger; rewrites one page-list
note in `crates/pdf-model/tests/oracle.rs`; adds one claim block to
`crates/pdf-model/examples/absence_audit.rs`. **No status moves, no pixel moves, and no report is
added or removed.** Extends ADRs 0028, 0208, 0226, 0268, 0285 and 0493.

## 1. The pair, and why the ranking gave it

The search was run on this base rather than read out of any document. Its order is unchanged —
§12.5 heads it, §12.8 second, §12.7 third — and once the clause-level parents are stripped the three
strongest pairs anywhere in the ledger are §12.4.4 ~ §12.4.4.1, §12.8 ~ §12.8.3 and
**§10.7.4 ~ §10.7.5**.

ADR 0600 §1 named two pairs and read one: it took §12.4.4 ~ §12.4.4.1 and left §10.7.4 ~ §10.7.5.
So 0593's third rule — *take the strongest pair the previous round named and did not read* — gives
this one, and the family it lands in is one no round of this method has opened.

The pair scores on quotations each row makes of the *other's* clause: §10.7.4's "[z]ero-width
strokes may be done in an implementation-defined manner that may include fewer pixels than the rule
implies" stands in both, and §10.7.5's NOTE — "[t]his is the thinnest line that can be rendered at
device resolution" — stands in both. That is exactly the shape the rare-sequence filter exists to
surface, and 0579's rule says which pair to prefer among them: **the one where the two rows do not
merely quote the same sentence but disagree about what it leaves standing.** These two disagree
outright.

## 2. What was wrong

**§10.7.5's row said a `shall` was unpaid, and the four-hundred-and-fifty-fifth session had paid it.**

The row narrates the four-hundred-and-thirty-second session's measurement of `tiny-skia`'s hairline
— chosen for every stroke width *up to and including* one device pixel, and laying one pixel down
per step along the line's longer device axis, so a 45° rule one device pixel wide carried 141.42 of
its own 200 where the fill of the same outline carries 177.44 — and then said:

> Not paid, because it is neither a disappearance nor a sub-pixel shape and because
> `Stroke::device_width` makes a `0 w` stroke — which §10.7.4 exempts by name — indistinguishable
> from it at the rasteriser. `doc/todo/11`.

Every one of those clauses is answered by ADR 0285, in the four-hundred-and-fifty-fifth session,
whose table is those two figures:

- `render_cpu::at_or_under_the_quantum` is `<=` rather than `<`, so a rule exactly one device pixel
  wide takes §10.7.4's general substitution — the same path stroked at the substitute width with
  the width it gave up in the paint's alpha, a factor of exactly 1 at the quantum — instead of the
  library's hairline;
- `sub_pixel_coverage.rs::a_turned_sub_pixel_rule_carries_its_area_on_both_backends` carries the
  `1.0` rung, and its own doc comment says it "is the rung that used to fail";
- §10.7.4's ledger row records the whole of it, under "the substitute's own boundary became
  inclusive (ADR 0285)";
- and `doc/todo/11` — the pointer the sentence ends with — heads that item **"The rule that is
  exactly one device pixel wide — closed (ADR 0285)"**.

**The two reasons the row gave for not paying are the two ADR 0285 decided the other way round**,
which is what makes this more than a stale sentence:

| the row's reason | what ADR 0285 read |
|---|---|
| "neither a disappearance nor a sub-pixel shape" | irrelevant to the requirement. §10.7.4's area rule is not about disappearance: "[t]he area covered by painted pixels shall always be at least as large as the area of the original shape. This rule applies both to fill operations and to strokes with non-zero width." A `1 w` stroke is a stroke with non-zero width |
| a `0 w` stroke is indistinguishable from it at the rasteriser, and §10.7.4 exempts that one | that is what makes the exemption *declinable*, not what makes the case unreachable. The exemption is a `may`; §8.4.3.2's "1 device pixel wide" is a `shall`; `Stroke::device_width` resolves both in the shared crate so that no backend decides it alone (trap 2) |

So the row was not merely behind — it was arguing, from the correct facts, to the opposite
conclusion from the one the tree had already reached and written down three places over.

## 3. It is ADR 0101's shape and 710's, at the maximum distance the ledger allows

A correction landing in the row that *states* a mechanism and never reaching the row that *depends*
on it. What is new here is the distance and the number of witnesses walked past: the paying round
wrote its result into `doc/todo/11`'s heading, into `render-cpu`'s own doc comment for the changed
comparison, into a test's doc comment, and into §10.7.4's ledger row — and the row three lines below
§10.7.4 in the same file, which the ranking pairs with it precisely because they quote each other,
was never opened.

**The ranking is what found it, and it could not have been found by a sweep.** `--bin blockers`
reads a stated blocker against the ledger's account of the clause it names; this sentence names no
clause as a blocker, it names an ADR-less "not paid". `--bin owed` asks whether a debt names a thing
the tree lacks; the thing this debt named — the hairline — is a thing the tree has, in the
dependency. The defect is a *conclusion*, and no program in this tree ranks conclusions.

## 4. Two places counted `/SA true` and disagreed, and neither named a command

§10.7.5's row said "49 corpus documents set the parameter true". `oracle.rs`'s
`AMBIGUOUS_STROKE_ADJUSTMENT` said "30 corpus documents state `/SA true`". Neither states which
population it is over, and a corpus that grew between the two cannot make 49 fall to 30 — so they
are not one claim measured twice, they are two questions wearing one sentence.

`CLAUDE.md`'s rule decides the repair: *a fact that can be counted is not written down; what is
written down is the command that counts it.* `absence_audit` is where the command belongs — it
already asks §10.7.2's `/FL` by the same two routes, a typed `/ExtGState` and a `/Resources
/ExtGState` entry — and the block added here asks the **value** rather than the name, because the
clause's rule fires "[w]hen stroke adjustment is enabled" and a `/SA false` states the entry too.
That distinction is the reason a name census cannot settle it: `witness_census` counts the name, and
the row's own crawl figure says so in as many words.

The block was calibrated against a natural witness rather than a plant: `bug1743245.pdf`, the page
this whole group is about, whose object 4 is
`<< /AIS false /CA 1 /SA true /SM 0.02 /SMask /None /Type /ExtGState /ca 1 >>`, is named by the run.

## 5. What the numbers are now

Both sentences are replaced by the command. The row states what it printed, with the population
beside each figure — **50 of the 974 pdf.js documents, 60 of the 1251 curated, 15 207 of the 65 944
`CC-MAIN-2021-31` files the example reads** — and the note in `oracle.rs` states the command and no
figure at all, because what is load-bearing there is that `bug1743245.pdf` is the one page where the
entry decides a pixel.

Neither of the retired numbers matches any of those, over any population this tree can measure. The
run is trustworthy on its own terms rather than on that: the same run reproduces §10.7.2's recorded
`/FL` figure of **88** exactly, which is the control for an instrument whose block sits three lines
above the new one and asks the same two routes.

The row keeps its crawl figure for the *name*, which was already attributed to
`witness_census --crawl SA`, and now has the value's beside it — and the gap between the two, a name
count above a value count, is the row's own sentence "a `/SA false` states it too" measured instead
of asserted.

## 6. What was not done

- **No status moves.** §10.7.5 stays `partial` and its debt is unchanged: the clause's first
  requirement — adjusting a stroke's *coordinates* to produce lines of uniform thickness — is not
  implemented, is licensed by §10.7.1's NOTE, and is reported by nothing because there is no page on
  which this device could do better. ADR 0028's argument is untouched.
- **No pixel moves.** Nothing in this ADR changes a mark; the code change is an example.
- **§10.7.4's row is not rewritten.** It was right about every sentence this round checked, which is
  0593's finding about a clean side of a pair: the pair still chose where to look.
