# ADR 0772 — The population a document was holding

Status: accepted, 2026-09-01. Session 845, an oracle round on `doc/todo/12` item 2.

The three contradicted pages on which a voting reference outside the consensus meets the bound
while we do not are read, and none of them is a defect of ours. Getting to them cost the first
part of the round, because **the list was in an ADR rather than in the gate and two of its three
names were wrong** — so the gate names the population now, and one figure this project has quoted
since ADR 0717 is counted rather than repeated. No verdict, bound or pixel moves.

## 1. The list that had to be rebuilt before it could be read

ADR 0771 §4 measured the base rate of trap 12's control — *put a reference where our render stands
and ask what the consensus concludes about it* — over the whole contradicted pool, found it holds
on 52 of 60, and named the survivors in prose: `bug847420.pdf`, `issue19633.pdf`,
`issue7891_bc1.pdf`. Read off the gate's own arithmetic, the population is

| page | consensus | excluded | its worst against a member |
|---|---|---|---|
| `bug847420.pdf` p1 | `poppler` + `mupdf` | `ghostscript` | mean 3.48 / 5.00, differing 7.53% / 8.09%, ssim 0.9681 / 0.9000 |
| `issue7891_bc1.pdf` p1 | `mupdf` + `ghostscript` | `poppler` | worst tile 5.98 / 6.04 |
| `freeculture.pdf` p313 | `poppler` + `mupdf` | `ghostscript` | differing 5.35% / 6.01% |

`issue19633.pdf` page 1 is **not** in it: `ghostscript` against `mupdf` is structural similarity
0.98828 where that page's vector floor is 0.9900, so the consensus convicts it like the other 52.
`freeculture.pdf` page 313 was missing.

Two implementations agree on that, which is trap 13's rule applied to a sweep this round wrote: a
Python loop over `examples/compare_rasters` on the artefact directories — cropping each reference
to the common size `normalise::to_common_size` produces, and applying the bound the gate printed on
the page's own line — returns the same three pages and the same 51 / 3 / 5 split of the 59 lines it
can parse, against the gate's 52 of 60.

**The fix is not a correction to a document.** `Examined` now carries an `ExcludedReading` — the
excluded reference, how many consensus members hold it outside, and how many it was compared with
— and `name_the_pages_the_excluded_reference_survives` prints the complement of the base rate by
name under the contradicted ranking, every run. The count was already printed; a count cannot be
handed to the next round as a to-do list, and this one was.

## 2. `bug847420.pdf` page 1 — the three references are one face

The sharpest-looking of the three, because `ghostscript`'s FreeType is its own statically linked
copy and trap 9's usual answer (*the convicting pair shares a glyph rasteriser*) does not reach it.

It does not survive asking what the three are reading. At 8× — `render_at` against
`pdftoppm -cropbox -r 576`, `mutool draw -r 576` and `gs -dUseCropBox -r576` — the line's ink
bounding box is device columns **98 to 1515 in all three references** and the capital `T` is **82
device rows in all three**, where ours is columns 90 to 1509 and 77 rows. Three programs agreeing
to the pixel over 1420 columns and to the row on a cap height are drawing one design, not reaching
one verdict.

They reach it separately and from different files. `ghostscript` without `-q` says which: *Loading
font Arial,Italic (or substitute) from /usr/share/ghostscript/Resource/Font/NimbusSans-Italic*,
120 927 bytes of Type 1, reached because `Fontmap.GS` maps `/Arial,Italic` to `/Arial-ItalicMT` and
nothing maps that. `fc-match Helvetica:italic` answers
`/usr/share/fonts/gsfonts/NimbusSans-Italic.otf`, 95 244 bytes, which is where `poppler` goes once
it has taken `Arial` for the base-14 Helvetica. `mupdf`'s route was not asked and does not need to
be: whatever it reads, the raster says it is this design. **The two files that were named are
different files** — different formats, lengths and digests — so `objdump -p`, a
digest comparison and a `desc` tag all come back empty. What is shared is URW's Base-35 clone of
Helvetica, which each of the three went and got.

That is trap 9's *sixth* bullet — implementations agreeing because each independently obtained the
same published thing — arriving on fonts, where the trap's existing entries are about
`libfreetype.so.6`, the rasteriser object. The tell is a rendering metric rather than anything a
dependency graph shows, and the bullet is written down beside the FreeType one.

