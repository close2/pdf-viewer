# A mark sized before it becomes geometry: the increments ADR 1817 picked

Status: **open — sections 1, 2, 3 built (ADRs 1829, 1835, 1841) and section 4 built in `quorra` (ADR
1855); section 5's scene half built (ADRs 1877, 1878); section 6 built in `pdf-model` (ADR 1880), section 7 the next build for the first frame (ADR 1888).** ADR 1817 argues the file and prices it against the other candidate, ADR 1845
re-takes the pipeline after sections 1 and 2 and picks between sections 4 and 5; this file is the
work, in the order it is to be taken.
Priority: 40-band — performance, measured and priced; the band was full when this was written.
Witness: `/tmp/demo_heavy_engineering_A1.pdf`, the owner's, **not in the repository** — 4 408 405
marks, 99.0 % of them under a quarter pixel at the window's fit; the deterministic in-tree generator
of its shape — cubicle grids, hatching, labels — is `crates/render-raster/examples/heavy_drawing.rs`
(ADR 1823), and a gate that holds its ladder is still owed.
Instrument: `crates/render-raster/examples/tile_price.rs` — its census, its fit row, its floor row;
`examples/pipeline_price.rs` for the read against the draw (section 4).
Clauses: §8.4.3.3 (Table 53's caps), §8.5.3.2 (a stroke's shape), §10.7.4 (the area a shape paints).
Code: `crates/render-cpu/src/stroke.rs` (`draw_stroke`, the place the classification goes before
`convert::path`), `crates/pdf-render/` (the classification itself, one copy for every backend —
trap 2), `raster/crates/raster-gpu/src/` (the device primitive and its per-mark price).

## 1. The processor's kernel — the first increment

A single straight segment, stroked with any of Table 53's three caps, no dash, a solid paint and
§11.3.5's `Normal` blend, under a transform that keeps the axes, whose ink lies within a 2 × 2
block of device pixels: its exact area in each pixel of the block is computed in closed form — a
rectangle's overlap is separable, a round cap's is a disc's segment — and composited once, with
the clip and the soft mask multiplied as every other mark's are. Nothing of `tiny-skia` is entered.
That population is the census's whole witness population; everything outside it takes today's path.

**Built** (ADR 1829): `pdf_render::thin_segment` admits the segment in any direction — the axis-only
first cut left the witness's 667 904 hatching dots on the general path — and
`render_cpu::thin_segment::draw` paints it. The fit raster on one thread went from 2.9 s to 0.84 s,
not the 0.46 s priced, and what is left is the command loop: every strip replays every command's
extent test. Two things are still owed. `drawn_row_by_row` may let a cut pass a stroke this kernel
draws, whatever its width. §8.5.3.2's round dot, a `m` and an `l` to one point, is one of these
marks on both backends (ADR 1857), unless no pixel of it holds a level; on the processor so is a
single-point closed subpath, and a dot inside a longer path where no two of the path's dots share a
pixel (ADR 1875); the device takes `m h` through `pdf_render::segment_ends` too. The kernel composites each area unrounded and rounds once when the pixel
is stored, which was the whole of the line lane's residue against it on the witness (ADR 1876).

**Owed beside it**: a unit ladder of the kernel against the clause's closed form — sizes 0.05 to
2 px, positions across pixel edges, all three caps, both axes — whose expected values are the
areas §8.4.3.3 and §10.7.4 define, never the old path's; a control that must stay on the general
path (a two-segment stroke, a dashed one, a turned one); `raster_golden` and its division test with
every moved digest named against ADR 1742; `tile_price`'s fit row and `tools/state.sh heavy`
before and after, as clock walks. Priced at 7.3 s → about 0.46 s on the witness's fit raster at
three times the measured floor (ADR 1817 section 3).

## 2. The same classification on the device

Raster gains a primitive for the same marks, its coverage computed analytically on the device and
its budget counted per mark — the witness is charged 96 B a command today by the encoder's
pre-count of one rect and one quad instance (ADR 1823), which the 2 GiB frame budget admits whole;
what the primitive removes is the resident outline per one-segment stroke, the 512 MiB resource
budget's ceiling (ADR 1823's chop 16), and the host's flattening of every such stroke. Byte agreement with the processor's kernel is the oracle's question and is
stated as raster's ADR 0082 tolerance, not assumed.

