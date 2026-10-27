# Where we are — what this program can already do

Status: **standing** — a capability list, not a plan. Every sentence below is something the
program does today.
Read by: a round asking whether the program already does the thing it is about to build, or which
clause a capability came from. **Not read to run a gate or to take a decision**, which is why
`doc/HANDOVER.md` points at it rather than holding it.

**It carries no counts.** `tools/state.sh` prints those; a population named here is named because
a *claim* rests on which population it is (ADR 0405), never as bookkeeping. **"The three
windows"** are `quorra`, `quorra-gtk` and `quorra-qt`; `quorra-confined` is named where it does the
same, and "all four windows" counts it among them.

**The project owner's current target, stated on 2026-10-14, is code improvement**: "switch to code
improvements/refactoring (bug fixes are ok as well) rounds until I say otherwise", with PDF/A
remedies continued beside it ("[p]lease continue with pdf/a remedies in parallel future rounds",
2026-10-15). The performance target before it (2026-10-10: loading and display time, zooming
especially, on the processor and on the device) keeps what is chosen and not yet built in
[todo 69](todo/69-a-mark-sized-before-it-becomes-geometry.md) (ADR 1817), and what was built toward
it is said below where it was built. This paragraph is the one in this file that is not a
capability.

What the program still owes is
[`doc/todo/README.md`](todo/README.md), one file per item.

## The state of play

A PDF **viewer**.

### What a page draws

It **draws** what a page says: geometry, colour, images, shadings — including §8.7.4.3 Table 77's
`/Background`, the wash a shading *pattern* asks for outside its own bounds, which §11.6.7 makes
one painting operation with the shading rather than two — patterns, embedded text,
transparency groups — composited in the blending colour space the page or an isolated group
states, four components as two rasters (ADR 0262), one component as one, `DeviceGray` on the
channel (ADR 0790) and `CalGray` or a one-component profile through its curve (ADR 0792), and
three CIE-based components — `CalRGB`, an RGB profile — as the space's own components through
curves and a grid (ADR 0797), a group's result carried into such a parent as the conversion's three
stages — the device's decoding, a linear map and the space's own encoding — rather than as one
sampled grid (ADR 1267) — soft masks, with §11.6.5.2's `/Matte` undone in the parent image's own
space before any conversion, as the clause orders, and a mask's samples read as the opacity whatever
one-component space it states (ADR 1268), a JPEG 2000 parent decoded at a reduced level
included, its mask carried onto that grid (ADR 1324) — whose `/Luminosity` groups in such a space take
§11.5.3's `Y` as three summed curves where the space decomposes, as a sampled grid where it
does not (ADR 0851), and — where the space has *four* components — off §11.4.7's pair of rasters
inside the mask, over a grid of the press's own four axes (ADR 0857) — as a `DeviceCMYK` group
whose content blends is, so that §11.3.4's complement meets each component (ADR 1342) — and
annotations both from
stored appearance streams and
constructed where the standard states one — including §12.5.6.4's seven icons, whose artwork is
this processor's own because the clause requires one and draws none, and §12.5.6.15's four and
§12.5.6.16's two, whose clauses only *recommend* one and whose names name objects, and a `Movie`
annotation's §13.4 `/Poster` image fitted into its rectangle, and a 3D or rich media annotation's
normal appearance as the inactive state §13.6.2 makes the default, the playing and the artwork outside
the clause 13 entry as `doc/questions/A254` amended it (ADRs 1561, 1946) — and a markup
annotation drawn from **the group it belongs to** rather than from itself, which is §12.5.6.2's
nine shared entries. §12.7.4.3's variable text is constructed in any script: a value in a
right-to-left, cursive one is joined by the Unicode Character Database's own tables, ordered by UAX
#9 and set in a face from the machine where the compiled-in fourteen have no glyph (ADRs 1413,
1414), which a save writes into the file where the face's licence permits: a `glyf` face subset
(ADR 1425), a `CFF ` one subset as a bare CID-keyed program under `/FontFile3` (ADR 1449), an
accented character in it composed into the outline it draws (ADR 1486), whole
where its licence forbids subsetting (ADR 1438); in memory a `CFF ` face is described as the
`CIDFontType0` Table 124 puts it under (ADR 1450); a `/DA`'s font is loaded once an interpretation
for every layout that names it (ADR 1956), and a `/DR` font named by reference once for its layout
and the appearance's `Tf` alike (ADR 1974). A font the document did not embed, other than
one of the standard 14 the tree compiles in and answers first (ADR 1483), is stood in for by a
machine face ranked by its descriptor — Table 120's weight, slope and width against each
face's own `OS/2` — among the faces that draw as much of its script as any, which is one search
for a page's font and an interface's label alike (ADR 1441). A non-symbolic TrueType font's code
that none of §9.6.5.4's steps reaches is drawn through the glyph the (3, 1) subtable gives the one
character its `/ToUnicode` states for it (ADR 1943). A simple font's widths are Table 109's from
`/FirstChar` through `/LastChar` and no further, a code past a stated `/LastChar` taking the
descriptor's `/MissingWidth` however long the array runs (ADR 2181), and a CFF program's units per em
is its `FontMatrix`'s scale kept fractional, so its outlines and advances are read under the matrix
the program states (ADR 2180).

### Three rasterisers behind one display list

**Three rasterisers behind one display list**: `render-cpu` is the correctness
oracle — and it is one because it computes a path's coverage of a pixel as the **exact integral of
§8.5.3.3's winding number over §10.7.4's half-open pixel square** rather than sampling it on a
lattice — a row at a time, so that a page is cut into strips across any stroke it draws
that way, a curve that cannot fold included, every fill the row walk draws freeing its rows too and a mark's self-overlap judged over the whole mark rather than its strip (ADR 1837), a rectangle stated at a strip's edge row as the library's rectangle converter states it unclipped, so that a cut across one changes no byte (ADR 2084), an unfiltered image's pixel coloured by the sample its centre maps to on the page's own device grid, so a strip takes the sample the page does (ADR 2100), a rectangle's converter chosen by the page rather than by the strip, so the first strip of a page past 8191 rows states its rectangles as the page does (ADR 2116), a rectangle inside one device row written by its closed form, where the library's own branch for it comes out under its area (ADR 2117), a fill, a stroke's outline, a folding stroke's pieces, a rectangle's closed form and an image's square measured on the page's grid in every strip and band, only their write moved by the surface's corner (ADR 2133), and so are §10.7.4's marks and bands, a dot's disc, a long mitre's wedges, a clip's own mask and a cone's, a washed shading's or a mesh's raster (ADRs 2148, 2167), the library's own converters keeping the surface's matrix (ADR 2149) and asking their range of the page (ADR 2166), and a tall mark is measured in runs of rows rather than handed back (ADR 1819); a single straight stroke whose ink lies within a 2 × 2 block of pixels is painted from its exact closed-form area in the frame where the pen is the unit disc, without entering the library at all (ADR 1829), §8.5.3.2's round dot as the same set at length zero, in every spelling and inside a longer path where no two dots share a pixel (ADRs 1857, 1875), composited at its area and rounded once when stored (ADR 1876), and measured on the page's own grid in every strip, so a cut through one is exact (ADR 1847); a page is cut into as many strips as the pool drawing it is wide (ADR 1859), the cuts decided on one thread from the list read on that pool, every number of it the serial reading's (ADR 1882), the bounds the list keeps borrowed rather than copied (ADR 1940) — leaving the library beneath it the axis-aligned rectangle (ADR 1082), measures a stroke some of
whose curves bend more tightly than its half-width — those curves as pieces, the stretches between
as the stroker's outline (ADRs 1348, 1359) — from the boundary of the set the pieces fill, its
chords at a twentieth of a pixel, its mitres and caps the path's own and its round joins turned
through the way the path arrives (ADRs 2122, 2252, 2268, 2269), a piece the path states twice
weighed twice and a thin piece entered from a vertical ray, every mark it still declines named to
the draw's caller and said in every window, once a condition on the terminal and the page in the
title — a C caller's window raster and a page the confined worker drew as pixels among them, a
stand-in's draw deliberately not (ADRs 2280, 2281, 2290, 2316) — lifts a shape too small to show to one level as a whole
shape, which is what §10.7.4 asks the lift of, so the sliver beside a shape that does show keeps its
own level rather than being painted as ink (ADR 1374), and measures a path
whose portions overlap as the set its fill rule declares inside rather than composite the portions
with one another, which §11.6.2 forbids (ADR 1341) — `render-cpu` is also where a **non-isolated group the file
composites under a mode other than Normal is drawn** instead of reported, by performing §11.4.4's
backdrop removal (ADR 1107) — which `render-raster` draws too, through raster's own reading of
the clause (ADR 1307) — and where §10.7.4's substituted width for a mark too thin to measure
begins **one level of 255 below visible rather than one whole device pixel below it**, so between
those two widths every backend draws the shape the document states (ADR 1102) — and where a matrix
of rank one maps a mark onto a line, every backend draws a fill as that line and a stroke as the
image of its set, one interval per subpath or dash computed from the path, its width, caps, joins
and dashes, so the one pixel §10.7.4 is departed on is a shape with no extent in either axis (ADRs
1348, 1360, 1060); `render-gpu` is Vello and the backend it is compared against — they agree to the channel
over `test-scenes`' fixtures **and over real pages at a real window's resolution** (ADR 0127) — and `render-raster` is the
third, over this tree's `raster/`, the document renderer this project commissioned
(`doc/RENDER_LIBRARY.md`), **what the window actually presents with**, held against the
processor's raster over the whole corpus at the page's own scale and at four times it — a page the
two still differ on held by name to a bound measured against a per-pixel reference from its own
geometry (ADR 1435), an isolated group that composites in a blending colour space of its own drawn
as a frame of its own on the same device and placed back as device pixels, which §11.4.5 makes
exact (ADR 1471), and a target past the adapter's side limit drawn in tiles and stitched (ADR
1472), a frame's budget being the largest buffer the adapter can make rather than a fixed figure
(ADR 1823), and the same thin stroke the processor paints in closed form drawn as the device's own line primitive, its area per pixel computed in the shader and its budget counted per mark (ADR 1835), §8.5.3.2's round dot among them as that line at length zero (ADR 1857), every one-segment stroke a line at every view with its width resolved per placement, so a page of lines keeps its scene across a zoom and a step only re-encodes it, its lines' closed forms made on other threads ahead of the walk (ADRs 1877, 1878, 1884), a replay of an unchanged view reading its instance streams from the device rather than sending them again (ADR 1993), those streams charged against a residency budget the device states and a page past it sending them every frame and saying so once (ADR 2030), a resident view drawing only the runs of line instances whose conservative bounds reach it, the instances' bytes, their order and the shader's arithmetic unchanged (ADR 2188), the op list built once only after the line lane maps on the device because the host's cull ties it to the view (ADR 2008), and each frame's cost carrying the stream every byte crossed in, said at the end of the trace's frame line (ADR 2009), a cropped read-back's straight alpha rounded to nearest as the whole page's is (ADR 2183), and an offscreen frame drawn from the marks that reach it as a window's is (ADR 1858) — its strokes, fills, clips and images each drawn as the set the clause defines rather
than as a sample of it, a clip meeting a mark's coverage as §10.7.4's intersection, and every
frame the same bytes at every thread count, which the corpus gate holds by drawing each page again
at one thread; `doc/crate-map.md`'s `raster-gpu` row names the module of each construction (ADRs
1375, 1389, 1395, 1397, 1407, 1409, 1419, 1420, 1421, 1431, 1435, 1443, 1444, 1445, 1455, 1467,
1468, 1479, 1480, 1491, 1492, 1503, 1505, 1513, 1517, 1529, 1541, 1555, 1567, 1582, 1594, 1595). **A shading end
`/Extend` leaves off stops at that end's own position** — a hard transparent stop in every
backend rather than a fade squeezed inside the ramp, and within half a texel of Vello's fixed ramp (ADR 1387).
**raster reads a ramp at each pixel's own `t`**, interpolating its own stops rather than a table of
colours sampled from them, and hands the target the level nearest an opaque answer and a translucent one to the blend
(ADRs 2310, 2350); a function program's answer,
which reaches the device only opaque, leaves as its level (ADRs 2311, 2366), and an opaque solid paint leaves the rectangle, line and coverage lanes as its
level while an antialiased edge is left to the blend, every level taken in integers so that each adapter writes it (ADRs
2324, 2325). The oracle's grid of a function-based shading holds the same level for the same answer (ADR 2340); it
draws an opaque axial or radial ramp from its own cells, the level nearest the ramp's colour at each pixel's `t`, a fill's
over its shape and a stroke's over its outline's device bounds (ADRs 2356, 2404), and hands an opaque solid paint over moved off a half level, so a pixel it covers wholly is that level whatever
shape carries it (ADR 2357); a translucent ramp it hands to `tiny-skia` is the library's store, which breaks a
half-level tie its own way (ADR 2341), as a translucent ramp over a translucent group meets in the composite's alpha, where the device draws the exact page's level (ADR 2367). **Where the device's own coverage lane, `Coverage::Gpu`, is
chosen, it samples a pixel where a committed search put the samples**: each with a column and a row of its own, the pattern whose
worst straight edge is nearest the area behind it among those no wider, in either of raster's ADR
0070 widths, than the count's rank-1 lattice, and that lattice at a count no search was run for,
the table derived by `raster-gpu`'s `examples/sample_pattern` (ADRs 2296, 2312, 2313).

### Interpretation, and what bounds it

A content stream that re-enters
itself at the nesting bound is reported as the cycle it is, by the stream it re-entered; what a
page's interpretation keeps is bounded in bytes — its commands, dashes, clips and painted
paths charged as they are made, the text it reads back as it is copied — so a page whose tiling
copies multiply its clips stops with a report rather than past the worker's memory, and the bound
is a count of bytes rather than a clock, so the prefix drawn is a function of the file alone (ADRs
1507, 1810); no count of operators refuses a page, so a valid drawing of tens of millions of
operators is drawn whole and a page that is only slow is the person's to stop (ADR 1809); **a
page's stream of a mebibyte or more is read on many threads**, divided after the `Q`s where only
§8.4.2's state is open, each piece begun from the state its predecessors' lasting statements compose
and the pieces joined into the list the page read whole states, and a stream holding anything the
join does not carry is read whole and says why (ADR 1880); the pieces start as the inflate passes
them, cut after a `Q` a byte scan guessed and the lex before confirmed, and a `gs` or an optional-content
section open across a cut is composed into the next piece (ADR 1894); each segment is let go once its piece
has read it, and a piece's commands are copied into the page's list once (ADR 1924); text shown before any `Tf`, which §9.3.1
says shall come first, reads none of its strings, each stepped over by §7.3.4's own
grammar, and a report a page repeats is one lookup rather than a sentence built again (ADR 1521);
and a font
program whose every glyph draws and draws nothing is what the file states rather than a report —
which is §9.7.6.3's own route for a CID-keyed program holding CID 0 alone (ADR 1411). The Vello
backend **bands a target the device cannot draw in one pass**, because its working buffers are fixed
constants with no knob and a page of small text at a laptop's resolution can exceed them.

### Codecs and encryption

