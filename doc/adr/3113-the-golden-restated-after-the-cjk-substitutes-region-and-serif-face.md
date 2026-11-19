# 3113 — The raster golden's 11 moved pages, restated after the CJK substitute's region and serif face

Session 2103. Status: **accepted.** Context: ADR 1016 (the golden), ADR 3051 §1 (the method: build the commit before
and the commit after on this machine, never splice a line), ADR 3083 (the last restatement), ADRs 3094 and 3095
(round 2098), ADRs 3101, 3102 and 3103 (round 2100), ADRs 0155 and 0165 (a box clip that cuts nothing comes off),
`crates/pdf-model/tests/raster_golden.rs`. Clauses: ISO 32000-2 §9.5, §9.7.4 (CIDFonts), §9.8.2 (Table 121's Serif flag).

## 1. The method

The golden at `f3597ae5` is ADR 3083's, written at `594ae80f`. Of the code commits after `594ae80f` on main, two change a
crate in `pdf-model`'s dependency graph: `b60382af` (round 2098: `pdf-model`, `render-cpu`, `render-gpu`,
`render-raster`, `test-scenes`) and `9095a81e` (round 2100: `pdf-font`). `720e1e1b` changes `pdf-model`'s test targets
only and `59c77c0b` changes `pdf-transform`, which `cargo tree -p pdf-model -e normal,build,dev` does not name.

`b60382af` was built in a scratch copy and its golden written by the walk's update mode: it equals the file at
`f3597ae5` on all 974 pages, so round 2098's washed stroke and mesh stroke moved no first page. The check walk at
`9095a81e` held 963 and moved 11, every one "raster and list". Round 2095's scratch dump (each first page's list and
reports as `{:#?}` text, its raster as bytes) was built at `59c77c0b` and `9095a81e`; at `59c77c0b` its digests equal
the golden's on all 11.

## 2. The classes

| pages | what the dumps show |
|---|---|
| `90ms_rksj_h_sample`, `issue11555`, `issue13343`, `issue15053`, `issue6286`, `noembed-identity`, `noembed-jis7` | with path geometry (points and path verbs) set aside the lists are equal line for line, and the reports are equal: only glyph outlines changed, the Mincho drawn by Noto Serif CJK JP (ADR 3102) |
| `issue19182`, `issue2128r`, `issue3521` | the same, the face now the font's region's (ADR 3101): `issue19182` in TC serif (`PMingLiU`), `issue2128r` in SC, `issue3521` in SC serif |
| `issue14438` | 5 of 4 305 items change, all five glyphs of one unembedded `STSong` run ("title", 12 pt): their outlines are Noto Serif CJK SC's (ADR 3102: a Song is serif), and each loses its box clip, `ClipId(2)` becoming `None`: the serif outlines lie inside the box, so ADR 0155's rule takes off a clip that cuts nothing. Transforms, paints and every other item are equal; 135 pixels in one 20 by 10 region differ |

Round 2100's list named ten pages; `issue14438` is the eleventh, a Song it did not render.

`issue3521`'s font names itself `黑体` (Heiti, a sans) in GBK bytes that this program does not read as a name, and its
descriptor states `/Flags 6`, Serif among them; Table 121's flag is what decides the face today, so the page is drawn
serif as the file states. Reading a `/BaseFont` through its CMap's encoding is round 2100's first item left.

## 3. The decision

All 11 are explained by `9095a81e` and nothing else, so the update mode rewrote the file and its diff is those 11
lines; no line was written by hand.

The first update run, at a load average near 200, moved a twelfth page: `issue19517`, "list only, and the reports",
to the value `6f409240f437a975 0f7448358dc8a387` that round 2095's series also drew once, at `fa29fe4c`. The check run
after it drew the file's own value back, and four dumps of the page at `9095a81e` outside a walk are identical to each
other. Nothing in `9095a81e` explains that line, so it was not restated: the update ran again, the page drew the
file's value, and the diff is the 11. The page holds a JPEG 2000 image, and `pdf-sandbox` kills a decode at thirty
seconds of wall clock (`REQUEST_TIMEOUT`): the likely cause, not yet shown by the variant's own reports, is a decode
that overruns the deadline at `nice 19` under load, and a list that follows a deadline follows the load. The same run's nextest failed `thumbnails` on `issue19517` and `matte_reduced`'s
reduced-level decode, both passing alone in 12 s. That is a finding for a later round, not a line of this file.
