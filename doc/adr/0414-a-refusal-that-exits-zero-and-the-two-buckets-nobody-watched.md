# ADR 0414 — A refusal that exits zero, and the last two buckets nobody watched

Status: accepted, 2026-08-18. Session 579. Takes ADR 0410's own "what this leaves": the oracle's
`not comparable` and `reference geometry` verdicts, diagnosed page by page and ratcheted in
`crates/pdf-model/tests/oracle.rs`. Adds `doc/oracle-and-corpus.md` §3e, keeps the artefacts of a
`not comparable` page, and amends ADR 0410, `doc/todo/00` and `doc/todo/README.md`. The spec half
corrects §12.11.1's ledger row.

## Where this started

ADR 0410 held the `no render` bucket by name and wrote down what it was leaving:

> `reference geometry` (2 pages) and `not comparable` (13) are still printed and held by nothing.
> Neither is an accusation against this tree by construction — one is the references disagreeing
> about the page's size, the other is fewer than two of them producing an image — but "by
> construction" is a claim, and the same claim was true of `no render`.

That is the whole brief. The gate prints seven verdicts; five were held; these two were not.

## What the fifteen pages are

Asked by hand on `doc/oracle-and-corpus.md` §3d's recipe — `pdftoppm`, `mutool` and `gs` with
`tools/pdfref/src/reference.rs`'s own invocations, explicit about the page box — and read with §3d's
two rules, which earned their keep again: read the stderr beside the raster, and a sheet of zero ink
is not a page.

**The claim survives.** Neither class accuses this tree, and nothing in either is a page drawn
wrong or a page not drawn. Two things came out of asking that nobody could have said before.

### The strong result: on four of the fifteen the one reference that drew agrees with us

| page | the reference that drew | ours | apart |
|---|---|---|---|
| `auth-event-ef-open.pdf` p1 | `poppler` 612×792 ink 0.269507 | 612×792 ink 0.264989 | **0.06 of 255** |
| `encrypted-attachment.pdf` p1 | `poppler`, the same two numbers | the same | **0.06** |
| `poppler-67295-0.pdf` p1 | `gs` 612×792 ink 0.426603 | 612×792 ink 0.409477 | **0.14** |
| `issue9418.pdf` p1 | `gs` 3024×2304 ink 20.376 | 3024×2304 ink 20.698 | **3.15**, on a text page whose class bound is 5.00 |
| `bug1978317.pdf` p1 (`reference geometry`) | `mutool` 612×792 ink 10.2767 | 612×792 ink 9.27853 | **1.69** |

One renderer is not a consensus and this is not a vote. What it is worth is the opposite of what a
bucket nobody watches is feared to contain: on every page of these two classes where anybody else
got to a page at all, they and we drew the same one.

The two encryption pages are the mirror of `NO_RENDER_NEEDS_A_PASSWORD` and the distinction is worth
keeping. There, four independent derivations of §7.6.4.3's key agree that the empty user password is
**not** the document's, which is principle 5's direction of inference. Here two say it is and two say
it is not — `mutool` *cannot authenticate password*, `gs` *This file requires a password for access*
— which is trap 9's two-against-two: a question with an answer in the clause rather than a tie, and
§7.6.6 puts a refusal on the stream whose key is missing rather than on the document.

### The finding: `reference geometry` is wrong about both of its own members

`reconcile` takes the largest set of references agreeing about the page's extent, and with two
rasters of different sizes there is no such set, so the verdict reads *no two references agree about
the page size*. On both pages that sentence is true of the rasters and false about the page.

**`pdftoppm` writes a 1×1 raster and exits 0 when it fails to create a page.** So a refusal enters
the reconciliation as an opinion about the page's extent, and outvotes the one renderer that drew.
Neither page has three references disagreeing about a size; both have one reference and two refusals:

- `bug1978317.pdf` page 1 — `poppler` prints *Page annotations object (page 1) is likely malformed.
  Too big: (32768)* and *Failed to create page*, and `gs` fails silently under the `-q` this gate
  passes. `mutool`'s 612×792 is the only reading, and ours is the same page.
- `boundingBox_invalid.pdf` page 3 — `pdftoppm` emits its 1×1 with **no diagnostic at all**, which is
  the sharper half: the tell here is not a message but a size.

This is trap 3 one step further along. That trap says to check what question each reference is being
asked before reading its answer as a verdict; this says to check whether it answered at all, because
a program can decline while returning success — which is trap 12b's rule about a *dependency*
arriving one directory over, in an external process instead of a library.

**The classifier is not changed for it**, and that is a decision. A blanket "a 1×1 raster is a
refusal" is a rule about one program's output shape and would misread a page whose crop box really is
one point square; curve-fitting the harness to `pdftoppm`'s failure mode is the same mistake as
curve-fitting the renderer to its output. What the verdict already does is print every reference's
size, so the group's own note says to read them, and the ratchet fails the build if a third page
arrives.

### `boundingBox_invalid.pdf` page 3, which ADR 0410 named and left

The file's three pages are one construction apiece with the producer's own captions, and ADR 0410
took the first. The third is *Empty /CropBox and /MediaBox intersection*: `/MediaBox [0 0 600 800]`
with `/CropBox [600 800 1000 1000]`, two rectangles meeting at one corner.

