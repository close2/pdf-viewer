# 845 — The population a document was holding

Date: 2026-09-01. ADR **0772**. An oracle round on `doc/todo/12` item 2, which is now closed.

Touched: `crates/pdf-model/tests/oracle.rs`, `doc/conformance/ledger.toml` (§9.8.1),
`doc/todo/12-one-bound-two-jobs.md`, `doc/todo/README.md`,
`doc/traps/oracle-and-references.md`, `doc/adr/0772-…`. **No rendering code, no pixel, no
verdict**: the oracle prints 980 agrees / 60 contradicted / 836 ambiguous before and after.

## The item, and the first thing that had to be fixed

The round was handed the three contradicted pages on which a voting reference *outside* the
consensus meets the bound while we do not — the sharpest population the oracle produces — with the
names ADR 0771 had written in prose. **Two of the three names were wrong.** Read off the gate's own
arithmetic the population is `bug847420.pdf` p1, `issue7891_bc1.pdf` p1 and `freeculture.pdf`
p313; `issue19633.pdf` p1 is convicted by the control like the other 52 (`ghostscript` at ssim
0.98828 against `mupdf`, where that page's vector floor is 0.9900), and `freeculture.pdf` p313 was
missing altogether.

So the fix is not a corrected sentence. `Examined` carries an `ExcludedReading` now — the excluded
reference, how many consensus members hold it outside, how many it was compared with — and
`name_the_pages_the_excluded_reference_survives` prints the complement of the base rate **by name**
under the contradicted ranking, every run. A count cannot be handed to the next round as a to-do
list, and this one had been.

The same field answers a second question the tree had been quoting since ADR 0717: over the 32
pages the gate convicts on the differing fraction with `poppler` and `mupdf`, `ghostscript` fails
the same bound against *both* pair members on **31 of 32**, not all 32. The exception is
`freeculture.pdf` p313, where the widening lifts the bound from the 5.00% class floor to 6.01% and
carries `ghostscript` inside with it. Trap 9, the gate's own printed line and
`CONTRADICTED_GLYPH_EDGES`' note all carried the absolute; the gate counts it now.

## The three pages, read

**`bug847420.pdf` p1 — the three references are one face.** At 8× the line's ink bounding box is
device columns 98 to 1515 *in all three references* and the capital `T` is 82 device rows *in all
three*, where ours is 90 to 1509 and 77 rows. Three programs agreeing to the pixel over 1420
columns are drawing one design. `ghostscript` without `-q` names the file it loads
(`…/Resource/Font/NimbusSans-Italic`, 120 927 bytes of Type 1); `fc-match Helvetica:italic` answers
`…/gsfonts/NimbusSans-Italic.otf`, 95 244 bytes; `mupdf`'s route was not asked, and the raster is
what says it is this design. **Two different files, one published design**, so `objdump -p`, a digest comparison and a `desc` tag all
come back empty — trap 9's sixth bullet arriving on fonts, one level further out than the
`libfreetype.so.6` its font entries are about. §9.5 NOTE 5 puts the choice of face beyond the
standard, and §9.8.1's descriptor route is blocked by this file's own `/CapHeight 500` beside
`/Ascent 728`, which nobody honours.

**`issue7891_bc1.pdf` p1 — inside the bound and 25.6× further from the arithmetic.** The consensus
agrees on the worst tile at 3.02 so the bound is 6.04; `poppler` is 5.98 against each member,
inside by 0.06, and ours is 6.73 against `mupdf`. ADR 0489 had already written that tile out as a
closed form and measured ours at 0.166 levels from it against `poppler`'s 4.255. The two readings
were three paragraphs apart in one note and had never been put beside each other.

**`freeculture.pdf` p313 — the bound falls inside a spread with no gap in it.** The page nobody had
opened. It fails the differing fraction and nothing else, by 0.04 of a percentage point. The five
cross-pair differing fractions that do *not* set the bound run 5.32%, 5.35%, 5.88%, 6.05%, 6.15%,
and the bound derived from the sixth lands at 6.01% — inside that range. The ink ladder was re-run
and still puts us *between* the two voting references at 8×.

**What the three share is the finding.** The control asks where a renderer sits on the deciding
measure, so it fires wherever we are the extreme of an ordering — which is what being on the clause
looks like when the pair that sets the bound departs in one direction. `issue19633.pdf`, which is
not in the population, is the same shape read the other way: ours 1.006 device pixels of stroke,
`ghostscript` 0.564, `poppler` 0.247, `mupdf` 0.219, with §8.4.1's clip and §8.4.3.2's one device
pixel at our end.

## Two stale sentences found on the way, both about the page nobody had opened

`CONTRADICTED_GLYPH_EDGES`' `freeculture.pdf` p313 paragraph quoted mean 2.56, worst tile 12.54 and
ssim 0.9445 where the gate prints 1.88, 9.54 and 0.9685, named none of the four measures the page
actually fails, and then said the page "is contradicted only because `poppler` and `mupdf` agree so
closely that twice their spread is a tighter bound than the floor" — which is **backwards** for this
page: twice their spread is 6.01% and the floor is 5.00%, so the bound the page fails is the one
the consensus *widened*. `--bin quoted` named the first half; the second was found by reading the
sentence beside the gate's line.

## Second track

§9.8.1's row, `partial`, read against the code and the corpus. `/CapHeight` acquires a **third
witness** and it is the sharpest of the three: `bug847420.pdf` states `/CapHeight 500` beside
`/Ascent 728` and a `/FontBBox` reaching 998, so a processor scaling a substitute to it would draw
capitals 27% shorter than this tree's 0.687500 em and 31% shorter than the references' 0.729167 em
— on the very page whose cap-height deficit that row and §9.8's price the entry from. ADR 0267's
condition is met again rather than retired. The 0.729167 em is also measured rather than inferred
now, from `ghostscript`'s own statement of the file it loads and the identical 8× cap rows.

## Gates

The whole §2 sequence, green, run alone. Sweeps run before it: `overtaken`, `quoted`, `unpriced` —
`unpriced` clean, `quoted`'s hits on this round's notes are reference-against-reference figures the
gate never prints, and each now names `examples/compare_rasters` as its instrument. §5's binaries
rebuilt and installed.

## What is left

`doc/todo/12` keeps two things: the consensus half and its 278 pages, untouched and still a
programme; and the substitution table's vector row, where the `mupdf` + `ghostscript` consensus
contradicts `poppler` on 119 of 226 vector pages against 13 of us.
