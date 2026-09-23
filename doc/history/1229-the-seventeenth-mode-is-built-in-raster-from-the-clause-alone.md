# 1229 — The seventeenth blend mode is built in `raster/`, from the clause alone

2026-09-23. ADR 1295. Acts on `doc/questions/A76`; closes `doc/QUORRA_FEEDBACK.md` section 49.

## What moved

No ledger status: §11.7.4.3 was `implemented`. Its note, `code` and `test` now say `render-raster`
draws the mode and `render-gpu` refuses it; §11.7.4 and §11.7.4.5 point at the new test. `raster_scene::Compose` gains `DestOver` and `DestOverIn([bool; 3])`, plus `Compose::keeping`. The
builder refuses the mode in three positions as `SceneError::OverprintComposeUnsupported`. In
`raster-gpu`, `DestOver` is a fixed-function blend state in every lane. `DestOverIn` goes through a
layer, and `composite.wgsl` composites it with §11.3.6's formula, choosing `B` per channel. In
`render-raster` the by-name refusal and `tests/overprint_refusal.rs` are deleted: fills carry the
operator, and a stroke or an image goes in a group of one.

## Independence

This round did not open, grep or read anything under `crates/render-cpu/`. The arithmetic and every
expected pixel in `raster-gpu/tests/overprint_compose.rs` and `render-raster/tests/overprint.rs`
come from §11.3.3, §11.3.6, §11.3.7.3 and §11.7.4.3 with Table 146. `render-cpu` enters only as a
black box in `the_two_backends_meet_on_the_mode`, which is the first meeting.

## Findings

**The two readings agree.** On hand-built lists they differ by at most 2 of 255. On the crawl, 33
of 40 overprinting witness pages compare now, and every one of them was refused before. One page
has a worst tile of 6.42 at 100%. Its difference is confined to anti-aliased edges, the channel
the mark keeps agrees there, and at 200% the worst tile is 1.15. What differs is coverage, and no
sentence of §11.7.4.3 decides coverage. **The §49 witness still does not compare, and the reason is not this mode.** It paints under
`/BM /Multiply`. §11.7.4.3's last paragraph therefore wraps the mark in a non-isolated group, and
raster draws §11.4.4's result step only under Normal. The name stays in `REFUSED_BEFORE_THE_SCENE`
with its new reason.

**In a knockout group the mode is Normal.** Raster's knockout groups are isolated, and §11.3.6 says
"An alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect". An early draft of this
round refused marks there, which kept `PDFBOX-4095-10.pdf` refused for no clause's reason. **Not
this round's.** `corpus.rs`'s differs list fails because `issue269_2.pdf` and `pr12564.pdf`
now agree. Neither page paints under the mode, and the image lane under them is round 1232's. Also,
`fill_through_blend_group` builds a child layer but never calls `unreplayable()`.