§14.11.2.1 states the rule and this tree applies it:

> If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the media box,
> a processor shall treat the box as its intersection with the media box.

The intersection encloses no area, and the clause states no recovery for that — which is ADR 0389's
and ADR 0410's shape a third time. Table 31 does state one, and this is where the third case parts
company with the first two: `/CropBox`'s "default value is the page's media box", so an unusable crop
box falls back to a rectangle **the file itself states** rather than to one this program invented. A
guessed sheet must not look like a measured one (ADR 0389); a sheet the document stated is measured.
So the fallback is right, it is not reported, and both of those are the same argument rather than
two.

We draw 600×800 at ink 1.502 and **no reference draws the page at all**: `mutool` produces 612×792 of
ink 0, `pdftoppm` its 1×1, `gs` exits *Unrecoverable error*. There is nothing here that could
contradict anybody, which is the honest content of the verdict.

## What is ratcheted, and over which population

`REFERENCE_GEOMETRY_A_REFUSAL_WEARING_A_RASTER` (2), and four `NOT_COMPARABLE_*` groups covering the
thirteen: §7.6's encryption two references decline (2), a cross-reference table one reference rebuilds
(3), a page no reference reaches at all (7), and the decompression bomb two references are killed on
(1). Held to equality in both directions by `assert_ratchet`, and — as with `no render` — over **all**
pages rather than the ones we call complete: `not comparable` is 7 complete of 13, and what these two
verdicts are about is what the *references* did, so our own completeness is the wrong filter.

`group_name` learns both prefixes, so
`every_group_of_pages_carries_a_diagnosis_naming_one_of_them` covers the five new groups.

Every count is unchanged by the round: 1794 pages, 906 agree, 67 contradicted, 786 ambiguous, 2 our
geometry, 2 reference geometry, 13 not comparable, 18 no render.

## The bucket whose evidence was deleted

`examine` called `remove_dir_all` on a page whose references could not be reconciled, on the
reasoning that a page with fewer than two references has nothing to look at. It has exactly what a
reader of this bucket needs — whichever reference *did* draw — and destroying it made this the one
bucket that cannot be diagnosed from disk at all. Three of these thirteen pages have a reference
raster worth seeing and it had to be re-rendered to see one.

Fixed, and the shape is worth the sentence: **the bucket where the evidence is thinnest is the one
where throwing it away is cheapest to justify.** Thirteen pages of PNGs, and our own raster is
written beside them.

## The spec half

Three of `doc/todo/01`'s sweeps that the round before did not run — the ninth (`tables`), the
fifteenth (`entries`) and the second (`unread`).

`tables` and `unread` are clean, and a clean run is a result. Every one of `tables`' surviving
suspects is one of its three known noise shapes, and the one that reads sharpest was checked rather
than assumed: `hostile_budgets.rs` says "Table 87 gives an image mask no `/Mask`" and the sweep reads
that as a denial the table contradicts, because Table 87 does state `/Mask` — with the qualifier
"(Optional; **shall not be present for image masks**; PDF 1.3)", so the sentence is exactly right and
the denial is conditional. `unread`'s sharpest hit is §12.6.4.3's `/SD`, and `destination.rs` reads
Table 202's `/SD` while the row denies Table 203's: one short key, two clauses, which is that sweep's
oldest noise shape.

**`entries` paid, on the discriminator its own description names** — an entry the row's *note* does
not name. **§12.11.1 is `implemented`, its note enumerates Table 273's common entries, and it walks
past `/RH`** — the fifth of the five, sitting between the `/V` and the `/Penalty` the note does
discuss. The entry carries a `shall`: it names "a requirement handler that shall be disabled (not
invoked) if the interactive PDF processor can check the requirement specified in the S entry".

Reading it against §12.11.5 says the requirement is met by construction rather than skipped: such a
handler is an ECMAScript — "[t]raditionally, requirements handling has been accomplished with an
ECMAScript segment" — and `CLAUDE.md` excludes ECMAScript, so every handler a file could name is
disabled here whatever the file says. **The condition is worth stating rather than assuming, because
it fires**: `Kind::unmet` answers all twenty-five of Table 275's types, so this processor *can* check
the `/S` entry and does owe the disabling. §12.11.5's own row has carried that reading since it was
written.

So nothing is owed and the row keeps `implemented` — what was wrong was its enumeration, which is
`doc/todo/01`'s fifth failure shape between two **neighbours** rather than between a parent and a
child. A family's row is not maintained by the session that reads the clause next door, because the
clauses do not cite each other.

## What this leaves

- **Nothing of `doc/todo/00`'s verdict work.** All seven of the gate's verdicts are held by name
  now. What is left of that item is its two standing halves: the equality ratchet in both
  directions, and step 7's ink sweep after a round that moves pixels.
- **`PDFBOX-4352-0.pdf` is still reachable by a rebuild**, which ADR 0410 recorded and this round did
  not take.
- **The `code` arrays several `entries` hits point at are short rather than wrong** — §7.6.2's four
  Table 20 entries are read by the crypt code and the row names one file. Worth a sweep round of its
  own; not worth conflating with a claim about a clause.