JBIG2 and
JPEG 2000 in a confined worker — its colour specification chosen by §7.4.9's precedence rather
than the codec's first box, every enumerated space of the JPX baseline that ITU-T T.801 M.9.2.4
lists drawn as defined, CIE Lab under the white point of whichever illuminant it names, and PIMA 7667's e-sRGB and
e-sYCC by that text's own equations into the connection space (ADR 2292), except CIEJab, whose
defining text is not held and which takes the clause's device fallback rather than a refusal (ADRs
1383, 1713) — Group 3 and Group 4 fax there too through this tree's own ITU-T T.4
and T.6 decoder, which conceals the damaged rows §7.4.6's `/DamagedRowsBeforeError` tolerates as the
clause states (ADR 1349), and Table 13's `/ColorTransform` read where the clause states it —
the entry, the `APP14` segment that silences it, and the component count, ranked in the order
§7.4.8 gives them (ADR 1183); a JPEG is decoded in bands on the pool wherever a band reads the frame as
the whole decoder would, an image is reduced to the page's scale across the device's threads, and
a soft mask is decoded beside its image (ADRs 1433, 1457, 1469, 1481, 1493, 1513, 1519, 1557). Encryption at every revision Table 21 lists and every method Table 25 names, in both
directions — including revision 5, whose algorithm is the Adobe extension the table points at
rather than a clause of the standard (ADR 0820).

### Navigation, presentation, and what a document says about itself

§12.3.2's destinations, §12.3.3's outline, §12.4.2's page labels, §12.5.6.5's links
performing every one of Table 201's actions that is not clause 13's media — `JavaScript` at the
reader's `Scripts` level, as the scripts below say, and a launch of a program the one half this host never performs, with
`pdf_model::action`'s table the list — and a rendition action's clip judged by §13.2.2's viability,
reported with its §14.9.2.4 alternate text and handed to the host at the reader's level for Table
286's `/TF`, with no player started (ADRs 1960, 1961), a media clip section's §13.2.4.3 offsets judged
against the clip's own container headers, which no sample is decoded to read (ADR 2452), and a movie's or a sound's file through the
same hand-off, a sound holding its own samples written as a RIFF WAVE file first (ADRs 2382, 2398),
while a rich media annotation's §13.7.2.3.2 assets are listed and saved as embedded files from its page
and handed to no player (ADR 2399) — §14.9's accessibility entries, §12.4.4's whole presentation
read **and played** — every one of Table 164's transition styles drawn frame by frame (ADR 0230),
four of them — `Blinds`, `Dissolve`, `Glitter`, `Fly` — at a quantity the table does not state,
which this program chose and says is its own (ADR 1299), with §12.4.4.2's states walked inside a page before an arrow key turns it, on the mode
a host states because that clause conditions its whole state machine on one (ADR 0316), and
§12.6.4.15's transition action played **outside** a presentation too, because that clause says the
transition is the action's and not the mode's — the same two faces and the same clock, driven for
one effect instead of a slide show (ADR 1216), each transition ending on the page it moves to and
nothing else between the two pages on `quorra`'s window (ADR 1853) — and
everything a document says *about itself*: §14.7's logical structure, §14.8's
tagged-PDF vocabulary, §7.11.4's embedded files, §14.13's associated files in **both** of
§14.13.2's forms — the embedded one listed, and the one that lives outside the document named
out loud when it opens, since following it is refused and naming it never needed a filesystem
(ADR 0918), §14.13.10's form whose named resource is the array itself read beside the dictionary
forms (ADR 1461), with §14.13.4's files of the page a reader is *on* listed beside the document's own,
which is where the clause puts them and as far as a panel may reach without walking the page tree
at launch (ADR 1186) — §12.2's viewer
preferences, §12.11's requirements, §7.12's extensions and §14.3.2's XMP.

### Fuzzing

The parsers that read bytes a stranger chose are **fuzzed**, a target each — `fuzz/fuzz_targets/`
is the population, every target with its line in `doc/verify.md` and its arm in `fuzz/seeds.sh`,
and `tools/state.sh fuzz` prints what the disk holds of each — and two targets are differential, a JPEG's band plans against its whole decode and the
meet's area against an exact integration — each seeded from what the disk's documents hold,
each recipe by the smallest seed of each shape its target branches on (ADRs 1559, 1571), and a
corpus on disk is asked whether it is stale against fresh seeds before a campaign starts on it
(`fuzz/seeds.sh check`, ADR 1559) — and what a campaign found is fixed: a token past the content
window's ceiling is stepped over by the
grammar that ends it, a composite glyph's cycle is found in time linear in the glyph graph, a face
written whole holds every glyph it is asked for, a band is refused wherever the whole decoder reads
the frame another way, and a page-tree node met beneath itself stands for no pages (ADRs 1423, 1424,
1439, 1495, 1496). Two patches to `zune-jpeg` — an overflow in its DC prediction, and a scan whose
data holds every minimum coded unit with no `EOI` after it, decoded whole rather than stopped a row
short (ADR 1520) — are carried by the owner's fork `close2/zune-image`, which the manifest pins in
place of 0.5.15 (`doc/questions/A227`), and both reproducers are regression tests (ADRs 1589,
1730). A JBIG2
symbol dictionary the codec decodes for minutes is ended at a deadline on either isolation — the
in-process one abandons rather than kills, and says so, and a process behind its own seccomp filter
decodes on its own thread instead, because a kept thread asks for `prctl` — and its three unbounded
loops are bounded in the codec itself: the manifest takes the JBIG2, JPEG 2000 and CCITT codecs from
the owner's fork `close2/hayro`, upstream's 0.3.1 commit with three T.88 commits on it (ADRs 1447,
2102), which ends in under a second every input the campaigns have left past the deadline (ADR
1746). The filter is held to ITU-T T.88 Annex K's conformance data, read from outside the tree:
T.88's own Annex H datastream and Annex K's fax pages decode bit for bit, the streams that depart
from the clauses they exercise are held by name with the clause, and a generic region on the
extended template decodes, Annex K's to its reference (ADRs 1459, 2102). Annex H.2's
arithmetic-coder sequence holds the codec's decoder register for register in the fork's own unit
test (ADR 1485).

### What a person does with an open document

