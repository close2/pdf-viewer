# ADR 1113 — The knockout's shape is stated, not channelled, and the flag reinterprets two inputs

Status: accepted, 2026-09-15. Session 1099. Two decisions about ISO 32000-2's separation of shape
from opacity: **not** to add a per-pixel shape channel, and to stop refusing a group on a record
that says the flag was stated rather than that it decided anything. Files:
`crates/pdf-model/src/content/transparency.rs`, `content/{path,pattern,text}.rs`, `content.rs`,
`tests/transparency_groups.rs`, `examples/group_shape_census.rs`, five ledger rows.

## 1. The channel is not owed, and the clause says who would read it

§11.3.7.1 makes alpha the product of shape and opacity, and every formula of §11.3.3 and §11.3.6
reads `α_s` alone. §11.4.6 names itself the exception:

> The existence of the knockout feature is the main reason for maintaining a separate shape value
> rather than only a single alpha that combines shape and opacity.

ADR 1009 proved the rest of the sweep — §11.3.7.3's result opacity re-enters as `α_r`, §11.5.3
reads a luminosity of the product — so §11.4.6 is the *only* reader, and it reads a **stated**
shape: `pdf_render::Command::Shaped` (ADR 0234), the element beside the same element with every
source of opacity removed, drawn as Destination-Out then Plus. That is already a per-pixel `f`,
materialised per element and only inside a knockout group, which is exactly what a channel would
have bought — at one raster per element that is open rather than a second channel every command,
every backend, both censuses and `viewer-confined`'s wire carry for the life of every display list
(ADR 1022 §7 priced that shape and declined it; this round re-took the price and declines it again).
**So no channel was added and none is owed by these rows.** §11.7.5.2's transfer identity is a
different quantity with the same shape of cost, and it stays named rather than built: its
population is still zero (`examples/transfer_function_census`), and `doc/todo/13` §3 carries the
derivation, including the clause's "nonzero object shape value" putting an antialiased edge pixel
*inside* the topmost object.

## 2. The census, measured before deciding (trap 8)

`examples/group_shape_census` gained §11.4.6's half — knockout groups, those holding a
`Command::Shaped`, those on the group's own backdrop, and the refusals by reason. Over
`doc/pdf.js`'s 963 opened first pages: **33 knockout groups on 18 pages, 5 holding a stated shape
on 5 pages, 0 on the group's own backdrop, 0 refusals.** Over `doc/corpora`'s 487: 6 on 5 pages,
0 and 0. **Calibrated** by planting `stated_shape`'s image arm away: the sweep then names
`knockout_groups_test.pdf` as "a stencil under its own /SMask" and 33 knockout groups become 31.
A crawl-wide run was stopped by the coordinator for a `pdf-sandbox` zombie leak and its partial
output is not summable; what it does show is that route 3 — `isolated: false` beside `knockout`,
which `render-gpu` and `render-raster` refuse by name — is reached in the wild where the tracked
corpora reach it zero times.

## 3. The flag reinterprets two inputs, so `Mixed` is not a refusal by itself

`/AIS` is a graphics state parameter, so one group's run may paint under both readings, which
`AlphaSourcesSeen::Mixed` records — and every reader took that as "no reading describes this
group". It does not. §11.6.4.3 and §11.6.4.4 give the flag exactly two things to reinterpret, the
soft mask and the two alpha constants, and §11.6.4.2 fixes the third:

> All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere.

So a group that states neither has one shape per element under **both** readings and is described
by both, where the guard described it by neither. `AlphaSourcesSeen::settled_over` asks the
elements, and the predicate it asks is `group_alpha_is_shape`'s own under `AlphaSource::Opacity` —
element by element, at every depth — so the two questions are one question. The answer is
`Opacity` because Table 57 makes it the default and because it is the cheaper of two equal display
lists. This is ADR 0554's shape one level down: a blanket guard in front of a construction that
had already narrowed.

**Cost, `callgrind_interpret`, 20 interpretations.** `Option::or_else` is not evaluated where the
record is settled, so a page stating no `/AIS` cannot pay: ISO 32000-2 page 101 reads
1 232 459 681 → 1 232 459 426 (−0.00002%, flat). `issue18032.pdf` page 1, which states the entry,
reads 1 230 783 187 → 1 230 473 434 — **0.025% cheaper**, because a group that now resolves takes
the bare knockout construction instead of falling through to the report.

**The fixture** is `transparency_groups.rs::ais_stated_both_ways_refuses_nothing_the_flag_reinterprets_nothing_in`,
and its numbers are §11.4.6's over §11.4.5's transparent initial backdrop with §11.3.6's "[a]n
alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect": the overlap is `(0, 0, 255)`
where the flat drawing gives `(0, 0, 0)`. Planted back to `settled`, the group is refused, the
report names `/AIS` and the overlap reads `(0, 0, 0)` — measured, not predicted. Its control is a
group whose lower fill states `ca 0.5`, an input the flag *does* reinterpret, which must stay
refused.

## 4. What is left, narrowed rather than moved

§11.7.4.4's row said `partial` for "a shading that is not opaque, an image whose samples state
either shape or opacity", and **both had stopped being refused** in sessions 997 and 1002 (ADRs
1017, 1022): `opaque_paint` answers `Shading::opaque`, and `shape_without_the_mask_and_the_constants`
answers the image for `SampleAlpha::Shape` and the unit square under its transform for
`Opacity`. §11.3.7's note carried the same decayed sentence. Both are corrected. What genuinely
remains, and is now all that any of these rows claims: `SampleAlpha::Both`; a non-isolated group
used as an element, whose accumulated alpha carries its backdrop's; route 3 on two backends; a
group the flag *does* split, which needs the reading recorded per element rather than per group
run; and §11.7.5.2's transfer identity.
