# 684 — The six agreements a blank sheet was holding up

Second merge round of the block. Four branches, **no conflicts**, and the first merge round in a
long while where the headline number is **lower** afterwards and that is the round's achievement
rather than its cost.

## The sequence, whole, on a quiet machine (load 1.57)

`fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 ·
`nextest` **2452 passed, 17 skipped** · doctests, 0 failures · conformance 182 + 5 + 1 · corpus
**974 documents, 68 incomplete** · `render-quorra` **957 pages at glyph quantum 1/16 — 933 agree,
22 differ, 2 refused** · `fixed_documents` **40 checked, 0 absent** · text, both censuses, dates,
XMP, JPEG 2000 · `cargo deny` all four ok. §5's binaries rebuilt and installed. Ledger unchanged
at 875 rows — 443 implemented, 224 partial, 18 reported, 69 inapplicable, 8 writer-side, 113
out-of-scope, **0 unreviewed**.

**The oracle, and 681's prediction reproduced on the merged tree to the page:**

| | before | after |
|---|---|---|
| agrees | 908 | **902** |
| contradicted | 65 | **60** |
| ambiguous | 786 | **768** |
| not comparable | 13 | **42** |

`our geometry` 2, `reference geometry` 2, `no render` 18 — unchanged.

Two counters that are now printed rather than asserted: the negatives queue at **26 done, 20 owed**
(the script's own count, run here, matching 682), and `tools/state.sh hosts` at **`Query` 20 of 31**,
up one because 683 gave `Query::LogicalSelection` a symbol.

## Why the agree count fell, and why that is the good outcome

681 asked whether a reference whose raster is *flat* should get a vote. 675 had found that the JBIG2
group's whole verdict line on four pages — mean 13.12, worst tile 144.56, differing 5.15%, ssim
0.8990 — is reproduced **digit for digit** by comparing our render with a synthetic white sheet, and
declined to act because acting moves pages between four lists at once.

The round's method is the part to keep. **"Constant" is not the right predicate**, and it proved that
by building it and measuring it: *a flat raster abstains where any reference drew* moved 32 pages and
cost **nine** agreements, three of them wrongly, because a page whose correct rendering **is** a flat
sheet is not a failure. What shipped is narrower — *a flat raster abstains where a reference that
drew marks fails to agree with it*, decided by the same `Tolerance::accepts` that governs every other
agreement — and costs six.

**Those six are six defects, not six regressions.** On every one, two flat sheets outvoted a renderer
that drew, and *our* raster was one of the flat ones: `issue17333.pdf` (`mupdf` and `hayro` draw a
mark we do not), `issue18042.pdf` pages 1–4 (`mupdf` alone draws 15.9 of 255),
`text_field_own_canvas_calc.pdf` page 3 (`ghostscript` and `hayro` draw). **All six printed
"PASS — agrees" before this round.** They are owed to `doc/todo/00` undiagnosed.

Nothing moved toward a verdict that flatters us, and on the further 21 pages where a reference
abstained while two readings survived, no verdict changed at all. The gate prints that census every
run, which is the rule `doc/todo/02` §7 asks for applied to an instrument change: **a count that
improves is not a picture, and a count that worsens is not a verdict either.**

681 also named what the rule cannot reach instead of reaching for it — two pages stay wrong, and every
refinement that would rescue the second one works by reading *our own* render, which is precisely
what would have hidden the six above.

## Two mechanisms of trap 9 corrected, from opposite directions

**681 found the trap's stated *tell* was wrong.** It said to read `%[fx:minima] %[fx:maxima]`, which
reports `0`/`1` for a solid **blue** sheet exactly as for a page full of ink — and one corpus
reference panel is a solid blue sheet. The trap now says `%k`.

**680 found the sixth mechanism over-scoped**, and replaced a claim with a number. "Our evaluator on
either SWOP-family profile predicts all three renderers to eight levels" holds on the sampled ramp
and fails by **eleven levels** on `transparent.pdf`'s deep ink through the *Artifex* profile that
`mupdf` and `ghostscript` both read, while `hayro`'s CGATS profile is within one. The cause is the
black point — exactly as `icc.rs::detect_black` had predicted in prose, with nobody having put a
figure to it.

## The eight groups are finished, and none was an unearned exemption

672 named eight, 675 took six, **680 took the last two** — and both are 675's middle case, mechanism
accounting for the bound but only under ablation. Both ablations came out at the strong end.

**`DEVICE_CMYK_CONVERSION`: the press owns 100% of every failing bound on all five pages**, and the
note had never said which bound any page fails. `transparent.pdf` fails on the differing fraction
alone and converts exactly: ours (28,32,40) against `ghostscript` (25,35,46) is 3, 3 and 6 levels
against a `JUST_NOTICEABLE` of 4, so **one channel of four crosses it** and the metric is the
bottle's own area ÷ 4 — 3.316 against a bound of 1.38. *Two levels of one channel decide that
verdict.* A §7.5.6 incremental update giving each page a `/DefaultCMYK` — our existing code path, no
code changed — puts all five inside every bound at a largest ratio of 0.63. **Per principle 5 that
prices the refusal and does not license the change**, which is the discipline that rule exists for.

**`SUBSTITUTED_FONT`: right unit, wrong operands.** Its 8.65 is our distance from `hayro` — the
furthest of four and **the one that does not vote** — where the nearest voter is `poppler` at 7.44;
the six pairwise reference means run 1.38–3.48 and the quoted 4.64 is none of them; and "twice as far
as any page that is not a link border" is false at `issue15716.pdf`'s 13.96. `doc/oracle-and-corpus.md`
repeated all three. Its ablation is unusually clean — embedding the references' face disarms §9.5
NOTE 5 for everybody, and all three references then render the rewritten file **byte-identically** to
the original on seven of eight pages, so the control is exact. Seven of eight go inside every bound.

And a methodological correction that outlives both groups: **the gate's printed line is the
worst-ratio member of the *agreeing consensus*, not of every reference.** `bug847420.pdf`'s numbers
are `mupdf`'s though `ghostscript` is further on all four. Every future before-and-after over a
contradicted page has to use that pair.

## 682: seven of eight negatives false, and three rules about counting

The queue moved **18 done / 28 owed → 26 / 20**. Beyond the counts:

- **A false negative does not imply owed work.** §7.6.5's witness is a file declined by name with
  `SyntaxError::UnsupportedEncryption` — the row's own sentence working, trap 5 satisfied. Count
  moved, status unchanged.
- **Count the condition, not the noun.** §11.6.5.2's residue is a mask behind an image codec:
  **2882 documents state one and six reach the sentence**, because `soft_mask_entry` consults the
  device scale only where `worth_combining` has already refused the finer grid. `image.rs` carried
  the same false negative in its own doc comment.
- **Probe a positive as well as a zero.** The planted witness caught two false zeros as designed;
  the other direction caught something with no recipe, when the first `/Decode` block scored
  `[255.0 0.0]` a departure on an eight-bit `Indexed` image where that is Table 88 NOTE 2's default
  **reversed** — so the claim it would have retired holds.

Its own `tables` sweep caught two wrong table numbers in comments the round had just written (Table
96 for Table 94, Table 173 for Table 176), and 676's errata habit paid again: **Issue #536** on
§12.3.2.2, never recorded, invisible to sweep 12 because it is a `Caret` with no `StrikeOut`.

## 683: the first thing the owner asked the UI work to produce

`doc/todo/30` item 1 — **all four consumers can now put a selection on the platform's clipboard**, and
§14.8.2.5's choice is made once in a new `viewer_host::copying`: logical content order where the
structure tree reaches every byte, page content order otherwise, **and the order named out loud
either way**. Verified by reading the X11 `CLIPBOARD` selection back from a *second process* under
`Xvfb`: all four byte-identical, 173 bytes, 170 characters.

"No new message" held. What did not: **the C ABI's `pdfv_selection_text` answers in page content
order and `Query::LogicalSelection` reached no symbol at all**, so a C caller could copy and could
not copy right — a fifth of item 5 arrived with item 1. And `viewer-qt` cannot call `QClipboard` from
Rust, which is this tree's rule rather than Qt's: C++ owns the `Host` and Rust never calls a Qt
object, which is what keeps that crate to one hand-written `unsafe` token. Hence a flag and a getter
rather than a second `unsafe extern "C++"`.

`arboard` was taken deliberately and priced — one compiled package on Linux, `image-data` and
`wayland-data-control` both off with their costs written down, and **no XWayland means the copy is
refused by name rather than silently**.

## The guard that is not sufficient, recorded as not sufficient

679 added `--skip-worktree` on each linked corpus so a blanket `git add` cannot stage a symlink over
a submodule. 683 was bitten anyway — **with the flag set on all four paths**, and `4/4` still showing
afterwards. Its branch was clean and nothing reached a commit, so there is no harm and no diagnosis.

In isolation the flag holds against every shape this round could construct: `add -A`, `add -u`,
`add -u doc`, the submodule path named directly. That does not explain 683, and **this file is not
going to pretend it does.** What was built instead is the thing that makes the next occurrence cheap:
`tools/worktree.sh list` now prints, per worktree, whether the guard is on — so a round that has just
been bitten can tell *the guard is off here* from *the guard is on and something else happened*
without spending an hour deciding. `doc/environment.md` says the same in prose: the rule against
blanket staging is what protects you, the flag is a belt of disproved sufficiency, and
`cargo test -p conformance` catches the result either way — the only one of the three that has never
failed.

## Owed

- **Six undiagnosed pages** where a renderer draws a mark we do not, newly visible because flat
  sheets stopped voting: `issue17333.pdf`, `issue18042.pdf` 1–4, `text_field_own_canvas_calc.pdf` 3.
- **Two pages the abstention rule cannot reach**, both named in 681's file with the reason each
  refinement is worse than the defect.
- **The negatives queue: 20 owed**, mostly rows a *name* census cannot settle.
- **`doc/todo/30` items 2 onward**, the ordering 678 built and 683 started.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
