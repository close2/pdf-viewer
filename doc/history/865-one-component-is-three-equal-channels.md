# 865 — One component is three equal channels: `DeviceGray` blending spaces are drawn

Date: 2026-09-01.
ADR: [0790](../adr/0790-a-one-component-blending-space-is-three-equal-channels.md).
Touched: `crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/tests/transparency_groups.rs`, `doc/conformance/ledger.toml`,
`doc/todo/23-transparency-departures.md`, `doc/todo/01-ledger-partial-rows.md`.

## Round 864, gated and committed

The machine was hard-rebooted before 864 could commit. Its eleven files were gated whole here —
the full `doc/todo/02` §2 sequence — and needed two minimal fixes on the way: `rustfmt` wanted
`subsections_from`'s signature on one line, and the quotation gate attributed the new test's
blockquote to §7.3.10 because that was the last clause the sentence before it cited, where the
words are §7.5.4's. Nothing in 864's argument moved; its commit message says so.

## The spec track: §11.3.4's one-component row

The blame list's top row since 864, with a debt named in the row itself: `DeviceGray`, `CalGray`
and `ICCBased` 'GRAY' reported by name as blending colour spaces, and `ICCBased` 'CMYK'.

Reading the clause for one component found what ADR 0262 found for four: the compositing formula
is per component, so a one-component space composites one number per pixel, and a raster holding
that number in each of three channels runs the arithmetic three times over. §11.3.5.3 says it of
the non-separable modes outright, and each of its four functions returns a grey for two greys. So
`DeviceGray` is one interpretation under a fourth `Compositing` — every colour becomes its
§10.4.2.2 or §10.4.2.3 grey on the way in, through the function the mask route already uses — and
nothing converts out, because §10.4.2.2's conversion out is the identity on three equal channels.
A `/DeviceGray` page group and an isolated `/DeviceGray` group on a device page are both drawn;
a group inside either that changes the space with something compositing in it sends the page or
group back to the device and to its report.

**What the reading found beside the code is the more general thing.** Both departure reports
fired only where something composites, on an argument — an opaque `Normal` mark carries its
colour through any space — that holds for a conversion with an inverse and not for one component,
where a red mark painted into a `CalGray` group is a grey with nothing compositing. A condition
derived for four components had been inherited by one; trap 11's fifth shape. `Departure` carries
the component count now and the reports fire on every mark for one component.

**And the four programs disagree, in a way the clause ranks.** On the fixture, `mupdf` and
`ghostscript` agree to within one level and put a pure red at 129 of 255 where §10.4.2.2 puts it
at 77 — sRGB's linear-light luminance, which is §10.4.2.1's *should* for an ICC-enabled processor
against the classic *may* this tree takes; `poppler` ignores the space. The classic route is kept
for trap 6's reason, one conversion for §11.6.6's one sentence, and the move to §10.3's route is
written down in `doc/todo/23` as one decision for the masks and the blending spaces together.

## The demand-driven half

A grep of the whole crawl — bytes only, one low-priority process — found four documents naming a
`/DeviceGray` group, and none of them paints one on a page: three are `/Luminosity` masks'
groups, one is non-isolated. ADR 0272's six one-component page groups are in object streams
where a grep cannot see them, and the census that could is a corpus walk this round was not to
run. The page looked at is the fixture's, in four renderers, which is where the table above came
from. `bug1721218_reduced.pdf`, the corpus page the ledger used as the example of a grey group
inside a press, is byte-identical before and after.

## Gates

The full sequence, twice — once for 864's tree and once for this one — and §5's binaries
rebuilt and installed. Figures are in the runs rather than here.