**Built** (ADR 1835, raster's ADR 0097): `render-raster` sends every stroke `thin_segment` admits as
one `Command::Line`, which raster draws from its exact area in the shader — any direction, under a
transform that keeps the axes — with no outline behind it, 60 B an instance and 64 B of pre-count.
The witness's device frame at the fit went from 9.7 s to 1.2 s on both lanes, and the generator is
drawn at chop 24 (12 185 819 commands), where HEAD's resource budget refused chop 12. Still owed: a
scene holding a line is rebuilt at every view (ADR 0702), so a zoom step pays the scene again.

## 3. Tiles from a culled list, on a pool — the multiplier

**Built** (ADR 1841): a zoomed frame is drawn from the marks that reach it, as the window's own row
strips on the pool, the bounds kept with the list. The device's offscreen path takes the culled
list too: what the whole list moved on `bug946506.pdf` was raster's census sending a glyph placed
beyond the page to the atlas, not a pixel of the off-page marks (ADR 1858).

ADR 1817's candidate A: 4.0–4.5× at four times the fit, 2.2× at the fit, and a seam that moves
pixels (588 and 778 on the witness). A tiled processor entry in `render-cpu` is where it attaches;
what this file adds is the order — after section 1, so that what the pool multiplies is the small
cost — and the bin index that makes the cull proportional to the view (the linear one is 64–70 ms).

The toolkit windows do not reach the cull: they ask for the whole page at a zoom. At four times the
fit the whole page is 1 303 ms on four threads, the band of rows the window shows 444 ms and the
window 161 ms; the band needs a host to say it places a raster at a row offset, and the stand-in
and the look to know the band (ADR 1859 section 4). The strip count is the pool's width (ADR 1859).

## 4. The page drawn while it is read — built in `quorra` (ADR 1855)

The read was 76–78 % of the witness's first frame and the window showed a bar until it ended. Now
the interpreter hands its list over in parts as it builds it (`pdf_model::progress::Parts`), page
one is read on a thread of its own once the read has run `GLIMPSE_AFTER`, each part is drawn into
the first frame's target as it arrives (`render_cpu::Underway`), `quorra` shows each look while the
document opens, and the first frame is finished from that drawing — byte for byte the page drawn as
one strip, every corpus first page checked. ADR 1855 has the rule for each page that is drawn whole
instead and the figures. `quorra --cpu` and a page turn after the launch are drawn while read too,
the turn read on the toolkit thread beside a drawer (ADR 1871: a turn's read's end to frame 0.55 →
0.09 s), and the confined worker's turn too (its draw after the read 0.50–0.66 → 0.05–0.08 s); the
confined window's launch is priced and not built (ADR 1871 §4). The two toolkit windows draw page
one while it is read (ADR 1867) and a turn too, the draw after the read 0.51–0.56 → 0.05–0.11 s on
the owner's display (ADR 1886 §3).

## 5. A line kept on the device, the view a uniform — the scene half built (ADRs 1877, 1878)

A scene of lines is kept across views: a segment is a raster line at every view, its width resolved
per placement, and a round dot is kept beside the scene and asked again at each view (ADR 1877); a
run of lines behind a queued mark is one job (ADR 1878), and a line's closed form is made on other
threads ahead of the walk, its instance written where the walk meets it (ADR 1884). The device's
zoom step on the witness went from 912–1 051 ms to 334–467 ms against a replay of 179 ms, and its
encode from 225–231 ms to 118–130 ms; on the owner's display the last of ten notches lands 0.19 s
after its key where it took 0.43–1.02 s. What is left is the walk's ordered share of every line per
view (about 280 instructions a line) and the instances staged again: a line's instance in page space
with the view a uniform the line shader applies would keep the stream resident and make a step cost
the replay (ADR 1877 section 5) — repriced at no more than the staging and transfer, 37 ms of a
304 ms step, because the walk's batches are per view while lines share them with marks whose
coverage is (ADR 1898 §4). The device's memory hint was priced and kept at the default (ADR 1885).
`quorra` builds the device's scene after a processor frame while nobody waits, and an upload no
longer replaces it (ADR 1898).

## 6. One page read on many threads, divided where `Q` restores the state — built (ADR 1880)

The read is now 96–97 % of the witness's first frame (2.48–3.52 s on the owner's display) and one
thread's work; its lexing is 335–342 ms of 1 739–1 929 ms, so lexing on many threads with one
dispatcher is worth at most 1.24×. §8.4.2's `Q` restores the entire graphics state, so a stream cut
after a `Q` that returns to a depth is interpretable piece by piece from that depth's composed
state: `examples/read_split_price.rs` (pdf-model) reads the witness in 236–267 ms on twelve pieces
against 1 828 ms whole, and the joined list equals the page's. The build: a pass on the pieces at
once that records each piece's effect on the state stack, composed in order; each piece interpreted
from its composed state on the window's pool; cuts only where no path, text object, marked-content
sequence, compatibility section or inline image is open; the budgets, reports, caches and ADR
1855's parts kept the page's; every corpus first page that divides byte for byte the page's list.
ADR 1873 §5 is the list in full. **Built** in `content/divide.rs`: what it carries, what still
declines a division (an `/ExtGState`, marked content, an inline image, text before the last piece)
and what the witness reads in are ADR 1880's.