It is **used**, which is a separate claim from the one above — and
the first sentence of it is **measured** rather than asserted: a gate drags across `pdftotext`'s own word boxes on every corpus document and asks
what came back (ADR 0421). What it reads back is the page's words where the page shows
them, because the pen's step between two show operations is taken back into text space — through
`Tm`'s linear part and out of the horizontal scaling — before it is compared with the space it would
take to be a word gap, so a mirrored or scaled text matrix breaks no word, and a font that states no
space is given a nominal quarter em as one (ADR 1490); a gap reads as a word break once it is more
than half that space (ADR 1502), or less where a font's own steps on the page show a wider empty
interval below it, settled when the page is finished (ADR 1515). A
locked document asks for its password (§7.6.4.1) — **in a window of the host's own in all four**, with no terminal needed (ADR 0545); the page zooms and scrolls; the cursor knows
what it is over and §12.5.5's appearances follow it, as does §12.5.6.19's `/H`; a drag **selects
text**, whose shapes cross to the host as geometry so that it draws them in its own colour, **and
that text can leave the program** — every one of the four consumers puts it on the platform's own
clipboard, in §14.8.2.5's logical content order — a line break at each block's end, a tab between
table cells, declared artifacts left out (ADR 1948), an annotation's text where the tree's object
reference names it (ADR 1970), and a soft hyphen's line division undone in both orders with the
character kept (ADR 1971) — where every byte of the selection is reached by
the structure tree or declared an artifact, and in page content order otherwise, said out loud either way (ADR 0519) —
with **§14.8.2.2.2 taken at its word, that content the structure tree does not include is an
artifact even where nothing tagged it one**, so what a reader takes as text can have the page
furniture subtracted from it and the file's own declaration stays distinguishable from this
reader's inference (ADR 1100) — and §14.7.5.4's parent tree is read as the standard writes it,
its array an indirect object of its own where the producer made it one, and a widget the tree does
not key is still spoken by asking §14.7.5.3's `/OBJR` (ADR 1151);
`/`
**searches the whole document**, one page read per turn of the host's event loop because a
thousand pages of interpretation is not something the launch path may block for, with the readback
kept under a per-document bound so that searching the same document twice does not cost twice
(ADRs 0250, 0256); **a form's data can arrive and leave as a file**: §12.7.7's FDF and §12.7.8's XFDF both read into
the same fully qualified names a field carries, so an import means one thing whichever format came,
and an FDF carrying Table 246's `/EmbeddedFDFs` carries FDF *files*, each read as one and applied
in the array's order, with only the encrypted form refused because only that is deprecated
(ADR 1185). **An FDF file's annotations are placed on the pages Table 254's ordinals name**, and
Table 249's `/AP`, `/A`, `/AA` and `/IF` replace the widget's own: a value that lives in the other file
crosses as a *value*, copied rather than referred to, so the interpreter still holds one document
and the imported appearance goes back into §7.5.6's update as the FDF producer's own marks (ADRs
1223, 1224); and Table 249's **`/APRef` naming a page of the target document** becomes that widget's
appearance, because Table 253 makes its `/F` optional and its absence puts the page in the file
being read — §12.7.7's own second purpose for naming a page, "either as a page or as a button
appearance" — so the page's content stream, its inherited resources, its crop box as the `/BBox` and
its `/Rotate` as the `/Matrix` are the Table 93 form §12.5.5 places; and a reference that *does*
name a second file is **asked of a host and applied when the file arrives**, which is a second
question raised while the import is being applied and which Table 252's `/TRef` travels on too — the
page becomes the button's appearance or a page of the document, copied so that it names nothing of
the file it came from, under the reader's `--remote-documents=` level (ADRs 1235, 1239) —
XFDF's fields against ISO 19444-1, which ISO 32000-2 names and defines nowhere, and its
**`<annots>` from the Adobe text that standard was made from**, every annotation element spelled
into the dictionary §12.5.6's tables define and placed by the same import as an FDF file's, with
Table 172's `/Popup`, `/Parent` and `/IRT` written as references to the objects the import makes
and every place ISO 32000-2 overrides the text, or the text says too little, refused by name (ADRs
1108, 1297); a person can **fill
in a form field** — where the host keeps the *point* it
clicked and never the text, so §12.7.5.3's truncation is read back rather than predicted (ADR
0201), with a caret that says where the next character goes so that correcting the middle of a
value is not deleting back to it (ADR 0211) — undo it and redo it; **a field's one-call script
runs** — RFC 0008's Tier 0: a keystroke, format, validate or calculate script that is textually one
call of Adobe's `AF*` form library with literal arguments runs as `pdf_model::aform`'s Rust
re-implementation of all twenty-one functions, every rule of which ADR 1578 names as a choice, a
keystroke the script refuses is not taken, a total follows its lines because Table 224's `/CO` is
walked in order after every value change, and a value is drawn and saved through its format once it
is not being typed — committed in all four windows when a tab, a press elsewhere or Enter in a
single-line field takes a person out of it, with a value the commit refuses put back and a character
the keystroke refuses left out, each said in the window's own way (ADR 1592), and in the two
toolkit windows a control shows the committed value as the field displays it — `$12.50` — and the
field's characters while it holds the keyboard (ADR 1604) — while every other script is reported as one this tier does not run and no
ECMAScript is parsed (ADR 1579) — RFC 0008's Tier 1 engine, `pdf-script`, is linked into the
worker program alone and runs in every window **at the reader's `Scripts` level**: `--scripts
off|ask|warn|on` on every command line and a third act in every menu, `off` by default and
`quorra-confined` pinned there and saying so, *ask* putting one question per document at its first
script with that script's first line and a `yes` running what was withheld (ADR 1616), and `CLAUDE.md`'s exclusion beginning only where a script would reach beyond this document and this viewer (`doc/questions/A286`, ADR 1904); where it runs a document's scripts share one realm — Table 32's
name tree runs into it once the first page is presented, less an entry a checked requirement's `/RH` names as its handler, which §12.11.5 disables and the open sequence says (ADR 1923), Table 198's `/O` of page one and its
annotations' Table 197 `/PO` and `/PV` run in the open sequence, and every later Table 197 and
Table 198 script is a request the view state exposes for a host to raise at its event — a widget's
pointer and focus through `run_annotation_scripts`, the page shown changing through
`run_page_scripts` with Table 197's four page events beside it, nothing before the open sequence
(ADR 1750), raised in all three windows, a press a toolkit's own control took included (ADR
1752), and an outline item's `/A` handed over as the item is activated, at Adobe's `Bookmark`
event (ADR 1904), with `this.bookmarkRoot` and the reference's `Bookmark` answering from the outline a person sees, read on first use, `execute` activating the item as a click does and every write refused as the document's structure (ADRs 1922, 1934) — a commit runs `/V` and every `/CO` entry's `/C` with
`event.rc` honoured and draws the runner's `/F`, a script's writes land in the edit log and the
colours, border style, alignment and limit it sets on a field are drawn into the widget's
appearance and saved as `/MK`, `/BS`, `/DA`, `/Q`, `/MaxLen` and `/Ff` (ADRs 1602, 1603, 1617),
and Table 200's five run at the close, the save and the print a host marks, a script's
`event.rc` reported and never obeyed (ADR 1614); a question a runner's script waits on —
`app.alert`, `app.response` — goes to the window as an event after the command and comes back as a
command, put on `quorra`'s card and on GTK's and Qt's dialogues titled with the document that asks,
with the script's buttons or an entry holding its default, the window's thread never waiting
while the script is held in its worker and what it does once answered applied late (ADRs 1627,
1628), an answer only ever the question's the window took (ADR 1641), and a script's write to
`this.pageNum` a page turn the window makes (ADR 1640), `app.setInterval` and `app.setTimeOut`
counted down in the ticks every window sends only while a document holds one, a due tick waiting
in `quorra` for the frame the last one asked for (ADR 1957), a script the view state does not run
said once and with its own reason at every event a host raises (ADRs 1950, 1975), and a press on an
annotation `/Annots` holds as a dictionary or on a widget no form makes a field saying the sentence
the page's open says, at every level (ADR 1970 §3), a member the surface refuses a function that
throws when called where Adobe's reference makes it a method and a property that throws when read
where it makes it one, so a script that asks `typeof` first reads the surface (ADR 1957 §2), the global `xfa`
absent rather than refused, because the form a script acts on here is the Acrobat form (ADR 1989), and `app.beep` a
sound GTK and Qt play and `quorra` refuses by name (ADR 1702), `getField("name.N")` one widget of a
field whose widget members are its own and whose value is the field's (ADR 1664),
`Field.style` a check box's glyph redrawn (ADR 1665), and `this.getAnnots` the document's markup
annotations, filtered by Table 167 as the page is drawn, whose `hidden`, `popupOpen` and free-text
or text-note `contents` a script sets as a reader's edit and a save writes, a window's state the
later of a click and a script's write (ADRs 1700, 1720, 1721), the pages' labels, boxes and
rotations, `title`, `calculate` and `app.activeDocs` read from the view state (ADR 1724), a named
destination carried out as its page and its Table 149 view in one view change (ADR 1751), the window's `zoom`, `zoomType` and `layout` told to the realm and a script's
write to them or its `scroll` carried out as a person's (ADR 1736), and a choice field's items read
from `/Opt`, chosen by index as a person chooses (ADR 1725) and rewritten as one property the page,
a host's control and the save read (ADR 1737) — the runner every window supplies is `pdf_script_worker::ScriptWorker`, which
runs each script in a third confined process started at the first trigger, under the narrowest of
`pdf_sandbox`'s three profiles, a deadline per trigger and a named loss (ADRs 1608, 1609), a script's
nesting bounded before it is parsed (ADR 1626), the worker built with the engine in a Cargo run of its own and installed beside the windows (ADR 1625); **a rich text field's value and a free text
note's `/RC` are drawn in their formatting**, because Table 228's `/RV` and `/DS`, Table 177's `/RC`
and `/DS` and Table 231 bit 26 are ISO 32000-2's own rich text string and not the XFA template
architecture `CLAUDE.md` excludes: XFA 3.3's Rich Text Reference is held and each run is set in the
face, size, width, colour, alignment, spacing, tab stops and leaders it states — and, where it asks
for pair kerning, kerned by the pairs a face the document embeds states, under the script its
characters select and the language system its annotation's language names (ADRs 1682, 1696,
1708), or for §9.6.2.2's fourteen by the pairs Adobe's Core 14 AFM files publish (ADR 1916) — with every list
type chapter 27 requires — in a comb's cells, in UAX #9's order, a tab moving to the stop on the
left where its paragraph reads that way, a character no run's face draws set in a machine's face
beside the runs, with a host's caret, point and range answered from the runs — a changed rich value
regenerating the whole appearance, a save writing `/RV` beside `/V` and an import carrying Table
249's `/RV` and XFDF's `<value-richtext>`, which a save writes (ADRs 1197, 1634, 1635, 1648, 1649,
1660, 1661); **a file-select control takes a
file rather than a value**, because Table 231 bit 21 makes the field's text "the pathname of a file
whose contents shall be submitted as the field's value" and only a host has a filesystem to read
them from — the path a *person* typed, under one policy function with a stated memory budget, which
is not the path a *document* wrote (ADR 1216), and which GTK and Qt offer a **native chooser**
for, inside the widget's own §12.5.2 rectangle and behind the same policy function
`viewer_host::form::edit_of` asks (ADR 1240); a person can **send a form** — §12.7.6.2's composed
request leaves from the host with `ureq` over `rustls`, never from the confined worker, at a level
the restriction menu holds (`ask` until a person picks `refuse`, `warn` or `send`), and an FDF
answer is imported into the form that sent it with its `/Status` shown while a PDF answer opens
beside (ADRs 1291, 1292), and Annex O's `fdf` naming a server is fetched by the same client at
the same level and imported into the document that asked (ADR 1527) — in `quorra-confined` too,
which sends both from its side under `--submissions=`, asks on its card, hands the answer to its
worker as `Command::Respond` and keeps a PDF answer rather than opening it (ADR 2076); a person can **choose an option
in §12.7.5.4's two controls in all three windows**, which is Table 233 bit 19 obeyed in both of the
directions it states rather than in the one that reads as a permission: the flag set is an editable
text box beside a drop-down list — composed in GTK4, which has no widget that is both — and the flag
clear is a drop-down and no way to type into it (ADR 0596); a **push button keeps its own appearance in all three windows** — the
producer's `/AP`, or the caption §12.5.6.19's `/MK` constructs — because GTK and Qt place no toolkit
button over it, and a click, or Space or Enter at §12.5.1's focus, performs its `/A`, the Qt page
declining the toolkit's own focus chain so that Tab walks the document's `/Tabs` (ADR 1357); a
click on a markup annotation
**opens the window §12.5.6.14 gives it**, which is the second half of §12.5.1's sentence about
activation (ADR 0191), and a text note that states no popup opens one of its own beside its icon,
§12.5.6.4's (ADR 1723), where a press gives a person the note's text to retype (ADR 1726), at the
place pressed in every window (ADRs 1739, 1770, 1782) and in the runs' faces where the window draws
them, and a
reply stating no popup is a threaded comment (ADR 1727) — **in all three windows**: the clause gives a popup "no appearance stream", so the window is
furniture rather than ink and each host places its own, over one reading of the two clauses that say
what goes in it (ADR 0613), drawn in all three as opaque paper with an edge (ADR 1466) — **with the subject and the creation date beside the title and the
text**, and the two dates kept apart, because Table 172 states when an annotation was made and
Table 166 when it was last changed (ADR 1224), and **Table 172's `/RC` drawn formatted** run by run — Pango spans, a `QTextDocument` built run by run, `quorra`'s chrome, each run in its own face and a right-to-left paragraph in one order across its runs, its tab stops set, leftward in a paragraph read right to left, with their leaders (ADRs 1679, 1690, 1722), and a list tag at its start edge, the font scales in two of the three — with what a window did not draw said under the note (ADRs 1642, 1654, 1666), and handed to a C caller as runs, stops and leaders (ADRs 1655, 1667, 1726); a **cursor changes over §12.5.6.5's activation region** in all three,
which no clause states and which is therefore recorded as this program's convention; a person can
**measure a drawing** — §12.9's viewports, traced by pointer in all four windows on the key `m`
and over the C ABI's `quorra_measure`, with the arithmetic and
§12.9.2's five formatting steps the document's, so a length, an area, an angle and a slope come back
in the units and the labels the producer chose rather than in any this program invented; a geospatial
viewport says which system the map is in and where the point put down is, to six decimal places of a
degree with its hemispheres — read through the affine map its registration points determine, with
how far they depart from it, which is a choice because §12.10 defines no function between them
(ADRs 1191, 1593) — and `pdf_model::geospatial` evaluates each method the census found in both
directions, so a projected position comes back as a latitude, a projected `/DCS` is displayed as an
easting and a northing, a `/DCS` on another datum is reached through the Helmert transformation the
WKT states — the older form's `TOWGS84`, or ISO 19162's bound CRS — and refused by both datums' names
where it states none (ADR 1958), and a
position given to a viewport answers its page point; a projected map
whose registration points are its base system's degrees, as every one the census found is, is
fitted in the system's plane through the forward projection, a departure from Table 269's sentence
(ADRs 1586, 1587, 1672, 1918); a person can **add an annotation** — §12.5.6.10's four markups over what is
selected (ADR 0196), and §12.5.6.6's free text drawn as a rectangle and typed into, which is the
one markup subtype whose text *is* the annotation and therefore the one whose geometry has to come
from a drag rather than from a selection (ADR 0238) — **and the producer's own free text annotation
can be retyped**, which is §7.5.6's second case rather than a second kind of writing, with Table
167's `LockedContents` asked as a policy and its `Locked` deliberately not, on the table's own
sentence (ADR 0304); a person can **put a file into the document and take one out** — §7.11.4's
embedded file, in either of §7.11.4.1's two homes: the name tree, or §12.5.6.15's annotation on
the page under the point it was dropped on, which draws its icon before anything is saved (ADR
0814) — with §7.9.6's one namespace over both homes, an undo that forgets it and a list that
answers the log rather than the file; and the result can be **saved** — the file it
was opened from, unchanged, with §7.5.6's incremental update appended, which is the one kind of
writing `CLAUDE.md` permits. Where a document states `/NeedAppearances`, the save names every widget
whose appearance this program did not construct, by §12.7.4.2's fully qualified name, rather than
leaving the reader to find out from the file (ADR 1159). Every annotation that update writes carries Table 166's `/M` where a
host has said what time it is, and none where none has: the renderer has no clock, so the instant
arrives as `Command::Clock` the way every other fact about the reader's machine does (ADR 1160).
**No window has a gesture for the attach**, by the owner's word
that the flows are being reviewed as mockups first; the C ABI has `quorra_attach` and `quorra_detach`,
because an ABI has no gestures.

### A document's restrictions, in every window's menu

**What a document asserts over its reader is the reader's to set, in a menu every window has.**
`CLAUDE.md`'s four levels — off, on, ask, warn — stand one per operation and in two scopes: the
window's, which every document it opens inherits, and one document's departure from them, which
ends when that document closes. One of those operations is not a verb a person presses: §12.11.6's
requirements processing, where a document whose unmet requirements pass §12.11.3's penalty
threshold is refused, asked about or warned of *before it is opened at all* — `on` raising no
`Event::Opened`, which is what the clause's "the processing of the document shall not continue"
is, and `off` being the default (ADR 1167). The *ask* level puts its question on a modal window in
all three, worded once in `viewer_host::restriction`; §12.2's `/HideMenubar` is read, answered in words and
deliberately not obeyed over that menu, because a file that could hide the reader's levels would be
taking away the control over itself (ADR 1145).

### Printing

**A page goes to paper, and what a printed page shows is not what a screen shows.** §7.6.4.2's
Table 22 bit 3 is asked as an operation at those same four levels, and the grant puts the document
into print intent: §12.5.3's Table 167 bit 3 decides which annotations are drawn — "[i]f clear,
never print the annotation, regardless of whether it is rendered on the screen", with `NoView` not
consulted at all because its own row says the annotation may be printed anyway — §8.11.4.5's
`Print` usage applications run over the reader's own layer switches for the duration and revert
after, and §12.5.6.22's fixed print watermarks are placed against the sheet rather than against the
media box. The window shows the same thing while the operation stands, so what a reader sees is
what would print. **Two windows print.** `quorra-gtk` spools through `GtkPrintOperation` and
`quorra-qt` through a `QPrintDialog` and a `QPrinter` its own C++ names, each painting the page the
processor backend drew — the same backend the oracle certifies — into the toolkit's context; the
winit host has no toolkit dialogue at all and shows what would print, saying so (ADRs 1179, 1180,
1203). The resolution is the printer's, clamped to 150–600 dots per inch with 300 where none is
reported, and §12.2's half of Table 147 — scaling, duplex, tray, page range, copies — is what a
dialogue opens on. **And the two entries of it that decide pixels decide them**: a page carries
§14.11.2's boundaries twice, `Page::print_box` and `Page::print_clip_box` beside `display_box` and
`clip_box`, and `Page::render_for_printing` selects between the pairs where the print operation
stated `Purpose::Print` — so a document naming `/PrintArea /MediaBox` lays a wider square onto paper
than it shows on the screen (ADR 1227). **Before the grant, a print through §14.11.5's output intent
says what it does not do with it**: the device the page was drawn through is named, and Table 401's
output transformation into that device's colourants is not performed, because the page goes to the
print system as the RGB the screen shows (ADR 1932). An XFA form is said on the document's report,
once, as §K.1 requires, with what of it is carried out; a form whose template generates its pages is
not laid out, as §K.1 permits (ADR 2430). **A
hybrid form saved here keeps its `datasets` packet consistent with the `/V` entries the save
writes**, §K.2's first writer rule: the data value each changed field binds is spliced into the
packet and every other byte is the producer's, and a field whose data it cannot write is reported
by name (ADR 2396). **A numeric, date or time field's data is its canonical form**, parsed from
what was typed by its own picture clause in the form's own locales (XFA 3.3 chapters 4, 24 and 26;
ADR 2414), and **written out by its `bind` picture, where its `validate` picture's format test
passes it** — a value the test fails is reported by name (ADR 2434) — or by the default bind
picture a `config` packet's `transform` gives its data element (chapter 15; ADR 2451). **Where the
form asks for its validations before a save, the save runs its null tests** and reports each field
left empty in the form's own message, saving it all the same, as XFA 3.3 page 1340 has it (ADR 2450).

**Table 22's bit 12 is a second operation beside bit 3, at four levels of its own**, because the
cell states two consequences: bit 3 clear withholds printing, and bit 12 clear limits a print that
goes ahead — "printing shall be limited to a low- level representation of the appearance". The
implementation-dependent algorithm the cell hands to a processor is chosen: a degraded job is drawn
at the lowest resolution this program draws at, and a destination that writes a document is refused
by name, because a file is a document a faithful copy could be generated from whatever it was drawn
at. The *ask* level over it is the one held question whose `no` starts something — a degraded
print rather than none.

**A page placed on a sheet is where §12.5.6.22's matrix B stops being the identity.** A page shrunk
to fit the paper, or set two or four to a sheet, states what it was scaled by and which portion of
the paper it landed in, and a fixed print watermark is drawn immune to the first and measured
against the second — the clause's own two post-EXAMPLE bullets, "at the specified size" and
"positioned as if the dimensions of the printed page were limited to a single portion of the page".
The scale mode and the pages per sheet are on a tab of `quorra-qt`'s print dialogue, because those
two decide where the marks land and every other setting in a print dialogue does not (ADR 1204).

### What a document may ask of this machine

**And what a document may ask this machine to do is the reader's to set too, at the same four
levels running the other way.** §12.6.4.8's link is opened by the host and by nothing inside the
confinement, under `--links=refuse|ask|warn|open` in all four windows — `ask` by default, so
nothing reaches another program without a keypress, and only `http`, `https` and `mailto` are
handed over at all, whatever the level, because a document free to name a scheme would be choosing
which of this machine's handlers runs. The words are not the restriction menu's four, because there *off*
is the permissive end and here it would be `open` (ADR 1155). **§12.6.4.3's remote go-to is a second such value and not the
same one**, `--remote-documents=refuse|ask|warn|open`, `ask` by default: a document that names
another file is asking this reader to open a PDF in place of the one being read rather than to
start another program, and the file is looked for beside the open document and nowhere else at
every level, including the permissive one; `quorra-confined` asks both on the flagship's card,
answered by Enter and Escape, and opens a named file in place of the one document it holds (ADR
2040) — Table 203's `/D` and `/SD` are then read in the
document that came back, because §12.3.2.2 makes an explicit destination's first element a page
number *there*. **A request for a new window opens a second document beside the first**: Table 203's
and Table 204's `/NewWindow true` are read in the core and answered where the host's answer is,
because whether this program has a second place to put a document is a fact about the window —
`Command::Beside` carries a name a host has free for one, and a window that offers none gets the
sentence saying the destination replaced what was open (ADRs 1227, 1263). **§12.6.4.6's launch action
is the same act where its Table 207 `/F` names a PDF**: the file is asked for under the same level and
path rule, its §7.5.2 header decides what it is once it has arrived, a PDF opens where `/NewWindow`
puts it, a program is started never, and any other file is a document handed to this machine's own
handler at `--launched-documents=`'s level, `ask` by default, since every window's restriction
menu sets it (ADRs 1368, 1358, 2171, 2185). The path
rule admits §7.11.2.2's relative specification into a directory below the document's own and
refuses `..`, `.`, an empty component and an absolute path (ADR 1369). A file the go-to names
that asks for a password is asked about under the name it would open under, and the second attempt
opens that file's own bytes, never the document in front (ADR 1332); §12.6.4.7's thread in another
file and §12.7.8's named page in one take the same route, the named page drawn into the document
that asked for it. The question says where the file would open, beside or in place, and a
replacement relabels its tab and moves the window's path whether or not a password was asked
(ADR 1335). A prompt a person cancels declines the file with the reason — a jump says so, and a
named page's references are let go, each widget keeping its own appearance (ADR 1345).
**§O.2.1's `ef` is a third
such value**, `--embedded-documents=refuse|ask|warn|open`, `ask` by default, on the restriction
menu's third group beside sending a form (ADR 1331). **Every one of Annex O's open parameters is
carried out** — the eleven parameters Tables Annex O.3 and O.4 print, `fdf` naming an FDF or an XFDF file — and
a fragment that names one the standard does not define says so while the rest of it runs; a `zoom`
beyond this reader's range lands on the bound and is named (ADR 1523). `tools/state.sh annex-o`
counts them.

### Several documents in a window, and a measurement drawn

**Three windows hold more than one document, in a strip of tabs apiece.** A `gtk4::Notebook`, a
`QTabWidget` and a strip `viewer-ui` draws for itself, with `viewer_host::Documents` as the
bookkeeping the three share — which names are open, what closing one does to the front, and where
a window's per-document state goes while the person is reading the other. Ctrl + Tab moves and
Ctrl + W closes; closing the last one closes the window; the strip hides itself for one document,
so a window that opened one file is the window it was. What travels with a tab is what is about the
*file* — the path, the caption, whether anything is unsaved, §7.6.4.1's attempts, Table 29's
arrangement, §12.9's points, and the document's own departure from the window's restriction levels
— and what stays is what is about the *window*. **A person opens one too**: Ctrl + O is a
`gtk4::FileDialog`, a `QFileDialog` and a line `quorra` draws for a typed path, and every path
after the first on a command line opens as a tab behind it once page one is on the screen — all of
them through `viewer_host::open_chosen` and one at a time, and each under `Command::Open`, so every
answer the reader gave reaches the second document as it did the first. A tab says §14.3.3's
`/Title` where the document states one and otherwise the file's name, and `quorra` sets a character
its compiled-in faces lack from a face the machine offers rather than as a box (ADR 1382), searched
for on a thread of its own so that no catalogue walk is on the launch path, while `quorra-qt` hands
Qt the same file for its tabs and panels (ADRs 1406, 1418); an Arabic or Hebrew label is joined and
read in its own direction in all three windows (ADR 1417), a word of it set in one machine face at
the requested weight and a wrapped line of it read in its paragraph's direction (ADR 1430).
`quorra-confined` holds one
document, on ADR 1190's rule. **A window is the front document's**: one opened behind obeys its
`/PageMode` when it first comes to the front, so a presentation running when a second document
arrives keeps its full screen and a `UseThumbs` document opens its pages panel then; full screen
hides the strip; and a transition's faces are drawn on the window's own surround from the front
document's page (ADR 1303). ADRs 1263, 1264, 1275. **A presentation is the front document's and is
parked, not ended**: in every window each tab keeps its own full screen and clock while another is
in front, and a hidden window's presentation counts no time, because a page nobody can see is not
displayed — one pump, `viewer_host::presentation_pump`, which also gives the WinUI host its `/Dur`,
its transitions and its script timers (ADR 2300), and in `quorra` its own winit turn parked on the
window's occlusion and minimised state (ADR 2317).

**And §12.9's measurement is drawn as well as said.** The traced path is over the page in all three
windows, each press marked, in each platform's own colour — the points are the host's and
`Query::Measure` answers what they mean, so it takes no message of its own (ADRs 1191, 1264).

### Separations, transfer functions and overprinting

**§10.8.3's separation simulation has a control, which is the one thing the clause conditioned
itself on.** The simulation is for when the colours of a display matter "on a device that normally
would not be used to produce separations", and §10.8.1 says whose choice that is; so
`--separations=on|off` and a key in all three windows, `Command::Separations` across the confined
wire, `quorra_separations` for a C caller, and `ViewState::separation_simulation` where it reaches
the colour route. A preference rather than one of the four levels, because nothing in any file asks
for it and there is nobody to ask on the reader's behalf (ADR 1228).

**§14.11.2.2's guidelines for the page boundaries are offered on the same shape**: `b` in every
window, `Command::Guidelines` across the confined wire, and `ViewState::guidelines` where the
interpreter draws a stroke over each crop, bleed, trim and art box the page defines, in the style
its `/BoxColorInfo` states or this program's own default — on the screen only, and not over a page
composited in a blending space of its own, which is said (ADR 2440).

**And under that control §10.8.3's four steps are performed, which is what §8.6.6.5's per-component
sentence needed.** A `DeviceN` space whose `/Subtype` is `NChannel` and which names a spot colourant
is read as separations rather than as one tint transform: each spot colourant through the
`Separation` colour space Table 70's `/Colorants` holds for it, the process components together
through Table 71's process space, and the results converted to flat XYZ against a white matte and
multiply-blended in that matte's own white. Every other space is the same space under both answers —
a `Separation` is one separation and a product of one term, and a `DeviceN` of process components
alone is what its process space already says — so the preference moves only the spaces the clause's
sentence is about. Off by default (ADR 1229);
a requirement executed under a control a host supplies is executed, which is the owner's ruling in
`doc/questions/A100`, so §8.6.6.5 is `implemented` with its default argued in its row.

**Under the same control a page naming a spot colourant is drawn as the press would print it.**
§10.8.3's step a) processes the page "as if separations were to be created for a simulated device
that supports subtractive process colourants and possibly spot colours", so the page is interpreted
once per plane of that device — the two process planes and one for every three spot colourants,
sixteen at most — on its own four-component press or the intent's, and every colour is resolved by
§11.7.3: a colourant with a plane is painted directly on it, every component a mark does not name is
no ink, `All` paints every plane, groups pass the spot planes through and soft masks carry none
(ADR 1311). The planes are the page's display list, and the CPU and `raster` backends put them
together by steps b) to d): each plane over the white matte, to flat XYZ through the press or the
colourant's own separation, multiplied, and converted to the screen — so LogoGreen overprinting
yellow comes out the product of the two inks rather than whichever was painted last. A spot plane
composites under Normal where §11.7.4.2 forbids the mode, the GPU backend refuses the page by name,
and an ink past the sixteen planes is named on the page's report (ADR 1317). A page whose group composites in one
or three components is separated the same way, its spot inks passing through the group untouched and
its process colours composited in the group's own space (ADR 1329). §10.3.2's one sentence that
sends an ICC enabled processor back to a classic method — grey into a *native* CMYK space by
§10.4.2.3 — is inapplicable on its own condition, because no output of this program is a CMYK
device: the screen is RGB, print hands over an RGB raster, and `archive` writes CIE definitions
(ADR 1437).

**§10.5's transfer function reaches the screen, and it is applied where §11.7.5.2 says.** The
clause chooses the function at a pixel by the topmost object whose shape there is nonzero, so the
mapping is applied once after compositing rather than per mark: the interpreter records every
elementary mark's shape and function as runs on the display list, in painting order, and both
backends resolve the runs top down over their own read-back, so the rule is stated once and the two
agree by construction; a page stating no function records nothing. **No colour anywhere in this
tree carries a transfer function**: a shading's ramp is sampled raw and rides its function on the
mark, and a tiling's function is the one in force at the mark that paints the pattern, which the
finished tiling carries as one run (ADR 1125, ADR 1266). Every mark states the clause's shape to
the channel, a stencil under a soft mask of its own included, and a mark whose overprinting keeps a
backdrop component takes the page's default, so §11.7.5.2 is `implemented` with nothing reported
(ADR 1279). **Colour is its own crate**: `pdf-colour` holds the colour spaces, the ICC
reader, the functions, shadings, meshes and transfer, below the interpreter with no cycle and
re-exported by `pdf-model` under the paths its callers knew (ADR 1131). **All four of §7.10's
function types are evaluated**, a Type 0 function's samples encoded from its own `/Domain` and its
`/Order 3` taken as the not-a-knot cubic spline, the choice ADR 1636 writes down. **§8.6.5.9's black point
compensation is performed rather than reported**, on the `ON` case the clause states by reference:
ISO 18619's procedure, whose source black is read from an output-capable profile's own perceptual
`B2A` where it carries one and taken as the display's `L*` 0 where it does not, so a press's deepest
ink converts to the destination's black point (ADR 1253). `/UseBlackPtComp` joins
Table 69's four intents in selecting the conversion, which is why `colour::MAX_PRESSES` is stated
against the standard's own floor of seven pairs per profile rather than against a round number
(ADR 1254). A press table a panicked thread poisoned is `NoPress::Poisoned`, carried to the caller as the
program's fault, never a route that cannot be built or a count of zero (ADR 2186).

**A page paints under §11.7.4's overprinting, and it is not a seventeenth blend mode.** Table 57's
`/OP`, `/op` and `/OPM` are read, §8.6.7's zero test is made before quantisation, and §11.7.4.3's
special mode is two Porter-Duff operators chosen per channel — so `render-cpu` computes it and
`DisplayList::overprints` carries it, with §11.7.4.3's and §11.7.4.4's implicit groups established
where those clauses say (ADRs 1157, 1158, 1169, 1170, 1182). `render-raster` draws the mode too,
through raster's `Compose::DestOver` and `Compose::DestOverIn`, which were built in `raster/` from
the clause alone so that the cross-backend run is the two readings' first meeting (ADR 1295);
`render-gpu` refuses a display list carrying it by name. ADRs 1178, 1181 and 1241 hold the census
of the crawl that paints under it. A `Separation` or `DeviceN` that
reverts to a `DeviceCMYK` alternate is in the first bullet on the components its alternate
receives, which is §11.7.4.3's NOTE 2 and the equivalence §8.6.7's own EXAMPLE states (ADR 1241).
An `ICCBased` CMYK colour on a press sampled from its own profile is in the first bullet too: this
reader passes its values through, and §8.6.5.7 attaches the device space's semantics, the mode
first, to that conversion — which also gives a `k` its mode on a page whose output intent is the
press (ADR 1996). Inside an isolated group or page whose blending space is an `ICCBased` CMYK one,
`DeviceCMYK` *is* that space, as §11.7.2 redefines it — after §8.6.5.6's default and before the
output intent — so a `k` there is the group's colour and keeps its mode under any intent (ADR 2012).
A pair that is a direct element of a knockout group takes the implicit group too, with §11.4.6's
NOTE 6 deciding which of that group's two initial backdrops it composites onto rather than refusing
the position (ADR 1265). **§11.4.6's knockout groups are drawn by `render-cpu`, isolated or
not, and under a blend mode of the group's own too**, taking §11.4.4's result step
with Table 140's group alpha kept beside the accumulation (ADR 1305); every element states its
shape on every route, a stencil painted through a pattern included, and §11.6.4.3's reading of
`/AIS` is kept by the scope that paints under it — a tiling cell, a text object, each glyph pair —
restored by `Q`, and read element by element where one scope paints under both (ADRs 1301, 1306,
1319). A tiling cell starts from the graphics state its parent stream began with (ADR 1320). A knockout
element that is a fill or a stroke is drawn in the clause's two stages, so its fractional shape is the
area `crate::area` measures rather than the library's lattice of sixteenths (ADR 2286). The
two graphics backends refuse a non-isolated knockout group by name, because each of its elements
composites with the group's own initial backdrop. **And where the file states a
black generation, it replaces the device's default**: Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2`
reach both routes into a subtractive space — the graphics state's and §11.6.7's pattern dictionary's
— so §10.4.2.4's conversion runs the producer's functions instead of this device's, `/BG2 /Default`
puts the default back on Table 57's own precedence, and a function this tree cannot evaluate is
`Unsupported::BlackGeneration` rather than a silent substitution (ADR 1207). The pair in force at a
`Do` reaches the conversion of a group's result into a four-component parent as well, which is
§11.7.5.3's second bullet and is sampled into the cube the backends already take (ADR 1242). **And the
pair is the one in force when the mark paints**, which is §11.7.5.3's first bullet: an image takes
the pair into its decoder's conversion beside its own rendering intent, and the departure ADR 1242
keeps — a `DeviceCMYK` target the output intent characterises converting through its profile — is
reported at a paint that bypasses the pair, never at a `gs` that only sets one (ADR 2187). A colour
is therefore kept as its source until it paints and converted under the context in force then: a
solid `DeviceRGB` colour's components (ADR 2198), an uncoloured tiling pattern's tint through every
nested target (ADR 2212), a direct `ICCBased` colour's profile and components, so an `ri`, `RI` or
`/UseBlackPtComp` set after the colour still decides its conversion (ADR 2222), and an `Indexed`
colour over an immediate `ICCBased` base as its decoded entry (ADR 2234), and a `Separation`,
`DeviceN`, calibrated or `Lab` colour as its resolved space and operands (ADR 2248), each restored
by `Q` and across a Form's resources (ADRs 2208, 2221). So is an `Indexed` entry over any base and a
plain `DeviceGray` or `DeviceCMYK` colour (ADR 2264), and `DeviceCMYK` itself is read where a colour
paints: a `k` the output intent resolved on the page is an `ICCBased` CMYK group's own components
inside it, §11.7.2's redefinition (ADR 2276). §11.7.5.3's row is `departed` on the one target ADR
1242 keeps.

