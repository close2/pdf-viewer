# Specification audit of ISO 32000-2 §10.5 to §10.8 (transfer functions, halftones, scan conversion, rendering for separations) — round 2248

The project's nineteenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2243 ([`2237-specification-audit-8-1-8-3-8-8-10-1-10-4.md`](2237-specification-audit-8-1-8-3-8-8-10-1-10-4.md)
is the clause family's first half). Baseline: main at `9ad7f2f1`. Evidence:
[`2248-specification-audit-10-5-10-8/`](2248-specification-audit-10-5-10-8/) — `findings.json`
(2038's schema) and `expected-failures.json` (the failing tests of each behaviour finding). The
same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §10.5 to §10.8 (`doc/md/ISO_32000-2_sponsored_EC3.md` lines 8862
   to 9294) were read with Table 57's `/TR`, `/TR2`, `/HT`, `/FL`, `/SM` and `/SA` rows, §11.7.5.2,
   `CLAUDE.md`'s clause 10 paragraph and `doc/todo/13-the-transfer-function.md`. §10.6's screens are
   inapplicable on the standard's own condition (§10.6.1) and were read only for what a screen still
   owes: §10.5's second bullet, a halftone's `TransferFunction` (ADR 0505), and Type 5's `/Default`.
2. 2038's findings on these clauses were read first and are not repeated (T08, T35 and T37 on
   transfer and stroke-fit assertions, B20 and T126 on degenerate bounding boxes, D13, D30, D54,
   D58, B77, T168). Errata: `doc/errata/site/docs/32000-2-2020/clause10.md` — issue #310 (Tables
   128 to 132's `TransferFunction` and Type 5's `/Default`), #311 (the struck paragraph on
   nonprimary colourants under a non-Type-5 halftone) and #371 (`/FL`'s 0-to-100 range moved into
   §10.7.2); #694 (open, §10.6.5.1's NOTE). None settles a finding below; #311 removes the
   gray-function rule for nonprimary colourants that B03's fix would otherwise have to state.
3. The lens was *every route to a thing*, applied to the finished colour: §10.5 says where a
   transfer function applies — "after performing any needed conversions between colour spaces",
   in "the native colour space of the output device" — so the routes are every device a page is
   drawn for (the screen, and §10.8.3's simulated CMYK press) and every backend route a display
   list takes to the screen: `render-cpu`, `render-raster`'s `rasterize`, the window's present
   (`QuorraWindowRenderer::render`, which `viewer-device` draws every frame through), `render-gpu`
   and the confined crossing. And *a bound has two sides*, applied to §10.7.5's fit.
4. Each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/rendering_10_5_10_8_spec_audit.rs` (a module of `content_suite`, CPU oracle)
   and `render-raster/tests/window_frame_10_5_10_8_spec_audit.rs` (the RX 580, under the lock the
   code takes). The viewer-device route to the present was read by a Sonnet helper and its one
   load-bearing call (`renderer/device.rs`'s `renderer.render(frame, into)`) checked here.

What holds, probed by reading or by the probes: Table 57's `/TR2` over `/TR`, `/Identity` and
`/Default` (the start-of-page transfer, the screen's identity) as a third answer beside "unsaid";
one function applied to every component; a halftone's `TransferFunction` overriding per component,
a Type 5's `/Default` taken by a component with no entry; the function evaluated on 0..1 and its
output clamped to 0..1; the map applied once over the finished raster, by §11.7.5.2's topmost
fully opaque elementary object, with no mark inside a soft mask's group choosing or occluding;
`render-cpu` and `render-raster`'s `rasterize` running the one `resolve_transfers`; `render-gpu`
and the confined crossing refusing such a list by name. §10.7.2's flatness ignored under its own
permission; §10.7.3's `/SM` honoured toward finer sampling under the internal-limit sentence;
§10.7.4's departures (ADRs 0025, 1082, 1560, 2330) as the ledger states them; §10.7.5's second
requirement (under half a pixel, one device pixel) in every backend; §10.8.1 and §10.8.2 as the
ledger reads them.

## Findings

Five behaviour gaps. In order of how much a person would see:

- **2248-B01** — the window draws a page without its transfer functions. The channel
  (`DisplayList::transfers`) is applied by `rasterize` and never by the window's present, and
  nothing on `viewer-device`'s route refuses the page or sends it to the processor; `render-gpu`
  refuses it by name and the shipping window lane neither applies nor refuses. A red fill under
  `/TR` = 1 − x is cyan through `rasterize` and red in the window's frame; by the same route `issue6931_reduced.pdf`,
  the clause's corpus witness, would show the black square its own text says is wrong (not run).
- **2248-B02** — the window draws §10.8.3's simulated press without its black ink: the second list
  (`DisplayList::black`) and the spot planes `rasterize` draws beside the page and multiplies back
  are dropped by the present. `0 0 0 1 k` under the simulation is dark through `rasterize` and white
  in the window. By reading, a four-component page group's black list (§11.4.7) is dropped the same
  way without the simulation.
- **2248-B03** — on the simulated press, whose device is CMYK, the transfer function is applied as
  an RGB device's: the fourth function of Table 57's array is never read, and the first three map
  the finished sRGB pixel after step d) instead of each subtractive component in additive form.
- **2248-B04** — on the simulated press a `DeviceGray` colour goes through the red, green and blue
  functions, where §10.5's special case gives it the gray function alone and renders cyan, magenta
  and yellow directly.
- **2248-B05** — `/SA true` fits only strokes under 1.5 device pixels (`grid_fit::FIT_BELOW`). The
  clause's first requirement binds every stroke `/SA` enables, and under ADR 2530's own reading of
  uniform thickness a two-point rule covers three rows at one placement and two at another.

One documentation finding, **D01**, left for the fix rounds of B01 and B03: three comments state
the screen as the only device, or the shipping backends as applying the channel.

## Ledger rows contradicted

Each stays as it is until the fix round, which re-reads it: `10.5` (B01, B03, B04) and `10.6.1`
(`inapplicable`, whose note states the transfer a screen still owes; B01), all others `implemented`: `11.7.5.2` (B01), `10.8.3` (B02, B03, B04), `10.7.5` (B05). `10.7.4` stays
`departed`; nothing here touches its departures.

## Suggested fix groups

1. **The window's present route** (`render-raster/src/present.rs`'s `scene_for` and
   `FrameSlot::render`, or `viewer-device`'s lane choice) — B01 and B02, and D01's last sentence:
   either draw the channel and the planes beside the page on the present as `rasterize` does over
   its read-back, or refuse a list carrying `transfers()`, `black()` or `separation()` there by name
   so the processor draws it, reported. One decision for all three; an ADR either way.
2. **§10.5 on the simulated device** (`pdf-colour/src/transfer.rs`, `pdf-model/src/content/marked.rs`,
   `pdf-render/src/transfer_channel.rs`, the separation in `content.rs`) — B03, B04 and D01's first
   two sentences: four functions kept, applied per process and spot plane in additive form before
   steps b) to d), with the `DeviceGray` special case; the screen unchanged.
3. **§10.7.5 at every width** (`pdf-render/src/grid_fit.rs` and `raster-gpu`'s copy, in every lane
   at once as A714 asked) — B05: width rounded to whole device pixels, edges on pixel boundaries;
   the module comment, ADR 2530's domain and the 10.7.5 ledger note corrected.
