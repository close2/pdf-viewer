# 1251 — An imported page is its own page group, and shape has one reader

Batch thirty-eight. Rows §11.4.3, §11.4.7 and their aggregate §11.4, all `partial` → `implemented`.

## §11.4.7 (ADR 1339)

The clause's second treatment: an imported page "shall be treated as a transparency group using
the page Group attributes dictionary" — the *imported* page's own (the brief had it as the
containing page's; the row's note had it right). The tree imported the page (ADR 1101) but ran its
marks inline in the containing page's compositing, and read the proxy's `/Group` after swapping
the target document in, which resolves an indirect `/Group` in the wrong file.

Built: the imported page is always a group — its `/Group`, or Table 145's defaults — and the
proxy's §8.10.4.1 group, where stated, holds it as its one element; each dictionary is read in its
own document. A group's content is now `GroupBody` (a stream, an imported page, or that page's
content and annotations), so the reruns a group makes reach an imported page too. Four fixtures in
`tests/reference_xobjects.rs`, each calibrated by planting the old route.

## §11.4.3 (ADR 1340)

A reading. The pair is read apart only by §11.4.6 ("The separate shape value shall be computed in
any group that is subsequently used as an element of a knockout group"); with b = i − 1 §11.4.8's
shape terms cancel, §11.5.2 and Table 141 read alpha, and `/AIS` leaves the product unchanged.
Every knockout route states the shape (ADRs 0234, 1205, 1279, 1319); §11.3.7.3's NOTE 2 licenses
the product elsewhere. One fixture holds a nested group whose opacity is its content's, isolated
and not, calibrated by planting the group's raster as its shape.

## Gates

Tier 1 on pdf-model: fmt, clippy pedantic `-D warnings`, nextest 1665/1665. `cargo test -p
conformance`: my rows, quotations and the frontier map pass; one failure is §10.7.4's
`render-cpu/src/area.rs` test name, a sibling's in-flight row. Tier 2 under the lock:
`render-raster --test corpus` and `pdf-model --test corpus` exit 0; `raster_golden` exit 101 with
248 pages moved, every one "raster only (a rasteriser change under an unchanged list)" — no display
list moved, so none is this round's (it changes interpretation only).