### A mesh patch on the backend

**A mesh patch travels to the backend rather than being tessellated in the model**, because the
fineness a patch needs is a number of device pixels and `pdf-model` has no device: `pdf_render::
SurfacePatch` carries §8.7.4.5's control net, its corner values and a tolerance, and the backend
derives the subdivision from the net's second differences against half a device pixel and
§10.7.3's colour tolerance (ADR 1217). **And a stencil keeps its shape apart from its mask's opacity**: an image that is
§8.9.6.2's stencil *and* carries §11.6.5.2's soft mask reaches the display list as a `Command::Shaped`, never as one raster holding their
product, whose two halves are §11.6.4.2's
shape and §11.6.4.3's opacity (ADR 1218) — including where the mask is behind an image codec,
which is decoded once per document into a grey plane under the bound that routed it there
(ADR 1232). A stencil painted through a **pattern** under the graphics state's soft mask is drawn
through both, on every backend: the command's mask is the stencil drawn through the state's mask,
and the stencil alone is its shape (ADR 1334). **A clipping path that encloses an area and rules a line admits both**: §10.7.4's
region is the union of two fills under two rules, which the processor composes into its own mask
and the two graphics backends refuse by name rather than admitting a smaller set (ADR 1231).

### Opening a document

**A document is opened on disk and read where its own offsets point** — `startxref` from the
last two kilobytes, each cross-reference section from where the chain names it, each object from
its entry, through a window the parser grows until nothing at its end was examined — so what a
file costs to open is its trailer, its table and page one's objects rather than its length, and a
six-gigabyte document opens in the time a small one does; a damaged file, which a scan reads whole,
costs on disk what it cost in memory (ADR 0809) — and a scan the process cannot hold the file for
is refused by name and said once on the document's report. A `stream` keyword followed by spaces
before its end of line, which §7.3.8.1 does not allow, is read with the spaces skipped unless a
`/Length` fits only with them as data, and the document says which streams it read that way
(ADR 1365). **Every revision in the chain of updates
can be read**: `/Prev` is walked forwards, each section laid over the one before it, so the version
a chain reached is the version its last update states and any earlier revision can be opened by
substituting its table — carrying the file encryption key, which does not change between them
(ADR 1171). **The confined viewer opens the same
way**: the file crosses to its worker as an open descriptor beside `Command::Open`, read behind the
filter through `pread64` and nothing else, so the host holds no byte of it and the six-gigabyte
document opens through the confinement too (ADR 0812). A signature's `/ByteRange` is digested
through 64 KiB windows of the file rather than held whole. **Page one goes to the graphics device** by default, as
`CLAUDE.md`'s startup rules state, so bring-up is on the critical path and a number to keep — and
it is brought up on the platform's primary backends, GL loaded only where they have no hardware adapter (ADR 1532), and
without `wgpu`'s validation of indirect calls, whose two compute pipelines `request_device` would
otherwise compile for a call raster never makes (ADR 1569) — the software Vulkan driver, where a
machine has one installed, costs a share of the adapter check that only a launcher can leave
out, a cost the owner accepted until the tree ships one (ADR 1585, `doc/questions/A270`), and the awake render nodes are opened before the loader
runs, so the power-up an idle GPU starts on an open runs beside it (ADR 1658) — while
page one is interpreted on the document's thread beside it (`Viewer::anticipate`, ADR 1531), so
the first resize goes straight to the render. Nothing on the device's thread waits for warmth:
the warm-up's state has a lock of its own rather than the pipeline store's compile lock, so a
window reading its bring-up as it detaches its presenter is answered at once (ADR 1558). Where a display is named, Mesa's device-selection layer asks its server for the device on every physical-device enumeration, and a launch makes one: the constructor chooses among the adapters the launch's check enumerated (`raster_gpu::LaunchInstance`, ADR 2298). That asking is the display's share of a window's bring-up, which the headless gate does not hold, and the toolkit windows print the device thread's steps on their bring-up line (ADRs 2282, 2283). The other three windows do the same in their own
shape (ADR 1539): `quorra-gtk` and `quorra-qt` open and interpret on a thread while the toolkit
comes up, and `quorra-confined` starts its worker and opens the document in it before the window,
the worker interpreting page one with no viewport. What page one does not need the open does not read: §12.3.3's outline
and §7.7.3's placed page tree, which the caption's section needs, are read on a thread each host
hands them after the join — in the confined window, on the worker's own, inside its confinement —
and the C ABI hands a caller the same handle to run where it likes (`Viewer::preparation`, ADRs
1543, 1553). What each step of that timeline
costs is [`doc/performance.md`](performance.md)'s first section, and the open half is
[todo 42](todo/42-the-launch-path.md).

