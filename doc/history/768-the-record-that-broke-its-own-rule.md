# 768 — The record that broke its own rule

Eighteenth merge round of the block. Four branches, **no conflicts**, and a batch whose sharpest
finding is a document that violated the rule it was written to record.

## The sequence, whole, on a quiet machine (load 1.73)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2687 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**.

## 767 — a `shall` met by two backends out of three, in a sentence that said "both"

§8.9.6.2: interpolation during stencil masking "shall be to smooth the edges of the mask, not to
interpolate the painted colour values". Two rows answered it correctly by naming the raster — a
stencil's cleared samples are `[0,0,0,0]`, so premultiplied filtering gives the painted colour at
partial coverage exactly — and then wrote **"both backends"**. There are three.

```
   cpu: worst departure from the painted colour   0
 vello: worst departure                           0
quorra: worst departure                         131 of 255
```

**quorra is what the shipped viewer draws with**, and `Image::is_smoothed` turns the filter on for
every *reduced* image as well as every `/Interpolate`, so the population is any image with a
transparent sample — `/SMask`, explicit mask, colour key, JPX opacity. **Nothing reported it.**
Nothing here can fix it either (premultiplying our upload would have the shader multiply by alpha
twice), so it is an ask: `doc/QUORRA_FEEDBACK.md` §39 with both fix options, `doc/todo/55`, and the
two correct backends now gated.

Beside it, **a refusal restated one condition too wide**: three rows named "a stencil under a
graphics-state soft mask" as owed, where the refusal is a stencil painted through a *pattern* under
one — an ordinary stencil is drawn through both, which §11.6.4.3 requires.

**And it answered the question it was actually set.** Two instruments would have predicted these, and
both are programmable because both sides are this project's own sentences: **a cardinal that counts
this tree's own parts** (`--bin counts` only reads a cardinal governing a *row's* words, so "both
backends" governs a population no instrument reads — and those two words still stand in a
`pdf-render` doc comment), and **an understating parent**, the mirror of `--bin overstated`: a parent
restating a child's refusal and dropping the condition the child stated it under.

One hazard worth keeping: **reusing a deleted todo number silently repoints every ADR that cited it**
— six dead pointers across four ADRs, visible only because `--bin pointers` counts *live* pointers
too, so the live count went **up**.

## 765 — the record that broke its own rule

**The live head had not stood for four uses. It left the ranking, and the instrument's own record is
what removed it.** 760's history file, in the sentence recording that two issue numbers should have
been written **bare**, writes both **with the `Issue #` prefix** — and step 2's grep cannot tell a
*mention* from a *use*, so both counted as named. Measured: restoring them reproduces 760's figures
exactly (120 unread, the live head at seven); without them the live head is six.

**Eighth blindness, and the second belonging to the instrument rather than to an erratum.** The
repair is a writing rule, because no grep can see the difference: *a sentence about the form of an
issue number must not contain one.* It left 760's file alone and wrote neither number itself, so both
are back in the population where they belong.

Its substantive find is a **closure nobody was checking**. Issue #216 turns "PDF files support a
standard set of filters" into "**processors shall**", so Table 6 is a closed set we owe — and it was
met with no closure check at all: every filter has a test of its *output*, none asks whether the table
is covered, **so a name falling off a match arm becomes indistinguishable from a name that was never
in any table, with the crate green**. Now walked in both spellings, calibrated against two plants,
each of which fails this test *alone*. Beside it, the row's own note counted Table 6 as **nine** where
it has ten.

## 764 — the mechanism that explained a different measure

761 left the gap between a *priced* page and a *diagnosed* one. All thirteen marked rows name their
measure now, and one mechanism turned out to explain the wrong one: a note argued §10.7.5's
single-pixel rule as two camps in **whole-page mean grey**, while the number putting that page on the
list is a **structural similarity against a renderer in our own camp on the mean**.

**The ablation is the round's craft.** Renaming the entry in place — eight bytes for eight, with a
control showing fresh renders byte-identical to the gate's panels — **moves not one reference by a
single bit**. All four render the modified file identically and only our raster moves, by 18.37 of
255, so **that entry decides a pixel for this tree alone** and the other two camps are unconditional
behaviour rather than the clause. Our number falls **31.43 → 2.62** against a divisor that does not
move, and the page leaves the list. What the 31.43 *is* turns out to be §10.7.5's **other** half — a
coordinate adjustment `poppler` implements and we do not, which that same note records as a departure
and **had never numbered**. It is numbered now.

Second result: on **8 of the 26 rows the two halves are different measures**, and naming them makes
those rows **sharper rather than softer** — one reads 14.9× like-for-like where the row printed 5.68×,
and on one page our own mean is *inside* its bound, so only the similarity puts it on the list at all.

## 766 — the mirror of 762's rule, and a lazy table whose laziness was misplaced

The sixth consecutive general round on the same shape, applied to the companion measurement: its
*total* is in five sessions plus a table four rounds ago, its **composition is session 58's**. And it
settled which half had gone stale rather than arguing it — **its baseline reproduces the recent figure
to nine digits**.

**The ranking has inverted.** Decompression was the largest item at 28.0% and is **3.11%** because
three ADRs happened to it; the glyph list was the smallest at 3.2% and has **doubled to 6.52%**
because nothing did.

Two costs, neither named in any document. **A font operator resolved and copied the font dictionary
twice before asking the cache** — the cache lives inside the loader, so everything above it ran first,
and that page states the operator 280 times for 7 fonts: **273 double copies made and dropped**
(−2.41%). And **a lazy table whose laziness was the table's rather than the entry's** — its own comment
argues that 256 searches must not be paid at load, and then the first character resolved all 256 where
the page uses a few dozen: **67 200 calls → 8 850** (−5.10%).

**−7.52% in total, display list identical at 150 350 commands.** And the judgement worth naming: it
**measured the eager arm rather than assuming**, found the lazy cell costs **+0.22% on this page**, and
kept it — because that page is precisely the arm where it loses, while a font with complete Unicode
mapping would otherwise allocate and zero 8 KiB at load for nothing.

## A test that asserts on a clock, found twice

**Two rounds independently hit `a_launch_waits_for_page_one_instead_of_polling_for_it`** failing under
load and passing alone — 765 at load 17, 766 at 35–51 and passing at 5. `Drawing::settle` takes a time
budget, so the assertion is on a wall clock **despite its doc comment saying it is not**. That is
"a duration on a shared machine measures the machine" living in a *test* rather than in a gate, and it
is recorded rather than changed so a future round does not diagnose it as a regression.

## Owed

- **`doc/todo/55`** — the filter that mixes in black, an ask to quorra with both fix options.
- **Two sweeps 767 argued for**: a cardinal counting this tree's own parts, and an understating parent.
- **`Interpreter::font` is still 21.4%** of interpreting that page, almost all of it seven loads an
  interpreter living for one page pays again on the next — a question about `Document`'s immutability
  and about memory, named and not taken.
- **Orca on all three binaries, by a person.**
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