The specification puts the choice beyond itself twice. §9.5 NOTE 5: the results "depend on the
availability of fonts in the PDF processor's environment". And §9.8.1's route — "[t]hese font
metrics provide information that enables a PDF processor to synthesise a substitute font or select
a similar font when the font program is unavailable" — is blocked by this file's own descriptor,
which states `/CapHeight 500` beside `/Ascent 728` and a `/FontBBox` reaching 998. Scaling a
substitute to 500 would draw capitals 27% shorter than ours and 31% shorter than the references'.
Nobody honours it.

## 3. `issue7891_bc1.pdf` page 1 — inside the bound and 25.6× further from the arithmetic

The consensus agrees on the worst tile at 3.02, so the bound is 6.04; `poppler` is at 5.98 against
each member, inside by 0.06, and ours is 6.73 against `mupdf`, outside by 0.69.

ADR 0489 had already written the same tile out as a closed form — a black fill through a luminosity
mask, so every pixel is `255 × (1 − L)` — and measured each renderer against it: ours **0.166**,
`hayro` 2.814, `poppler` **4.255**, `ghostscript` 4.596, `mupdf` 6.723. The two readings were three
paragraphs apart in one note and had never been put beside each other. **The reference that meets
the bound is 25.6 times further from what the file says than the render that does not.**

## 4. `freeculture.pdf` page 313 — the bound falls inside a spread with no gap in it

The page ADR 0771's list was missing, and the only one of the three nobody had opened. It is one
leaf of a book whose other three hundred are `ambiguous`, it fails the differing fraction and
nothing else — mean 1.88 of 5.00, worst tile 9.54 of 40.00, ssim 0.9685 of 0.9000 — and it misses
by 0.04 of a percentage point, 6.05% against 6.01%.

The ladder this group is diagnosed by still holds. Ink in levels of 255, `-alpha off -channel R`,
at 72 and 576 dpi: ours 5.854 → **5.993**, `poppler` 5.943 → **5.983**, `mupdf` 6.013 → **6.019**.
At eight times the resolution we are *between* the two references that vote, so the marks are the
right marks and the difference at the page's own scale is glyph coverage.

The verdict is where a threshold fell. The five cross-pair differing fractions that do **not** set
the bound are 5.32%, 5.35%, 5.88%, 6.05% and 6.15% — `ghostscript` against each pair member, ours
against each, and ours against `ghostscript` — and the bound derived from the sixth, the pair's own
3.00%, lands at 6.01%: inside that range, 0.13 points from the top of it. `ghostscript` is 0.69
points inside a cut through a continuum and we are 0.04 outside it.

## 5. What the three have in common, which is the finding

The control asks **where a renderer sits on the deciding measure**. So it fires wherever we are the
extreme of an ordering — and being the extreme is what being on the clause looks like when the pair
that sets the bound departs in one direction. `issue19633.pdf`, which is not in the population, is
the same shape read the other way: in ink the four are ours 1.006 device pixels, `ghostscript`
0.564, `poppler` 0.247, `mupdf` 0.219, with §8.4.1's clip and §8.4.3.2's one device pixel at our
end, and `ghostscript` is *between* rather than *inside*.

So `doc/todo/12` item 2 is closed. It is not that the population was uninteresting; it is that
membership of it is a statement about position on a one-dimensional spread, and on all three pages
the specification's or the page's own answer is at our end of that spread.

## 6. And one figure this project has repeated since ADR 0717 is 31 of 32

ADR 0717 measured, over the 32 pages the gate convicts on the differing fraction with `poppler` and
`mupdf`, that `ghostscript` fails the same bound against **both** pair members on all 32. The gate
counts it now instead of the sentence being copied — `ExcludedReading::outside_of_every_member`,
printed on the same line as the population count — and the answer is **31 of 32**. The exception is
`freeculture.pdf` page 313, and the mechanism is the widening rather than the renderer: that page's
bound is 6.01% rather than the 5.00% class floor, and `ghostscript`'s 5.32% and 5.35% are outside
the floor and inside the widened bound. Nothing about ADR 0717's conclusion changes — the two
distributions still do not overlap, and `ghostscript` still sits further outside than we do on 27
of the 32 — but a claim quoted in three documents was an absolute and is not.

## 7. What this does not claim

That any page is drawn differently, that any bound is right, or that the control is worthless. The
oracle prints 980 / 60 / 836 before and after. What changed is that the population is the gate's
rather than a document's, that each of its three members has a diagnosis, that trap 9 has the
shared-substitute mechanism written down, and that one absolute is a count.