### Chrome, keys and panels

**And it has chrome, and all three windows have the same six panels**, because the *list* of them is one value every host matches exhaustively on
(`viewer_host::Tab`, ADR 0564), which is what `viewer_host::keys` is for a key press — and a key
press carries `viewer_host::Modifiers`, so **Control means something** rather than falling through
to the unmodified row: the conventional bindings — copy, save, print, find, the next document, close
and open, and Ctrl+Shift+A's archive chooser, which starts on the choice the window already holds (ADR 2001) — are held in one place, `viewer_host::keys`'s
`ctrl_meaning` (ADR 1985), and an unbound Control means
nothing at all, threaded through GTK, the Qt bridge's own signature and winit alike (ADR 1192). A sidebar of
six tabs, drawn in `viewer-ui` with `pdf-font`'s compiled-in Helvetica and a `pdf-render` display
list so that every rasteriser draws it, and in the two native hosts with a `GtkNotebook` of
`GtkListView`s and a `QTabWidget` of `QTreeView`s: §12.3.3's outline, where a click
**activates the item** and the document decides whether that is a jump or a URI; §8.11.4.3's
layers, where a switch turns one on unless Table 99's `/Locked` forbids it — and **§8.11.4.4's
`User` and `Language` categories are answered from an audience a *host* supplies and never read off
this machine**, a category with nobody named staying unanswerable rather than falling to the
clause's default (ADR 1106); §7.11.4's embedded
files, where a click writes the file beside the document — as does a click on §12.5.6.15's
paperclip, because §7.11.4.1 gives an embedded file two homes and a file hung on a *page's* own
annotation is in the one the name tree does not list — and where a document stating §12.3.5's
`/Collection` gets its folder tree and the schema's columns instead of a flat list — in Table 153's
`/Sort` order, which is a `shall` about the rows and is resolved once below the three windows
because the values it orders by are §7.11.6's collection items no host holds (ADR 1168), and in the
view Table 153's `/View` asks for: the details view's columns are the whole visible schema and the
tile view's are its head beside an icon naming the file's kind, every value of the entry obeyed (ADR 1215), and in §12.3.6's own
named layouts, where all seven of
Table 160's are drawn — `FilmStrip` a run of thumbnails indexing the files and the folders with
the selected attachment's fields beside it, `FreeForm` those thumbnails scattered over a surface,
`Linear` one attachment large with the schema's fields and the file specification's beside it, each
thumbnail being that attachment's own first page's §12.3.4 `/Thumb` and its Table 44 kind where the
file states none (ADR 1251) — and in the window region Table 158's `/Direction` `N` asks to be
given to the file list, with its `H`, `V` and `/Position` and Table 157's `/Colors` named out loud
as the furniture this window decides for itself (ADR 1252) — because a
collection is how a document *arranges* its files rather than a new population of them (ADR 0202);
§14.3.3's `/Info` with §14.3.2's XMP under it; §12.3.4's thumbnails, one row per page with the
miniature fitted above §12.4.2's label and **fetched only for the rows about to be drawn**, so that
Table 29's `/PageMode /UseThumbs` opening that tab costs a launch no list (ADR 0564); and §12.4.3's
article threads, followed on a click to Table 163's `/R` rather
than to the page the first bead sits on, because activating one composes §12.6.4.7's own thread
action rather than adding a second route (ADR 0200) — **including a thread Table 209's `/F` puts in
another file**, whose bytes a host supplies under the reader's `--remote-documents=` level and whose
`/Threads` array, title and bead index are read in the document that arrived (ADR 1239). Which
corpus states a thread is `crates/pdf-model/tests/articles.rs`'s to say — not pdf.js's, and
`doc/corpora/`'s does — because which population a claim is about is part of the claim (ADR 0405). `?` puts the tree's `NOTICE` over the page in Courier, **and it does so in all three windows** — the two native hosts ship the same compiled-in standard 14
font programs, whose licences it carries. **What a key means is one
value, `viewer_host::keys`, that all three hosts translate their toolkit's key into**, each with a
test that fails when it stops translating the whole of it (ADR 0526), and Control with the wheel zooms about the
pointer in all three through one `viewer_host::ZoomWheel` (ADR 1466). **A wheel counts whole notches in both toolkit
windows** — `viewer_host::WheelTravel::of` takes a wheel's angle and a touchpad's pixels, `DiscreteWheel` counts the
fractions as GTK's `DISCRETE` controller does, a notch scrolls `SCROLL_NOTCH`'s 48 logical pixels, a click zooms one
step and less than a click zooms by that part of a step at once (`ZoomTravel`) — and a zoom gesture holds the page
point it was pointed at through the steps on which the page is centred (ADRs 2274, 2275); a finger's surface pixels move
both pages as far as it went, by `WheelTravel::distance`, and under X11 a touchpad is a wheel's fractions in both (ADR
2291). **A draw a person is waiting for says so and
offers a key**: a page — or, on the window that draws whole frames, a view — still being drawn
after `viewer_host::drawing::WARN` puts a sentence in the status bar naming Escape, and Escape then
takes the drawing thread back without reporting anything to the core, so the page keeps its picture
and is drawn again when the view moves (ADR 0729). It is the owner's "warn the user and allow the
user to abort, however don't block", and it is a *warning* rather than the deadline ADR 0657
measured and refused. **A page still being read at launch is a loading bar**: each of the four
windows waits for its opening thread no longer than the same `WARN`, then answers its toolkit and
shows how much of the page's `/Contents` has been read and how many operators that was — the
confined window's figure relayed from its worker — until the page is in (ADR 1811). **The confined
window answers while its worker draws**: each command goes to the worker on a thread of its own,
waited for no longer than `WARN`; past it the title says what the worker is doing, Escape, `q` and
the close act at once, and every other key is held and done in order once the worker is free (ADR
1827). **A page that draws for seconds is shown as it is built** in the two toolkit windows: past
half a second the drawing thread is asked for the page so far four times a second, in painting
order, and the bar carries how much is drawn until the page lands (ADR 1825); **between a zoom and
its frame they show the page's last frame scaled to the extent it has now**, by the toolkit's own
scaled draw, until the part-way picture or the frame replaces it (ADR 1843); a page they hold no
frame for — turned back to, scrolled back into a column — is shown from a reduction of its last
frame, `viewer_host::stand_in`'s `RETAINED_EDGE` on a side and `RETAINED_PAGES` kept, `--proxy-pages`
the extent as in `quorra` (ADR 1866); a
page's own pixels go up at a whole device pixel, a transition's faces being that same picture, so
the effect ends on the page without a seam (ADR 1865); a page drawn whole of which the window shows
less than half the rows — a tall page a few notches into a wide window, a neighbour scrolled mostly
out of a column — has those rows drawn first, a look over the stand-in, and the page whole after as
the frame (ADRs 1868, 1930); a page zoomed until its largest window keeps at most seven eighths of its commands or half its pixels (ADR 1994), or past `viewer_core::MAX_PIXELS`, is drawn as the window shows it — the viewport and a quarter round it, answered `Rendered::Listed` and drawn again when a scroll leaves it (ADR 1930), its pixels the page's drawn whole to the byte — drawn on the page's own grid with its corner composed last, its converters the page's (ADR 2132), and each mark measured where the page measures it, only its write moved by the corner (ADR 2133) — a page past 8191 pixels measured as its window is (ADR 2058) and cut into strips as a page inside that range is (ADR 2068), its commands culled from bounds kept per magnification and drawn as a selection of the page's own list rather than a copy (ADR 1980), a whole page's cull answered by the bounds' edges and a list of `display_list.rs`'s `LET_GO_ON_THE_POOL` commands or more let go of on the pool (ADR 1941) — rather than whole or refused, and a refusal any window meets is said once for its view rather than at every command (ADR 1892); the confined window draws a page past the bound from the marks its worker sends, on the device at the window's size or as the window and its margin on the processor (ADR 1944), or from the window its worker draws where the marks cannot cross (ADR 1986) — each route under the bound too wherever the toolkits' rule holds (ADRs 2016, 2026) — and a page past the bound whose marks cross takes the route the window prices with the crossing counted, its marks or its worker's window (ADR 2017). **A pipeline the device's
compiler refuses is captured and kept** — the render lane's by kind and format, the compute lane's
four as one outcome, a generated function shader's by its key — so a frame that needs it is refused
in the adapter's words without a second compile, the title taking the refusal's first line, and
every other frame stays on the device (ADRs 1929, 1962); the image and line lanes write their
polygons' corners by constant indices, which DX12's FXC compiles (ADR 1928). **A page the graphics device
refuses is paid once** in `quorra`: the device is not asked again for those pages at that
magnification — nor, where the adapter refused the compute lane's pipelines, for any frame of that
coverage while the window's device lives (ADR 1968), nor for a retained page's picture of that lane
(ADR 1984), and a refusal the retained layer heard first is told to the window once so both memories agree (ADR 2000) — the processor's draw is taken back at the first notch of zoom that leaves it, and past
`WARN` it is warned of and Escape stops it, as on `--cpu` (ADR 1821); the lane is chosen by each lane's last measured cost per command for the pages, a frame the device is predicted to draw within `WARN` staying on the device and the device asked once on pages the processor has drawn — a frame finished from its drawing ahead or taken back included — before the processor keeps them (ADR 1860), and the device priced by building its scene or by drawing the one it holds, whichever the frame does, every frame's lane said on the trace (ADR 1886), that scene built after a processor frame while nobody waits and a processor frame's upload drawn through a lane of its own so that it does not replace it (ADR 1898), and a processor draw is shown as it is built, its whole-page picture drawn on the processor too (ADR 1831; page one on the processor where it is the faster lane is `CLAUDE.md` principle 2's, `doc/questions/A393`); a processor window's stand-in is resampled on every core and a refusal holds for its view (ADR 1832). **A measured price prices only the view that measured it**: a sample carries each page's target size and transform, the window's extent, its command count and the device's coverage lane, and a view with no compatible sample takes its bucket's seed, said to be a seed, rather than another zoom's or another coverage's measurement — nor is a whole device frame's price scaled by the share of the page a view reaches (ADRs 2196, 2197). **Page one that reads for seconds is drawn while it is read** in `quorra`: the reader hands its list over in parts, the opening thread draws each into the first frame's target from half a second into the read and the window shows it, and the first frame is finished from it — drawn in as many even strips as the pool is wide, each batch of waiting parts in one pass on the pool and the rest of the list there once the read ends, the page drawn whole at those cuts byte for byte, and a mark across a cut ends it; the toolkit windows call the same `viewer_host::Ahead` (ADRs 1855, 1867, 1896). **A page turn is drawn while it is read** in `quorra`, on either surface, and in the toolkit windows (ADRs 1871, 1886): the core leaves the turned page's read to the host (`Viewer::handle_unread`), the read is handed out owned (`Viewer::read_out`) to a thread of its own beside a drawer while the key's handler returns with the retained picture on the screen, a second turn waits for the first's read, and the frame is finished from that drawing — in `quorra-gtk`, `quorra-qt`, the C window and `quorra`, whose loop keeps the page being left on the screen while it waits to be woken, every unread page of a two-page spread in the one read and a tick that turns the page (§12.4.4.1's `/Dur`) handed out as a key's turn is (ADRs 2344, 2360); and `quorra --cpu` draws page one while it is read and shows its composer's frames as they are built (ADR 1871), each
finished frame and look waking the window's loop rather than waiting for its next tick (ADR 1936). The confined worker draws a turn beside its read too, with the two types the arrangement is made of rather than `viewer-host`, which a confined process does not link; its launch still reads page one before the window has a size (ADR 1871 §4). **The
document chooses what opens**: all six of Table 29's `/PageMode` values reach a window in all three
hosts — four name a panel, `UseNone` names none and `FullScreen` is §12.4.4's presentation — and
§12.2's `/DisplayDocTitle` puts the document's own title in the title bar of all three, on one line
whatever line breaks its text string holds (ADR 1973), `/FitWindow`
sizes each to the first displayed page, and `/CenterWindow` centres `quorra` and `quorra-qt`, where
GTK 4 has no call and Qt under Wayland no effect, and each says so and names it in the title — one
placement decision for Qt and the C window's caller alike (ADRs 1429, 2308). **No dialogue a
document raises opens over a presentation** — `quorra-gtk`, `quorra-qt`, a C caller's and
`quorra`'s, whose card over the slide is Table 29's other window as its panel is: a question, a
script's question and a password prompt wait until the window leaves full screen (ADRs 2309, 2329);
`quorra-confined` presents nothing. **A document this program
cannot open, and one whose page tree has no leaves, are two sentences rather than an exit**, in all
three windows (ADR 0564); one whose pages were found by Table 31's `/Type /Page` because its tree
yields none is shown with a note saying how many and that their order is the object numbers'
(ADR 1533). §12.6.3's trigger events are
raised by the pointer.

### The drive, the boundary and its consumers

**And what a reader does is driven in all three windows by one command**: `tools/drive-windows.sh` takes them — and `quorra-confined` through its own steps — under `Xvfb` through open, the outline, page turns, zoom, find, a popup, a link, a markup and a form saved and reopened, the restriction levels, print, a password and AT-SPI, and photographs each step (ADR 1453). The find bar finds a right-to-left word typed in reading order on a page that shows it as presentation forms in display order, folding the forms and reading the order off the glyphs' positions (ADR 1465), and a word typed without its vowel marks or accents finds one printed with them, while a mark typed is asked for (ADR 1477). No step of the drive rests on a person looking at a picture: the reopened form is read off AT-SPI in all three windows, `quorra`'s form nodes carrying each field's value and each character's place in it (ADRs 1489, 1501), and `quorra-confined`'s device refusal is driven to its title (ADR 1478). The `ask` level's question is answered both ways in all three windows — a toolkit dialogue's buttons pressed through their AT-SPI action, `quorra`'s drawn card by its two keys — and the answer read off the log and the drive's server (ADR 1540). The reader's three policy words are driven in all four windows, each with the word and without it: a signature called valid only under the anchors `--trust-anchors` names, a reference `XObject` drawing the page `--reference-files` supplies, and a layer drawn for the reader `--reader-name` names and not for another (ADR 1580).

**All of it sits behind `viewer-core`**: `Command` in, `Event` out, `Query` → `Answer` beside
them, with no type from a windowing or graphics library anywhere in its API.
[`doc/ui-boundary.md`](ui-boundary.md) is the whole story; ADRs 0116 to 0121.

**Seven consumers on that boundary, and not one of them has ever asked for a new message.** The
list below is grouped by crate, so the seventh — `viewer-ui`'s second window, `quorra-confined` —
sits inside another bullet. **Three host *crates*, four *windows***, which is what a sentence below
counting three means; `tools/state.sh windows` excludes the fourth and says why in its own output.
That is the boundary's own evidence, and what let the C ABI be frozen once two Rust consumers had
shaken the API out (`doc/todo/30`). **A *clause* has asked for one**, which is the other direction:
§12.4.4.2 conditions its whole state machine on being in presentation mode, and full screen is
chrome, so `Command::Present` is a statement only a host can make (ADR 0316).
[`doc/ui-boundary.md`](ui-boundary.md) holds the consumers' account and the test a message has to
pass.

- **`viewer-ui`'s winit window**, and a **headless test harness** (`crates/viewer-core/tests/headless.rs`).
- **`viewer-confined`'s `pdf-view-worker`** — a `Viewer` and `render-cpu` behind seccomp-BPF,
  Landlock and an address-space ceiling, with `viewer_confined::Confined` speaking
  `Command`/`Event` and `Query`/`Reply` to it over a pipe: principle 3's other half, beside the
  three image codecs ADR 0014 confined. **One outcome in `viewer-core` had to change for it** —
  `Rendered::Listed` (ADR 0640): rules 2, 3 and 4 already forbid that crate a
  filesystem, a clock and threads it was not handed, which is a description of a confined process.
  The worker rasterises on a pool as wide as the machine, built after the confinement so every
  thread inherits it, with `MALLOC_ARENA_MAX` set by `confined-transport` so that `glibc` does not
  ask the kernel the processor count the filter kills for (ADR 1554). Verified by drawing a page
  byte-identical to this process's, and by asking the *kernel* rather than the source whether the
  worker can open a file, open a socket or start a program. **Every `Query` crosses**, the panels
  and §12.7's whole form among them, so a confined host builds native controls rather than taking
  a form as pixels. **A frame carries the marks or the pixels**, chosen per page in the confined
  process by comparing two byte counts (ADRs 0607, 0626, 0633): a window on this boundary receives
  display lists because a process holding a graphics device cannot be confined at all, an
  unchanged page's `Arc` identity survives the pipe so a host-side cache still hits, and the
  decoder refuses a raster target past what a render request is held to, admitting a page past it
  as its marks only for a host that said it draws such a page, and a window its worker drew of any
  page only for such a host (ADRs 1944, 1986, 2016). **A draw is stoppable on
  both sides of the pipe**: a `Canceller` ends the worker, because a cancel a hostile document can
  decline is not one, and on the host's side `pdf_render::Interrupt` is raised between commands by
  the policy `viewer_host::drawing` gives every window, `quorra_window` among them — a draw is
  abandoned exactly where finishing it would produce a picture the program will never show, with
  no clock read, because a document picks its own cost (ADRs 0650, 0657, 0668). A C caller holding
  a bare render request is the one host without a way to raise the flag, which is an entry point
  the header does not have (`doc/todo/30`). A document
  too large for the ceiling is refused by name on a budget derived from the ceiling, and a worker
  killed anyway carries its own last line to the host. **And it can be *given* a face** (ADRs 0870,
  0880): a worker that cannot walk `/usr/share/fonts` sends a description — a family, a weight and
  the characters a script needs — and its broker matches, reads and answers; the allow-list did
  not move for it, and declining is the default everywhere. The allow-list admits one *command* for
  the descriptor rather than one call — `fcntl(fd, F_GETFD)`, which `OwnedFd::drop` asks — and
  every other command of that call still kills (ADR 0888); it, `pdf-sandbox-worker` and
  `pdf-script-worker` close every descriptor they inherited before reading a byte (ADR 1945). **A window uses it** —
  `quorra-confined`, the one host on that boundary and not a fourth toolkit: it holds no `Viewer`,
  no document bytes and no parser, so what it owes is decided by the operation it can perform
  rather than by its size (ADR 1190). Escape ends the worker and the in-flight draw together (ADR
  0713), §7.6.4.1's password crosses as `Command::Open`'s `Secret` (ADR 0718), `Query::View` and
  `Command::View` restore the view the worker produced when one dies (ADR 0737), and the graphics
  device draws its pages on a render thread of the window's own, the CPU thread kept for the
  frames the device refuses and each said in the title (ADRs 0725, 1466), `--cpu` the window with
  no device. The three established windows stay in process; for `viewer-ui` the move is a change
  of tier whose number is measured (ADR 0597). ADRs 0218, 0223, 0235, 0241, 0889; `doc/todo/34`,
  `doc/todo/15`, `doc/todo/59`, `doc/todo/61`.
- **`viewer-gtk`'s `quorra-gtk`**, a real GTK4 application on the same boundary: the panels in
  a `GtkListView` over a `GtkTreeListModel`, §12.7's fields as native widgets placed over the
  page, the selection and §12.5.1's focus ring drawn in the theme's own colour, and the three
  decisions a host owns — §12.7.6.4's file, §7.6.4.1's password, and how much of what the document asserts over its reader this
  window obeys (ADR 0604). `gtk4-rs` is Rust-safe with no C++
  bridge, and the crate keeps `#![forbid(unsafe_code)]` to prove it. **Tier 1 to the toolkit, because GTK4 admits no other**: a widget has no
  native surface and GSK hands out no device, so the page — drawn by default on a device of the
  window's own and read back onto its medium by the device's own pass (ADR 2150), by the drawing
  thread under `--cpu` or where the device refuses or is predicted the slower lane (ADRs 2049, 2070,
  2071), its launch held to bands on both lanes (ADR 2125) — becomes a `gdk::MemoryTexture` with no
  conversion at all, placed at one device pixel per texel: each placement gives its picture the
  whole translation and scale from device to logical pixels, a stand-in scaled to the extent asked
  for until its raster arrives (ADR 2214) — and on Wayland at the surface's own fractional scale, read
again when the surface's scale changes, a finished frame put on the surface's pixel grid (ADR 2250). A form host draws the page *without* its widget
  appearances — §6.3.2.2's "unless otherwise instructed" as `Command::Delegate` — and at the
  *scale* it states. ADRs 0244, 0245.
- **`viewer-qt`'s `quorra-qt`**, a real Qt 6 Widgets application whose pages are drawn and read
  back as `quorra-gtk`'s are, into a `QImage`, and the one that costs a C++ bridge: **one hand-written `unsafe` token in this crate**, the `unsafe extern "C++"` header `cxx`
  requires, under `#![deny(unsafe_code)]` with one exemption on `mod bridge` and a test asserting
  its position — and asserting that the crates lifting the denial are exactly the ones the
  workspace names, every other crate still *forbidding* it. **The claim is about this crate rather
  than about the tree**: a C ABI writes its entry points as `pub unsafe extern "C" fn`, which is
  what a C ABI is, and the crates and worker programs that lift the denial are read off
  `tests/unsafe_position.rs`'s `only_the_named_crates_in_the_tree_lift_the_denial`. A refresh's
  placements are taken once and copied into `QImage`s by one synchronous C++ image sink local to
  `showFrame`, before the borrowed slices end (ADRs 2236, 2237); on Wayland it holds the size Qt states before
the compositor's first configure, so page one is asked for at the scale the window is given (ADR
2267). **Both toolkit windows can say
  what a zoom reached**: under `PDFVIEWER_ZOOM_PROBE=1` each writes versioned `zoom-record` lines —
  a wheel callback, its modifiers, the branch taken and the host's outcome counted apart, and the
  content widget's geometry in each coordinate space it is measured in — and `quorra-gtk` with
  `PDFVIEWER_ZOOM_CAPTURE_DIR` also keeps each settled whole-page source joined to the generation
  that painted it; `tools/zoom-probe-drive.py` drives an owned window through them and
  `tools/zoom-probe-records.py` refuses a stale, foreign or ambiguous record. On a nested headless KWin
  `tools/zoom-probe-wayland.py` compares the GTK window's settled page with the compositor's output
  at each fractional scale and sends the wheel through KWin's own input interface, reading a notch
  and a zoom's drift off the output (ADRs 2250, 2251, 2274, 2275). They observe input and
  geometry and judge no latency and no sharpness (ADRs 2224, 2225, 2228, 2229, 2240, 2241).
  **`crates/viewer-host`** holds what the hosts decide alike and no toolkit type names — the panel
  rows, the control decision, §12.7.6.4's file policy and the launch timeline among them. ADR 0246.
  **`viewer-host` also decides the *shape* of a thread for both hosts**:
  Rust never calls a Qt object here, so a finished page cannot be pushed into `QApplication::exec`
  and is *pulled* instead, on a timer whose interval `viewer-host` decides and each toolkit arms —
  which is what `Clock` and the accessibility drain already do, and is why `viewer-gtk` does not use
  the file descriptor GTK would have given it (ADR 0668). **A pull has one moment it cannot be made
  at**: a poll asks the toolkit's loop for a turn, and at launch that loop is inside its own first
  frame, so a page one already drawn waits for the timer (ADR 0678 measured the cost). A host with
  nothing on the screen yet therefore *waits* for page one, out of a one-refresh budget spent once
  over the whole launch, and polls for everything after it (ADR 0678, trap 21).
