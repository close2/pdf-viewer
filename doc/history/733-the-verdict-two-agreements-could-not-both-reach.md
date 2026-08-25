# 733 — The verdict two agreements could not both reach

Eleventh merge round of the block. Four branches, **no conflicts**, and the first batch in a long
while where a headline verdict count moved **and the reasoning for moving it was taken from the
project's own founding ADR** rather than from a preference.

## The sequence, whole, on a quiet machine (load 1.01)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2620 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · `render-quorra` **933 agree, 22 differ, 2 refused** · both censuses ·
`fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445 implemented, 223 partial, 0
unreviewed**, and the ledger command leaves the tree clean — 728's normalisation held.

| the oracle | before | after |
|---|---|---|
| agrees | 983 | **983** |
| contradicted | 65 | **61** |
| ambiguous | 832 | **836** |

`our geometry` 3, `reference geometry` 2, `not comparable` 42, `no render` 18 — unchanged.

## 729 — the clause was ADR 0005's, and its silence was the defect

727 found that a page can carry two maximal agreeing sets and that the instrument reported one, the
survivor decided by the order the `Reference` variants are declared in. It handed the decision on
deliberately. **The answer came from ADR 0005's own second rule**: a divided page satisfies *both* of
its conditions at once — "two or more references agree with each other" **and** "the references
disagree among themselves" — and it never said which wins, because nobody had noticed a page could do
both. **That silence is where the enumeration order got in.**

Its justification settles it on one article: *two unrelated implementations reaching **the** answer*.
Where they reach two, each backed by a coincidence of identical improbability and neither set
contained in the other, mutual agreement — the only ranking the design has — **ranks neither**. So a
verdict is one every maximal consensus reaches, and where they differ the page is `ambiguous`.

**The control is what keeps this off "the rule that flatters us"**, and it is ADR 0497's sixth
criterion pointed at the room instead of at us: put each *reference* where our render stands, and ask
what the sets it is not a member of conclude about it. On all four divided pages the deciding set
**contradicts a voting reference that is itself in a consensus** — and on one, our raster *is*
`ghostscript`'s to the byte, so the worst tile accusing us is literally `ghostscript`'s distance from
`poppler`. That disqualifies *hold us to every set*: it condemns the implementations whose agreement
is the evidence. *Take the tightest* is disqualified twice — undefined over four measures that do not
rank together, and it ranks readings by the very quantity trap 9 says shared code and a shared ICC
file manufacture, the tighter pair here being the two reading the same profile.

Three things it did that a weaker round would not. **It printed the number that says the rule is
one-way today rather than by construction**: of the 41 divided pages, **36 carry sets that concur in
agreeing with us**, each an agreement a moved pixel could cost by dividing them — and a page carrying
two sets that *both* reject us stays contradicted, with a fixture holding that. **It said which of the
four is not flattered**: one divides by *width* rather than by camp, we sit further from each
reference than any two are from each other, and what removed it is the rival pair's own spread
doubling — its diagnosis stands where it was measured. And it **chained the four into the diagnosed
list on arrival**, because a verdict rule that emptied a watched list into an unwatched one would be
`doc/todo/00`'s own failure mode.

## 732 — a codec whose bounds all come from the reading side

The boundary 724 settled now has its transport. **The format's shape is one decision repeated: the
tables precede the body**, so every identifier the *unconfined host* reads is bounds-checked against
a length **its own decoder established by reading a table**, never against one the confined side
asserted. Counts are checked per table at the smallest record that table admits — tighter than the
generic one-byte-an-element assumption — nesting is bounded by what every backend already refuses
past rather than by an invented number, and an image whose dimensions its samples do not fill is
refused by name.

**Two things a smaller design would have got wrong.** The *whole* clip table crosses, because a
`ClipId` is an index and dropping an unreferenced entry renumbers everything after it. And the
decoder checks that rebuilding the table reproduces the message's own numbering — because `add_clip`
**deduplicates**, so one region stated twice would silently renumber every identifier after it, and
every command naming one would clip by the wrong region, **in the privileged process**.

**The fuzz target found something at 750 executions and it was the target, not the codec**: `f32`
equality is not reflexive, so a decoded list holding NaN is unequal to itself. The judgement is the
part to keep — the decoder deliberately does *not* refuse a non-finite number, because the confined
path must draw the page the in-process path draws, so a value the interpreter can produce may not be
refused at the transport. The assertion became a **byte comparison**, which is total where equality is
not and asserts the stronger property. Clean afterwards at **4 175 795 runs in 901 s**.

724's price held to about a tenth when re-derived with the real encoder (median 0.0368 against 0.034,
worst 1101.09× against 1101×); the aggregate is 0.42 against 0.37 **because a real format has tables,
tags and length prefixes**, said rather than glossed. And the codec is **deliberately not wired into
the frame reply**, for three reasons rather than one.

## 731 — the tree, argued on the standard's axis rather than the toolkits'

**All three windows drive AccessKit directly; neither native host publishes through its toolkit's
accessibility layer.** The argument is the standard's: a screen reader on Linux talks to AT-SPI
whichever route is taken, so what is being chosen is *how many times this project maps §14.7 onto
somebody else's vocabulary*. §14.7.3's role map is a `shall` on this reader and §14.8.4's forty-one
types are mapped **once**; a toolkit route inserts a third vocabulary, does it **differently in GTK
and Qt**, and puts a second tree builder outside the census that ratchets the first.

**It disproved a claim in this tree by writing the code it said was impossible.** `viewer-gtk` held
that `#![forbid(unsafe_code)]` "is what makes subclassing the wrong answer here"; the subclass macro's
`unsafe` is proc-macro expansion, the lint does not fire, and it compiles with the attribute
untouched. Trap 17. The Qt route is closed for a real and different reason.

