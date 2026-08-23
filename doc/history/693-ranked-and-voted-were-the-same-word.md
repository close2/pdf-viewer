# 693 — "Ranked" and "voted" were the same word

Third merge round of the block, and the largest merge this project has taken: **eight branches**,
one conflict. The batch closes two of `doc/todo`'s long-standing items, adds 151 pages to the
gated oracle, and completes §12.8's second question for every algorithm family the standard names.

## The one conflict, and why it is worth a paragraph

688 gave `pdf_render::Shading` a `background` field; 690 refactored
`crates/render-cpu/tests/clip_intersection.rs`'s mark into a `Mark` enum with a shared `gradient()`.
Both are right and they land on the same twenty lines. Resolved by keeping 690's structure and
giving `gradient()` 688's field, **with the reason it is `None` written beside it** — those scenes
state the clip's arithmetic, and §8.7.4.3's wash would put a second colour under every assertion in
the file. A merge resolution that only compiles is a merge resolution that has thrown away one
round's reasoning.

## The sequence, whole, on a quiet machine (load 6.84)

`fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 ·
`nextest` **2491 passed, 18 skipped** · doctests, 0 failures · conformance 182 + 5 + 1 ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · `render-quorra` **957 pages at
glyph quantum 1/16 — 932 agree, 23 differ, 2 refused** · `fixed_documents` 40/0 · text, both
censuses, dates, XMP, JPEG 2000. §5's binaries rebuilt and installed.

**The oracle, enlarged and reproducing every round's prediction:**

| | before | after |
|---|---|---|
| pages | 1794 | **1945** |
| agrees | 902 | **983** |
| contradicted | 60 | **65** |
| ambiguous | 768 | **832** |
| our geometry | 2 | **3** |
| not comparable | 42 | 42 |

Every delta composes and every one was predicted by the round that caused it: 692's 151 new pages
from `pdf20examples` and `pdfbox`, 688's one contradicted page leaving, 690's 25 moved per-page
lines with no verdict change, 685's one document entering `incomplete` and 688's two leaving.

Ledger **implemented 443 → 444, partial 224 → 223** (688's §8.7.4.3). Negatives queue **26 → 30
done, 20 → 16 owed**. `tools/state.sh hosts`: `Query` **20 of 31**.

## What the batch was actually about

The last block's lesson was that an instrument which cannot see the population it claims to measure
reports success in the voice of a program that checked. **This batch found the same failure in the
project's own vocabulary**, twice, and in both cases the word was doing the concealing.

**692 is the clean case.** `doc/todo/03` says in four places that each corpus was "put in front of a
reference", and concludes that "every population on this disk is ranked". Every word of that is
true, and all of it is about the **ink ranking** — page one at 72 dpi, ordered by our ink minus the
lightest live reference's, by a script that lives for one round, reaching no verdict, holding no page
by name, unable to fail a build. **None of the 275 documents had ever been through the oracle.**
*Ranked* and *voted* are different instruments recorded in the same words, and `doc/habits.md` now
says so.

**688 is the same shape one level down.** `issue13372.pdf` has been the headline witness for
§8.7.4.3's `/Background` in ADR 0452, in `doc/todo/17` and in the ledger row. Its `/Coords` are the
exact **diagonal of its own stencil rectangle**, so every point of the area to be painted has
`t ∈ [0,1]` and the wash has **zero area**. The claim that "the page's corners project outside
[0,1]" had measured the *page* where the clause says *the area to be painted*. A witness named in
three places for three rounds, witnessing nothing.

## The two items closed, and the prices that were not there

**§8.7.4.3's `/Background` is painted** on all three backends and all four shading kinds, `partial`
→ `implemented`. Its clause finding is better than the feature: **§11.6.7 states the construction
outright** and nobody had cited it — *"[i]f the shading dictionary has a Background entry, the
pattern's implicit transparency group shall be filled with the specified background colour before
the sh operator is invoked"*, the group being a **knockout** one whose colour and shape become the
object's. So the wash and the shading are one painting operation with one coverage, one `ca` and one
blend. `doc/todo/17` had reached that shape by an antialiasing argument; the standard says it.

**`doc/todo/11` item 4's stroke bullet is closed.** The item priced it at "duplicating the library's
stroker or contradicting its hairline" and **the first half does not exist**: `tiny_skia::Path::stroke`
and `Path::dash` are public and are the same `PathStroker` and dasher `stroke_path` calls; its
non-hairline branch is exactly those two followed by a non-zero `fill_path`. The hairline half was
answered **by moving the boundary rather than crossing it** — `treat_as_hairline` compares an
approximate length along the transform's two basis vectors where `pdf_render::thinnest_line` is the
larger singular value, and they part by up to **√2 under a shear**. That is a device decision the
*library* made for one backend, and it is `pdf-render`'s now: **trap 2's sixth instance.**

**Three prices died this batch, all of the same cause.** 688 found three of `doc/todo/17`'s four
wrong, including "quorra's gradient lane, the largest single item, not this tree's to change" —
**not owed at all**, since all three backends already have the lane, and nothing went into
`doc/QUORRA_FEEDBACK.md` because there was nothing to ask for. 690 found the stroker price wrong for
the same reason. `doc/habits.md`'s rule keeps being right: *the cheapest re-derivation of a price is
asking what the libraries and the layers already contain.*

## §12.8's second question is answered for every family the standard names

689 built Table 260's ECDSA over P-256/384/521 and the Ed25519 that ISO/TS 32002 §5.1.2 adds, on the
owner's standing decision to use reviewed crypto rather than in-tree arithmetic — 23 new locked
packages, `default-features = false`, `precomputed-tables` off with the reason written down, no new
`cargo deny` exception, and both `x509` and `cms` fuzzed clean at a million cases each. Binary cost
**+1.10%**, A/B in one sitting.

**Its correction is the durable part: BSI TR-03111 was never a principle-5 blocker.** `doc/todo/51`
had recorded it as one through two rounds and two ADRs. ISO/TS 32002 §5.1.3's NOTE 2 says a
conforming ECDSA signature value **is** the DER `ECDSA-Sig-Value` — so the files using the other
encoding are outside the specification, and naming them by identifier is *correct behaviour rather
than a gap*. No missing document was ever needed. Verified by census over 67 460 documents before
and after: a ~750-line report whose **diff is 15 lines, all intended**.

## The six defects, and what they turned out to be

685 took the pages 681's abstention rule exposed, and the answer is that **on none of them do we
draw the wrong thing** — three documents, three different clauses, each read to its end.

`issue17333.pdf` turns on §9.6.5.4 running out: one `Tj`, character code 0, a two-glyph subset, and
every route the clause states dies — `MacRomanEncoding` gives the code no name, the
`StandardEncoding` fill gives none, both cmap rules *begin* by mapping the code to a name, `post` has
no name to look up. What decides it is the closing sentence, *"a PDF processor may supply a mapping
of its choosing"* — **a permission that ranks nothing**. `issue18042.pdf` is a `DCTDecode` stream of
four ASCII digits, and no clause states artwork for an image that will not decode; `mupdf`'s panel is
`255 ÷ 16` = 15.9375, derived rather than observed. `text_field_own_canvas_calc.pdf` was **already
diagnosed** by ADR 0032 and wanted only its closed form, which prices the departure to six digits.

**And the second defect the round was told to look for was there.** `issue17333.pdf` drew zero
commands and reported `unsupported: []`. `codes_without_a_glyph`'s doc comment names two exclusions
and the code had **a third nobody had written down** — a code §9.10.2 could not *name* was excluded
too, though that gate answers the reader's question while whether the *program* answered is decided
by the glyph the code reached. Fixed; codes reaching no glyph in silence went from 5 documents to
**129**. Two existing tests asserted the opposite and had been wrong since they were written.

## What the new population says, and the price of asking

692 gated `pdf20examples` and `pdfbox` whole, on a rule that is ADR 0005's own precondition made
explicit: **a vote is evidence only where there is a clause the references are both reading.**
`format-corpus` is deliberately damaged files — three programs agreeing there agree about three
recovery heuristics, which is trap 9's shared gap put in on purpose — and `pdf-differences` stays out
on ADR 0393. **Both exclusions are printed rather than asserted**, by a declining census that renders
all four and names every non-agreement; on this run the pages it called contradicted in
`pdf-differences` were exactly the two ADR 0393 had named, the decision reproducing itself.

Its best find is why `pdf20examples` earned its place: on the UTF-8 string and annotation example our
ink, `mupdf`'s and `hayro`'s agree to four significant figures, `poppler` draws a **self-crossing
bow-tie** because it synthesises from `/QuadPoints` in Acrobat's vertex order instead of drawing the
`/AP` — *and the file's own `%` comments say that is what it is for* — and `ghostscript` omits an
annotation whose Print flag is clear. We draw the appearance stream, §6.3.2.2's second obligation.

**The price is stated rather than absorbed**: `ambiguous_undiagnosed.txt` has been empty since
session 379 and now holds 63 `pdfbox` pages. Not one is a regression. **The emptiness was a fact
about one population**, and `doc/todo/00`'s claim that the list is empty and the task is only the
ratchet is corrected.

## Two obligations found by rounds that were not looking for them

**A licence obligation the binaries were not meeting.** 687 found that `pdf-viewer-gtk` and
`pdf-viewer-qt` compile in the standard 14 font programs and **reproduced their notices nowhere** —
no card, no dialog, no `--licences`. There is now one `viewer_host::NOTICE` and all three hosts show
it. This is the kind of thing no gate was ever going to find, and it was found because a round
reading a key table asked what `?` should do.

**Erratum #74, which makes 11 of 25 encrypted corpus documents conforming.** 691 found it inserting
"or 5" into §7.6.6's *"the value of the V entry shall be 4 to use crypt filters"* — under the
published text every AES-256 file's crypt filters are non-conforming, since Table 25 gives `AESV3` no
home but a `/CF` entry and Table 20 requires `/V` 5. `crypt_filters` has read `/CF` at `/V ≥ 4` since
it was written **on no stated authority**, and now has the clause's. That is the **fifth**
Caret-with-no-`StrikeOut` erratum this block, and the automatic sweep is blind to every one of them.

691's method is worth keeping too: **three of its four findings are disagreements between two rows of
the same family** — a shape no ordering by age can produce, which is the argument for reading a
family over working down a blame list.

## The instrument fixed here, and the one that stays disproved

**`tools/round.sh` told two rounds they were session 685.** It computed the session from
`ls doc/history/`, which in a worktree counts the rounds that finished *before the batch started* —
so 687 and 691 were both misnumbered and both told they owed a fifth round's obligations they did
not owe. Each caught it and said so, which is two rounds spending attention on the instrument rather
than the work. **The branch name is the assignment and cannot go stale**, so `round-NNN` now outranks
`doc/history/` and the line says which of the two it used. Verified in a worktree.

**The gitlink guard stays recorded as insufficient.** Nothing this round learned changes 684's
finding: 683 was bitten with `--skip-worktree` set on all four paths, the flag holds against every
shape anyone has been able to construct, and no diagnosis is possible after the fact.
`tools/worktree.sh list` prints the guard's state so the next occurrence is cheap.

## Owed

- **63 `pdfbox` pages in `ambiguous_undiagnosed.txt`** — a queue of measurements, head at 1.11
  against 28.91 at the head of the contradicted ranking.
- **`issue19083.pdf` on the cross-backend gate** (22 → 23 differ). Not a regression: our CPU render
  moved *toward* the references and quorra's did not. `doc/QUORRA_FEEDBACK.md` §24b.
- **Four curves refused by package availability** — brainpoolP256r1/384r1/512r1 and Ed448, each
  named at runtime via `Authenticity::CurveNotVerifiable` with a "what would change it" row.
- **§12.8's third question**, trust: a certificate store and a network, the only thing between
  §12.8.3 and `implemented`.
- **The negatives queue: 16 owed**, and `doc/todo/30`'s items 3 onward.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