- **`viewer-ffi`**, a C ABI over the same vocabulary, with a hand-written `include/quorra.h`
  and a `c/open_a_page.c` that a test compiles with `-Wall -Wextra -Werror` and runs. Four shapes
  decide it, each because C takes something away that Rust gave: **commands are functions**,
  because a union's size is part of an ABI and a symbol is not, so a command added later costs a
  compiled caller nothing; **events and answers arrive owned in a batch the caller frees**, so no
  borrow of the viewer crosses and re-entrancy stops being a rule anybody keeps; **a render
  request is an opaque handle** the caller may move to its own thread, because a display list is
  clauses 8 and 9 in a data structure and a frame comes back by copy into the caller's own buffer;
  and **a variant added later is named, described and counted** — `quorra_abi_check` turns "fails to
  compile in every consumer" into "fails to start, once, naming the number that moved", which is
  weaker and is the strongest thing C admits. A `Command` is a symbol and only an `Event` is a
  number, so the ABI grows without `QUORRA_EVENT_KIND_COUNT` moving (ADR 0346). **How much of `Command` and `Query` a C
  caller reaches is counted rather than claimed**: `tools/state.sh hosts` says, and names what it
  does not (ADR 0509). **Every `Query`
  reaches a symbol**, held by a test rather than by care: `every_query_reaches_the_abi.rs` matches exhaustively over the
  enum, so a question added to the boundary fails to compile in this crate (ADR 0576). **And
  `tools/state.sh windows` asks the same question of each window** — the instrument for "all three
  hosts stay level" (ADR 0577). **It prints the *reading* beside the count**, one line per unreached variant saying whether it is a
  debt and why, checked in both directions — because a count of what a window does not reach is not
  a list of debts (ADRs 0603, 0604). **Above the vocabulary, `quorra_window`
  runs the loop the toolkit windows share** — `NativeWindow` implemented in the library, the open
  joined at the first resize, the drawing thread on a tick the window times, a key acted on, the
  prompt and the *ask* level's question answered — and owes a C host its sentences, repaints and
  dialogues as a batch of records, never a callback; `c/drive_a_window.c` is that loop (ADR 2088).
  **What changed is a record and what it changed to is read**: the panels' rows, the form whose
  controls the caller places, the popups and the marks over the page cross as the handles the
  vocabulary already hands over, and a control's commit goes through the decision every window
  takes (ADR 2104). What a document asks of the machine is carried out once in
  `viewer_host::native_window` for the two toolkit windows and the C window, each keeping only its
  dialogue (ADR 2105). The C window opens under what a host states on a `quorra_settings`
  handle — the levels, and the reader's anchors, target documents, audience and conversion — by
  the command-line words every binary reads through `viewer_host::policy::Words::take` or by
  number (ADRs 2120, 2156); the print key is `viewer_host::print_key`'s in every window (ADR 2157);
  the launch is `viewer_host::native_window::Launch` and `NativeWindow::sized` in the three windows
  (ADR 2172), and a named reader's layer reaches it on the panels record and its frames (ADR 2173);
  and the chooser, a script's dialogue and a print grant are records its caller
  answers (ADR 2121). **And .NET reaches the ABI through `hosts/windows/Quorra.Interop/`**: one
  C# declaration for each function, constant and struct `quorra.h` states, under the header's own
  names, held to the header by `tests/header_and_interop_agree.rs` and compiled by CI (ADRs 2054,
  2073; RFC 0010). **The WinUI 3 window over it is `hosts/windows/Quorra.App/`**: the page drawn on
  the processor into `Image`s, the marks over it in Windows' highlight and accent colours, a
  platform control over each widget whose appearance the window draws itself, committed as the
  toolkit windows commit it, the title and the status line, the keys, the pointer, the wheel and
  Control with the wheel (`quorra_window_zoom_wheel`, the fraction of a step carried by the
  library), each owed record copied into managed memory before the library frees it and answered on
  a card — a password, a question, an archive target and profile, a script's dialogue, a print
  preview — one at a time, a withdrawn or replaced one never answered, the open panel's rows read and
  only its realised thumbnails, and the shared restriction menu as its Permissions menu (ADR 2185);
  the chrome the library says is in effect, read through the native window rather than from the
  document's preferences (ADR 2200), Table 29's `HideWindowUI` hiding the navigation toggle and the
  status line and never the document or its outline (ADR 2201); Table 29's full screen as the
  `AppWindow`'s own presenter and Table 147's fit and centre through `quorra_window_place`, compiled
  and given to CI's `window` job to run (ADRs 2308, 2436);
  the pictures copied only where a repaint calls them fresh (ADR 2168), Table 231's combs, the
  measured fit, §12.5.6.14's popups and §12.9's traced path drawn (ADR 2169); its C# compiled on
  Linux against the Windows App SDK's references by `tools/dotnet-check.sh` (ADR 2184), its XAML only by CI's `window` job, on
  `windows-latest`, with `viewer_ffi.dll` beside it (ADRs 2136, 2137, 2152, 2153); published
  self-contained by that job and shipped in the snapshot pre-release as
  `quorra-winui-x64-<commit>.zip`, started there on a document and its title read, a gate (ADRs 2374,
  2436), and run there under its `--probe` word on Table 147's placement and full screen (ADR 2436).
  ADRs 0247, 0509.

### A password field's value on a save

**A password field's value does not reach the file it is saved into.** Table 231 bit 14's own
NOTE is the reason: `save` writes neither the value nor the appearance for such a field, and
reports each one it withheld. ADR 0247.

### A program's questions