Five defects, and two travel: **a click on a §12.7 widget does nothing in a host that delegates it** —
`viewer-ui` toggles three of nine, both native hosts toggled **none** while the accessibility action
answered `true`, so an assistive client was told the click worked. Refused by name now, nine of nine.
And **`--trace` and `--trace=all` asked for four of five topics**, the mask being a literal `0b1111`
while the topic list had grown, so 726's new topic printed nothing unless named; derived from the list
now, calibrated against the defect first.

**A real platform difference recorded rather than smoothed**: only Qt can place a node on the screen —
GTK4 exposes a toplevel's position in none of four APIs — so GTK reports position within its own
window and says so on its own trace topic. And **the price is written down**: two applications on the
accessibility desktop per native process, because the adapter embeds a root beside the toolkit's.

## 730 — the selection rule ran out, and a record wrong when it was written

**All three of the strongest pairs are spent**, so "read what the previous round left" had nothing
left and the rule ran out above rank 4. Rank 4 was a **tie at 31**, broken by ADR 0579's rule: one
pair agrees everywhere it duplicates, the other disagrees outright — one row citing another's opening
sentence as current while that row's own later paragraph had closed it. **The method needs a successor
rule rather than another application**, and that is the round's most durable output.

Four findings, none printable by any sweep. A row that is *a disposition of a table* **omitted one of
its entries** — the row calls itself "a list to check rather than to read", an earlier round's "four
entries" was **five**, and the omitted one carries two `shall`s deciding whether the image is drawn at
all, executed in a file the row's `code` array did not name. The same list gave two entries to the
wrong table, where the clause's own first sentence puts them elsewhere. A transform was named as one
the tree does not select, **in the row that also says it reads it** — 710's shape, with the file that
owns the choice right throughout.

And the errata record is **725's shape**: an erratum recorded as carrying two carets carries **four**,
and the two missing ones repair exactly the unevenness `doc/errata-read.md` concluded it left
standing. Wrong when written, from right facts, because `emit` files by page.

## Owed

- **The consensus rule is one-way today and not by construction** — 36 divided pages carry concurring
  sets, and the gate prints the number so a later round cannot mistake the property for a guarantee.
- **The codec into `Answer::Frame`**: three reasons in ADR 0626 §6, held by `doc/todo/15`.
- **The other half of the delegated click** — no message needed, `doc/todo/31`'s next item.
- **A successor selection rule** for the `partial` rows, the pairwise ranking having run out.
- **Orca on all three binaries, by a person**: it is not installed here, and how a screen reader
  behaves with two applications of the same name on the desktop is the one consequence of 731's
  decision that only a real client can judge.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