## 7. What §6 leaves serial: the inflate pipelined into it, and the drawer after it (ADR 1888)

Status: **the first two items built (ADR 1894)**, the third ADR 1896's. Instrument: `crates/pdf-model/examples/serial_terms_price.rs`. Once the pieces run at
once, the witness's read is the inflate and a tail: 48 pieces on 12 threads take 394–442 ms after a
163–175 ms inflate and **268–306 ms** behind it, each piece started when the inflate has passed its
end. The draw whole on the pool, 490–525 ms and the same on 12 threads as on 24, is then the first
frame's largest term.

- **Pipeline the inflate — built (ADR 1894)** (§7.4.4.1's Flate, one decoder on a thread of the
  pool, into place): at least four pieces a thread, since the last piece starts when the inflate
  ends. `gs` and marked-content sections that record nothing but whether they hide now go through
  the join as well.
- **Plan without a serial lex**: the lexer reads at 0.71–0.78 GB/s and the inflate writes at
  1.69–1.80 GB/s, so an exact planner behind the inflate is the floor (359–397 ms). Guess each cut
  from a `Q` between delimiters (4.6 GB/s), and let the piece before confirm it: its lex must arrive
  at the cut between tokens, at the depth it summarised, with nothing ADR 1873 §5.2 refuses open. A
  failed guess moves the cut and never the list.
- **The drawer — built (ADR 1896)**: `render_cpu::Underway::in_strips` draws the waiting parts in
  even strips on the pool and finishes the rest there, and the join hands each piece over as it is
  joined. **The strip count** (ADR 1915): what holds the witness's 1 000 rows to 10 strips on 12
  threads and on 24 is the balanced cut's 64-row floor, not the count asked. Under 32 rows the
  planner grants 12 and 16 and the page drawn whole is 6–10 % quicker, but the new cuts move pixels
  by ADR 0219's residual past ADR 1758's bound, so it was taken back. **Next**: the residual itself,
  which a mark converted row by row does not have (ADR 1819); a smaller floor costs what is left.
- **The read's memory — built (ADR 1924)**: a segment is let go once its piece has read it, a spine
  states copies of its statements, and the join copies each piece once. The read's peak is gone,
  and the witness's heap peaks in the draw.
- **The planner's memory — built (ADR 1940)**: the strip planner borrows the bounds the list keeps
  and states only the extents that are not their command's bound, and a band's keys are 32 bits;
  the peak is 1 266.5 MiB, the list's own commands, paths and bounds. **Next**: the list itself —
  a command is 120 bytes and its path an `Arc` of its own.
- **Not taken**: dividing the inflate itself. The witness's one stream has no restart point
  (`00 00 FF FF` count 0), so dividing it means rapidgzip's speculative decoder in-tree, for at most
  42–45 ms of the read.