**And a *program* can ask it questions.** `tools/pdf-retrieve` is JSON on stdout over the readers
this tree already had, and what it adds is the three joins between them `doc/todo/63` named:
§12.3.3's outline turned into the range of pages a section occupies, the text cut at that section's
own two headings, and §12.5.6.10's `/QuadPoints` deciding which *section* an annotation belongs to
rather than which page. **Its default answer is `Interpretation::text` byte for byte**, which a
test asserts: a tool that tidied it would put itself between a caller and the only independent
measurement this project has of its own extraction. ADR 0257.

### Archival documents, validated and made

**And a program can ask it whether a document is an archival one.** `crates/pdf-archive` holds a
document to ISO 19005-2 or ISO 19005-4 — six targets, the levels and flavours being an
applicability column over one requirement table rather than six implementations — and
`quorra-retrieve archive-check` is the front door, JSON on stdout. **A verdict says what it did
not check**, by name, per requirement, per target, with the reason each one is unchecked: a reader
this tree lacks, a standard the project does not hold, or a clause that genuinely decides nothing.
That is `A20`'s requirement and `doc/adr/0923`'s shape, and it is why the verdict leaves out
the obligations ISO 19005 places on a *processor* rather than on a file — a validator cannot pass
or fail a document for those. Non-conformance is an answer rather than an error, so the exit status
stays zero and the JSON carries the verdict. The JPX baseline its JPEG 2000 rule turns on is read
from ITU-T T.801 M.9.2, the held text of ISO/IEC 15444-2, subclause by subclause (ADRs 1383,
1399). A packet's properties are held to the edition of the XMP Specification part 2 itself
identifies, its bibliography's September 2005 one, so a predefined property is held to that
edition's type — an `xmp:Rating` of `1.0` spells a Real where it types an Integer, and the corpus
comparison records its pass of that file as a ruling against the corpus (ADRs 1991, 2020), and a
structure of each of that edition's structured types is held to the fields its own table defines
(ADRs 2295, 2318). The survey walks a soft mask's group from the graphics state that installs it,
as a form from §8.4.1's initial state, so a selection under a mask is compared with the
interpreter's like any other (ADRs 2247, 2263).

Its own reading is compared against the veraPDF corpus clause by clause, and **the comparison is
adjudicated rather than tolerated**: where the corpus and the clause disagree, the clause is read
first and the ruling recorded with its reasoning, because a disagreement is not evidence of
ambiguity by itself. The approved PDF Association errata are an input beside the standard, one of
which *withdraws* a rule from part 4. `tools/state.sh archive` prints where the comparison
stands, and `doc/todo/02` §2's map places its walk (ADR 1015).

**And a separate ledger says how much of the *standard* the viewer implements**, clause by clause, one row per subclause in `doc/conformance/ledger.toml`. Its statuses include `departed`, the owner's word in answer to `doc/questions/Q63`, so a deliberate departure does not wear `partial`'s word for unfinished work and `tools/state.sh` counts it as its own figure; `doc/PLAN.md` section 5a defines it (ADRs 1119, 1622). Three of that script's sections read the tree rather than the ledger: `departures` prints each `departed` row's deciding ADR and what has cited it since, `remedies` prints per profile and target how many answers sit at a site the target's own listing does not name, and over the corpora how many documents fail each site with no remedy built (ADR 1983), and `flags` holds at zero the rule that **every command-line flag a message names is one the program accepts** — both populations derived, the programs from the workspace's manifests and the flags from each binary's own source (ADRs 1166, 1213). The ledger's own count says per status how many rows a fixture holds, how many only corpus walks or corpus witnesses hold, and how many name no test; an `implemented` row of the second kind keeps its status and owes a fixture, and the gate admits exactly as many as it has named (ADR 1497).

**And it can *make* one.** `pdf-transform`'s `archive` verb brings a document to a stated target in
three stages — validate with `pdf-archive`, decide each failed requirement as a refusal, an
authorised loss or a default, then rewrite through the same structure-preserving serializer `split`
and `merge` use — and **the output is validated again before a byte is written**, so what the report
claims about it is a measurement rather than a promise (ADR 0947). A source that already conforms is
copied rather than rewritten (ADR 1006), and each loss the run is willing to take is asked for by
name on that run. **What a target will not hold where it was is kept wherever a clause says where it
may go**: an annotation's marks go back onto the producer's own page under §12.5.5's matrix, with an
appended page for the two cases that construction refuses (ADRs 1099, 1105, 1123); content with no
other home goes onto a page the conversion composes from the document's own content and describes in
the document's structure tree (ADRs 1014, 1025, 1163); a metadata packet or an XFA resource the
target forbids is kept as an embedded file at PDF/A-4f and 4e (ADRs 1245, 1270); and the information
dictionary a part 4 target forbids moves into the packet under Table 349's properties (ADR 1269).
**Behaviour a target forbids is taken out and the behaviour behind it is not**, a removed action's
§12.6.2 `/Next` promoted into its place (ADR 1175); an encryption does not cross, and every flag it
withheld is named in the report and in the output's `xmpMM:History` (ADR 1187); and where a fact is
the operator's to state — which font program may be embedded, which of an ink's two definitions
wins, what a stream outside the file holds, what an unnamed optional content configuration is
called, what their own metadata field means (ADR 2082) — the operator states it and the report says whose
authority it was (ADRs 1188, 1209, 1221, 1222). Every other repair is the one its clause states: a
hexadecimal string's odd digit (ADR 1176), a page boundary by §14.11.2.1's defaults (ADR 1210),
annotation flags and states (ADRs 1233, 1234), a font the file never embedded given the face it is
drawn from — one of the standard 14 stated by name alone with the descriptor that face's program
states and the widths it was laid out by, a dictionary of the other kind from its face restated as
the face's, each read back and held to the source glyph by glyph (ADRs 2146, 2147) — the shipped face fitted
to the file's widths the loss `substitute-stretch` and a machine's face the loss `machine-face` on
every route that embeds over one (ADRs 2262, 2279), and a `Differences` name the glyph list does
not hold restated on `WinAnsiEncoding` (ADR 2278) — a font program filed under the key, and for a
`/FontFile3` the `/Subtype`, of the format its bytes are (Tables 124 and 125, ADR 2307), a Type 2
CIDFont's absent `/CIDToGIDMap` written `Identity` where the edition the source is labelled with
gives that default (ADR 2246), a composite font of an
Adobe character collection given a subset of the CJK pack's program for that collection, a second
download beside the converter that CI builds by digest and the snapshot publishes, under the loss `machine-face` (ADRs 2302, 2303, 2386, `doc/cjk-pack.md`), its descendant given a descriptor of its own where it states none or shares one at other widths (ADR 2461), an `Adobe-Identity` font re-keyed from its `/ToUnicode` values (ADR 2378) or, where no face can be keyed and the page reads back no character, its page photographed as the window draws it under the loss `text-as-image` both shipped profiles carry (ADR 2394) — an extension schema described from what the packet
states and an amendment identifier cut by span (ADRs 1245, 1246), a form's appearances constructed so
that `/NeedAppearances` can go (ADR 1257), a push button's single appearance stream made the one
entry of a subdictionary a new `/AS` selects (ADR 2319), an embedded file no part admits taken out with every
reference to it (ADR 1258), a JPEG 2000 image outside the baseline transcoded to `FlateDecode` in the
space it states, its opacity channel as Table 87's soft-mask image (ADRs 1400, 1412), a reference XObject's proxy kept and a named CMap embedded (ADRs 1285, 1286),
a TrueType code's glyph named in the encoding where only a reader's choice reached it and a Type 3
`.notdef` renamed (ADR 1942), a code a standard font's own encoding sends to `.notdef` given an empty
glyph in its CFF face (ADR 2458), a filter named by its inline-image abbreviation named in full
(ADR 1952), and a destination profile's header restated as the ICC edition its own tags are written
in, or its Profile ID as the profile's digest (ADR 2021). An ICC profile short of the tags its
edition requires is given the operator's text where this reader drew through it and lacks only
`desc` or `cprt`, and is written as its Table 65 alternate where it did not draw through it — a
stream with no profile file signature among them at part 2 and at part 4 (ADRs 2295, 2318) — at part 4 against the
edition the profile's own header names (ADRs 1990, 2006); a composite font's `/ToUnicode` stating
U+0000, U+FEFF or U+FFFE is restated without those statements where the target leaves the code free
to go unmapped (ADR 2294); an output intent's
only destination profile, if this reader cannot parse it and drew nothing through it, is written as
the shipped sRGB where it stated three components and refused otherwise (ADR 2007). A graphics state's transfer function other than the identity, and a halftone's
`HalftoneName`, go only as losses a person authorises — the function first written into the
output's own `xmpMM:History` (ADRs 1964, 1965), a halftone's own `TransferFunction` under the same
two answers where §10.6.5.6 does not require it (ADR 2007), and a `/TR2` other than `Default` is written
`Default` under the same two answers (ADR 1978); overprint mode 1 under an `ICCBased` CMYK mark is set
to 0 as a statement where the states govern no `DeviceCMYK` mark and as a press's loss where they do
(ADR 1978). A simple font's widths array is made as long as the range it states — a short one narrowing
`/LastChar`, a long one losing the entries past it — since this reader already gives a code outside
the range the descriptor's `/MissingWidth` (Table 109, ADR 2178); a bundled Foxit program carries
its notice in an XMP Rights Management packet on the font file it describes (§14.3.2, ADR 2194), and
a Liberation program whose metrics the conversion changed is renamed, its licence reserving the
family name for unmodified versions (ADR 2195) — both the owner's `A550`. Annotation flags are
answered per object under `as-shown`: an annotation hidden at rest is removed and a visible one
without `Print` is kept with the flag set, from one prepared population (ADRs 2210, 2211), and a
hidden markup annotation and its popup are removed together only where each was authorised on its
own, which says nothing about references to them elsewhere in the file (ADRs 2218, 2219), a
popup's parent being any markup annotation but `Sound`, as Table 171 and §12.5.6.24 read (ADR
2284). A conversion is the same file from every build of one source, and the instant its history
records is the plan's, the machine's clock read once or the caller's (ADR 2306). Under a
part 4 page output intent that overrides the catalogue's, an RGB mark is given a `/DefaultRGB` only
where every resource context the page and its Forms reach is proved and a neutral `Gray` page group
is shown preserved first; any other context still refuses (ADRs 2230, 2231). The validator enumerates a page tree in one walk, so a flat `/Kids` array costs it
linear rather than quadratic time (ADR 1953).
[`doc/pdf-a-mitigations.md`](pdf-a-mitigations.md) is the catalogue of each, with its clause. A
refusal is a question answered in advance: a configuration names each refusal site and its remedy,
`--remedy-sites` prints every site a target binds with what this version carries out, and the
profiles under `doc/profiles/` answer them for a purpose each (RFC 0007, ADR 1012) — each a lower
bound on what may be lost, never an instruction to lose it, `as-if-printed` and `as-shown` among them,
named with `--profile` (ADR 1942). **It is `quorra-transform archive`, a library verb, and Shift+A in
`quorra`, `quorra-gtk` and `quorra-qt`**, the target and profile chosen with `--archive
<target>:<profile>` or on the card or dialogue Shift+A puts up, which offers each profile with its own
sentence and is kept for the window's life (ADR 1966), and Ctrl+Shift+A puts it up again (ADR
1985): the core asks Table 22's bit 11 and grants the
file, the window converts it on a thread of its own through the same profiles and executor — under
the password a protected document opened with (ADR 1967) — says the report per site and writes the
file beside the document; `quorra_archive` in the C ABI (ADR 1954).

### Redaction

**And it applies a redaction.** §12.5.6.23's `/Redact` annotation names a region — `/QuadPoints`
else `/Rect`, and *within* is bounding-box intersection, a documented choice — and `redact` writes a
**new** file, never §7.5.6's update, in which the bytes are gone rather than covered: text cut by
the placed quad's own advance and held to the interpreter's code count; an image's samples, its
masks and inline images cleared on their own grids, a codec's image re-encoded Flate with its
dictionary carried and a JPEG 2000 image written back as the decoder's own integers (ADRs 1333,
1371); a painted path cut to the region's complement and a Bézier split at its roots so that the
survivor is still a curve, and a stroke cut as the outline it marks and painted in its stroking
colour (ADR 1324); a form entered under its `/Matrix`, an object another page also draws copied
rather than replaced, a clipping boundary kept without its marks, and Type 3, composite and vertical
codes, `sh`, soft-mask groups and shadings each cut by their own clause's geometry (ADRs 1351, 1352,
1363). It refuses rather than cuts wrong — a calculator function serving the region's own colours, a
JPX image beyond the operator's budget or with a component deeper than sixteen bits, or in
e-sRGB or e-sYCC, which no §8.6 space restates (ADR 2293), a file whose
dictionary contradicts its codec — each with its sentence, and the overlay text and fill it does not
compose (A65's fence), said as a departure in the report; §12.5.6.23 is `departed` on the two
refusals, the overlay being a choice its own `should` leaves (ADRs 1622, 1124, 1126, 1132, 1133,
1143, 1195, 1196, 1236, 1248, 1277).

### Files derived from a document

**And a program can ask it for a *file* derived from a document.** `pdf-transform` renders pages
to PNG, PPM or PGM at a dpi — the oracle backend's own raster byte for byte, any of Table 31's five
boxes as the extent, with or without §12.5.3's annotation pass, each page's report stating the
sub-pixel strip of raster the page does not reach (ADR 0873) — extracts a page's images through the
same confined decode the viewer draws them by, or under `--native` as the file the stream already
is, and lists or saves §7.11.4's embedded files from all three of their homes, over one
plan-and-sinks seam a KIO worker, a FUSE filesystem or a menu item calls the same way (RFC 0002 §5).
**It writes three things by §7.5.6's incremental update** — `attachments --attach`, `--attach
--to-page N` and `--remove NAME` — and **whole files on `pdf_syntax::serialize`**, RFC 0002 §10's
serializer under `CLAUDE.md`'s authoring exclusion, which emits structure and never content:
every stream's bytes cross encoded and untouched, and a reference to an object the output does not
hold becomes §7.3.10's null and is counted. It encrypts where a caller asks — `/V` 5 `/R` 6 with
`AESV3`, the one configuration Table 20 and §7.6.4.1 leave undeprecated, every unpredictable byte
supplied from outside so that a plaintext write stays byte-deterministic — and never inherits an
encryption, because revision 6 stores each password as a one-way hash (ADRs 1161, 1162).
`optimize --linearize` writes Annex F's linearised file, every offset computed to a fixed point
before a byte is written, and `pdf_syntax::linearize::state` says of any file whether it is still
linearised (ADRs 1293, 1309, 1337). `split` cuts by page, by group or at §12.3.3's outline, each
piece carrying the outline, labels and named destinations its pages reach and naming each
document-level construct it leaves behind (ADR 1461); `merge` reconciles §8.11's optional content,
§7.9.6's name trees, §12.3.3's outlines, §12.4.2's labels, §14.11.5's output intents, §12.7's form
and §14.3's metadata, each by a documented construction or a refusal by name (ADRs 0821, 1473, 1474);
and `pages` rotates, deletes, reorders and inserts within one document. All three carry §14.7's
logical structure, the parent tree rebuilt under the output's own keys and Table 354's three
namespaces answered by three clauses (ADRs 0834, 0835), and every verb takes one page-range grammar,
with §12.4.2's labels addressable as `@iv`. The suite has its own gate, with
RFC 0002 §12's perf floor and its inventories held to the document's own structure, and a corpus
walk per writer beside it, each holding every surviving page to its source page, which `doc/todo/02`
§2's `pdf-transform` row names (ADRs 0800, 0801, 0802, 0804, 0816, 0817, 0818, 0821, 0830).
**The document is parsed in a confined worker**: `quorra-transform` starts
`quorra-transform-worker`, which confines itself under seccomp-BPF and Landlock before it reads an
argument and runs the same program over a world its broker answers — each named file, document,
descriptor and output given only where the person's command line names it — and `--in-process` is
the fallback, said aloud (ADRs 2140, 2141).
`doc/todo/57` is what the suite still owes.

### A document as a directory

**And a document is a directory.** `pdf-vfs` is RFC 0003's shared core: one declarative table
that says what every path in a document-as-a-folder is, what generates it, and what writing to it
and deleting it would each mean — `pages/0007.pdf` a complete single-page PDF so that `cp` *is*
page extraction, `renders/` the same pages drawn, and `images/`, `text/`, `attachments/` and `meta/`
the extractions under the names they give — most generators being a `pdf_transform::Plan` and
nothing else, held byte for byte against `pdf-transform`'s own output so that the core cannot
become a second implementation (ADRs 0840, 0841). A generation key of (mtime, size, §7.5.5's last
`startxref` offset) is asked before every answer, so no reader is handed a splice of two documents,
and a `stat` generates, because a kernel clamps reads at the size a `stat` reported. **It is written
to** by all five of RFC 0003 §5.2's verbs — a page inserted, a page removed, a file embedded or
removed, `meta/info.json` overwritten — through §7.5.6's update edited in place, a staged write
visible in the tree and absent from the document until its flush, the commit a synced `rename(2)`
after the broker checks §7.5.6's prefix property against the disk, and §5.3's four refusals refused
by design with a sentence and an `errno` each (ADRs 0854, 0855); all five verbs are walked over the
corpus with every surviving page held bit-identical to the page it was (ADR 0860). **It has two
faces**: `quorrafs <file.pdf> <mountpoint>` mounts a document on `fuser`'s pure-Rust path, an inode per
*name* because an ordinal is a position, with RFC 0003 section 5.4's invalidation on a thread of its
own (ADR 0861); and `kio/`'s C++ worker serves `pdf:/` to Dolphin and every KIO-aware program over
`pdf-vfs-ffi`'s C ABI, showing a person the core's own sentence for a refusal and a deletion's
§7.5.6 consequence as a non-modal warning, and putting a restriction's question through
`quorra_vfs_consult` and `quorra_vfs_answer` (ADRs 0868, 0869, 0874, 0875) — driven by a KIO client
with no desktop session and by no Dolphin, so the dialogue itself is driven by nothing. **And a
confined worker is under all of it** (ADRs 0846, 0847): `pdf-vfs-worker` confines itself before it
reads a byte, takes the document as a descriptor, answers question for question what the in-process
one answers, and is *killed* for asking the filesystem anything; `confined-transport` is the wire
under it, as it is under `pdf-view-worker` and `pdf-script-worker`.

### A document's restrictions, read once in the model

**What a document asserts over its reader is read once, in
`pdf_model::restriction`, for every operation this tree performs** (ADR 0803): every Table 22 bit
the table still uses is named (bit 10, which its own row retires, has no variant, and the type says
why), the operations — a field filled, an
annotation added, a page printed or rendered, a page printed faithfully rather than degraded, a
file extracted, a file written in, a document assembled out of another's pages, the document
processed at all, and an embedded media clip handed to a player, which reads Table 286's `/TF`
rather than a Table 22 bit (ADR 1961) — each read their bit at
the document's revision and §12.8.2.2's certification besides — and §12.7.5.5's Table 236 `/P`,
the permission a signed signature field's lock states over the document, whose several instances
compose as the minimum its own words make them (ADR 1156) — and the four levels are one type
whose verdict a caller matches exhaustively. `pdf-transform` honours all four (`--restrictions`
takes `off`, the default, `on`, `warn` and `ask`,
which puts the question on the terminal where there is one and is a refusal saying nobody could
answer where there is not), **RFC 0003's mount honours all four for the same reason a pipe does** —
a file system has no dialogue, so its *ask* is a refusal with its own sentence and never a silent
proceed, and both it and *on* leave as `EACCES` — and **the viewer supplies all four**:
*refuse* is `Event::Refused`, *warn* is the edit done and `Event::Warned` after the `Dirty` it
caused, and *ask* is `Event::Asking` with the edit held until `Command::Answer` settles it — the
`Event::PasswordRequired` shape, and the condition `doc/todo/38` set for shipping a level at all.
All three windows put the question on a modal window (ADR 1145), a C host of `viewer-ffi` through
`QUORRA_EVENT_KIND_ASKING` and `quorra_answer`, and `quorra-confined`, whose click reaches a check
box and a rendition (ADR 1987) and which has no dialogue to put the question on, says out loud that
nobody could answer rather than letting *ask* behave like *on* (ADR 0814).

### A page imported from another document

**It can draw a page one document imports from another, where the person running it names the file
it comes from.** §8.10.4's reference `XObject` is a form carrying Table 95's `/Ref`, and §8.10.4.1
writes a `shall` for a processor that draws the referenced page and a `shall` for one that draws the
proxy the producer put there instead; which of the two this is depends on whether the target file is
in front of it, and with no filesystem it never is unless somebody puts it there. `--reference-files
<dir>` names a directory, `viewer_core::Command::References` carries the bytes in, and **which file
a reference names is decided by §14.4's identifier and never by the path Table 95 states** — a path
a document writes would let the file choose what this machine opens. A supplied file whose
identifier does not match is refused by name; one whose *changing* identifier moved is drawn with
the clause's own warning that it is "a different version of the correct PDF file"; and a reader who
names no directory gets every proxy drawn and is told nothing, which is what the clause asks of a
reader with no target file. The imported page is placed by the proxy's `/Matrix`, clipped to its
`/BBox`, and rendered with §8.10.4.3's annotation appearances inside that same box. ADR 1101. It is
composited as §11.4.7's second treatment asks — a transparency group under the imported page's own
`/Group`, not the medium — and inside the proxy's group where the proxy states one (ADR 1339).

### Signatures

**It can tell a person that a signed document changed after it was signed, whether its signature
verifies, and — where the person running it names a certification authority — whether the signature
is valid.** §12.8.1 divides verifying a signature into three questions, and the third is a host's
input rather than a list this program ships: `--trust-anchors <dir>` reads PEM or DER,
`viewer_core::Command::Trust` carries it in, and `pdf_signature::verdict::Verdict` is the only place
the word *valid* is reachable — its `Valid` has no public constructor and the only one that makes it
takes a proof no caller can build without a path that reached a supplied anchor. **A reader who
names nobody is told that nobody was named**, which is a statement about this program rather than
about any document (ADRs 1039, 1076). `Signature::integrity`
recomputes the digest over §12.8.1's `/ByteRange` — with the algorithms Table 260 and Table 256
name — and compares it with what `pdf_signature::cms` reads out of §12.8.3.3's `SignedData`, over a
bounded in-tree X.690 reader that allocates nothing (ADR 0215). `Signature::authenticity` then
finds the certificate the `SignerInfo` names among the ones the signature itself carries, reads its
key with `pdf_signature::x509` and verifies with `pdf_signature::pkcs1`, `pdf_signature::pss`,
`pdf_signature::dsa`, `pdf_signature::ecdsa` or `pdf_signature::eddsa` — RFC 8017's RSASSA-PKCS1-v1_5 and
RSASSA-PSS, the latter over the `RSASSA-PSS-params` the signature's own algorithm identifier
carries; FIPS 186-4's DSA; ANSI X9.62's ECDSA over RFC 5753's `ECDSA-Sig-Value`; and RFC 8032's
Ed25519 and Ed448, which sign the message rather than a digest of it. **That is all three of Table 260's
algorithm families and the fourth row ISO/TS 32002 section 5.1.2 adds beside them** (ADRs 0229,
0314, 0322, 0532). The constructions, budgets, encodings and refusal names are this tree's; the
modular arithmetic and the group law under them are RustCrypto's `crypto-bigint` and curve
packages, by owner decision (ADR 0331). **Every curve ISO/TS 32002's Tables 3 and 4 name is
computed**, and two of them by the owner's exception to that decision (`doc/questions/A170`):
brainpoolP512r1, which no package carries, is RFC 5639 section 3.7's constants over the reviewed
frames the other Brainpool packages are built from (ADR 1385), and Ed448 is RFC 8032 section 5.2
over `crypto-bigint` (ADR 1386) — private modules, advertised nowhere, held to their RFCs' vectors,
and replaced by a package the day a stable, reviewed one covers the curve. The sentences the program uses keep every
asymmetry: a mismatch is decisive, a match is the absence of one kind of evidence, and a
certificate that arrived in the same file as the signature it verifies proves the two are
consistent with each other and nothing about who made either. **Without an anchor nothing here says
a signature is valid**, and with one the sentence that does says whose anchors made it sayable. **And §12.8.2.2's second question — what changed after the signature — is answered without
mutating anything**, because §7.5.6 makes a signed revision a byte prefix of the file: the prefix
is opened as a second immutable `Document` and the two are diffed object by object, each change
ranked by which *entries* the update wrote rather than by what the object is (ADRs 1043, 1049).
**All three of §12.8.2's transform methods are read, scoped by their own parameters, and ranked on
that comparison** — `/DocMDP` against Table 257's `/P`, `FieldMDP` against Table 259's `/Action` and
`/Fields`, and `UR` against Table 258's rights, which are a *grant* rather than a restriction, so a
save that outgrows what the signature granted withdraws the `/UR3` entry rather than leaving a grant
the new bytes do not support — and all of it reaches a reader rather than only the crate (ADRs 0159,
1096, 1104). **Where the file marks a part this program
still does not do**, it says that too: Table 255's `/V 1` states that "the Reference dictionary
shall be considered critical to the validation of the signature", so a `/Reference` naming any
*other* transform method is named in the note beside the questions that were answered (ADR 0637).
**And a signature's policy is named, and its published copy checked against what the signer
signed**: §12.8.3.4.4's explicit-policy profile carries a signature-policy identifier under CAdES's
rules, and `pdf_signature::policy` reads it per signature as far as the held texts reach, every
sentence saying the policy's constraints were not enforced (ADRs 1219, 1709); the copy at the URL
the signer's qualifier names is fetched in every window only at the reader's submissions level,
compared with the digest the signer signed, and opened beside the document only where it binds
(ADRs 1728, 1738), and a C caller that fetched one itself hands the bytes in to be bound (ADR 1753).

**Two more of §12.8's questions are answered from the file rather than from a network.**
`pdf_signature::revocation` reads §12.8.4's document security store and §12.8.3.3.2's archival
attribute, applies them to every certificate on the path, and **absence of evidence is *unknown*
and never *good*** — a host willing to accept an unknown status says so in the open rather than
having it assumed (ADR 1067). §12.8.5's document timestamps are *established* rather than read:
the token has to parse, be covered by the signer's own digest attribute, verify, and reach a
supplied anchor, with each token's path validated at the next one's stated time, and what a token
merely claims is reported as a claim (ADR 1071).

### Accessibility

**And it speaks a page.** `viewer-accessibility` maps §14.8.4's standard structure types — and the
eleven of Annex M that §14.8.6.1's default PDF 1.7 namespace alone defines, `TOCI`, `Note` and `Code`
among them, each read in the namespace its element ends in (ADR 2014), a `Private` element and
everything below it withheld as ISO 32000-1 Table 333 asks, and a foreign namespace's homonym or a
circular role map described for what the reading side resolved rather than read again from the name
(ADR 2028) — onto
`accesskit::Role`, and `accesskit_unix` puts the result on AT-SPI — where a real client walks it
off the bus, `Frame` → `DocumentFrame` → the page named by §12.4.2's own label → §14.7's elements,
with §14.9.3's `/Alt` where the document states one, a table cell announced with **the headers that
describe it** — Table 384's `/Headers` where a producer wrote one and §14.8.4.8.3's own search where
none did, each header said in the author's own short form where it states Table 384's `/Short` — a
table described by its stated `/Summary` (ADR 0715), a `TH` carrying the axis §14.8.5.7 gives it
rather than a guess, a list that §14.8.5.5 says **carries an earlier one on** saying so and
pointing at it with AT-SPI's `FlowTo` where the earlier one is on the same page (ADR 0748), an
element placed by
§14.8.3.3's content rectangle — everything its own content drew, then what the document says
about it, then what it encloses, which is what places a table cell whose only content is a widget
(ADR 0768) — and a `StatusBar` group carrying **what the
page could not draw**, because the person who cannot see the page is the one for whom a count in
the title bar is no answer. An element's own text carries Table 380's line under, over or through
it and its raised or lowered position — `/TextPosition`, else the sign of `/BaselineShift` read as a
superscript or a subscript, a choice the clause leaves — on its runs, where AT-SPI publishes the
underline and the line-through (ADR 2384); a C caller reads the same three through
`quorra_structure_style`, an unstated colour as an absence rather than black (ADR 2422). An untagged page says that it is one rather than being given an
invented reading order, and its widget annotations cross as controls a client can press, named by
Table 226's `/TU` and in §12.5.1's tab order (ADR 1369); a tagged page's widget its structure left
out crosses the same way, after the structure's own nodes (ADR 1381). A client's click on a text field or a choice gives the keyboard to that field in all three windows (ADR 1566), and each window tells the bridge whether it has the keyboard, so a client following the active window reaches the page (ADR 1565). A tagged document's page its
structure reaches nothing on says that instead of the untagged sentence, and names the producer's
omission where the catalog claims §14.8.1's tagged PDF (ADR 1393); a `Form` element with no text of
its own is named by its `/T`, else its field's `/TU` or §12.7.4.2 name (ADR 1394), any element with
no text of its own is named by its `/T`, and a titled `Sect` is a region (ADR 1405). **And a client may *act* rather than only listen**: a check box says a
click may be asked of it and a person using a screen reader alone can tick one, an element says it
may be scrolled to, and the page says a caret may be put in it — each carried out as a place, in the
device pixels a pointer already works in, so the boundary gained no message and one definition of a
click serves the mouse and the bus alike (ADR 0425) — **in all three of this project's windows, and
one definition for the three of them** (ADRs 0623, 0630): a click on §12.7.5.2's check box or radio
button is decided once, by `viewer_host::form::Clicked`, so a person using a screen reader ticks the
same boxes and is refused the same read-only ones whichever window they opened the file in, and a
box ticked is saved as §12.7.5.2.3's name in the field's `/V` and each widget's `/AS`, so the file
reopens ticked in every window (ADR 1453). A field's characters are placed where its layout put
them, so a client asking where a character of a value lies is answered by the document's own node
(ADR 1501), and that node carries the page area's place in the window as its transform, so its
answer lies inside the field a person types into — in GTK's window too, whose own entry answers no
character's box at all (ADR 1516), and in Qt's, whose node is placed by the page area the window
reports rather than by its frame (ADR 1528). The one
async runtime this tree has is confined
to that crate, it is Linux-only in its own manifest, and the adapter is created **after** the first
frame is presented. ADR 0214.
