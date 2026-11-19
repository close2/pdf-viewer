# 3083 — The raster golden's 63 moved pages, restated after the shading's `anti_alias` field and the drawn face's advance

Session 2095. Status: **accepted.** Context: ADR 1016 (the golden), ADR 3051 §1 (the method: build the commit before
and the commit after on this machine, never splice a line), ADR 3024 (round 2083), ADR 3046 (round 2087), ADR 3074
(round 2093), `crates/pdf-model/tests/raster_golden.rs`. Clauses: ISO 32000-2 §8.7.4.3 (Table 77), §9.2.4, §9.6.2.1
(Table 109).

## 1. The method

The train at `594ae80f` held 911 pages and moved 63. Each commit from `5adc3d83` (main before round 2083) to `594ae80f`
that changes a crate in `pdf-model`'s dependency graph was built on this machine, and its golden written by the walk's
own update mode into a scratch copy: `5adc3d83`, `66c33750`, `fb3699c3`, `5506296e`, `8a995579`, `fa29fe4c`,
`5fac2b49`, `3b78a396`, and `594ae80f` in the worktree. `122d9952` was not built: it touches viewer crates only, and
`cargo tree -p pdf-model -e normal,build,dev` names none of them. A scratch example planted only in the scratch copies
wrote each of the 63 first pages' display list and reports as `{:#?}` text and its raster as bytes, at `5adc3d83`,
`66c33750` and `fb3699c3`; its digests equal that commit's golden line for every page, so the dumps are the golden's
own measurements spelled out.

At `5adc3d83` all 63 lines are the golden's own: the file held them, and the only pages that differed there are the
fifteen font pages `fa29fe4c` restated. Each of the 63 moves at exactly one commit and at no other.

## 2. The classes

| commit | pages | layer | what the dumps show |
|---|---|---|---|
| `66c33750` (ADR 3024) | 55 | list | every added line is `anti_alias: false,`; no line removed; raster bytes and reports identical |
| `66c33750` (ADR 3024) | `issue17065`, `issue18529` | raster, list | the added lines are `anti_alias` fields, three and one of them `true`, each file's own axial (type 2) shading's `/AntiAlias true`; no line removed; raster deltas of at most 2 and 1 levels in 14.6 % and 5.4 % of the pixels, inside the shaded area only, in columns across the ramp: the mean of sixteen sub-pixel samples of a smooth ramp rounding apart from the centre sample |
| `fb3699c3` (ADR 3046) | `issue18099_reduced`, `issue2391-2`, `issue6069`, `issue6106`, `issue6108` | reports | one report's wording: "every other code by Table 120's /MissingWidth" becomes "107 by the drawn face's own glyph or its .notdef, since no font descriptor states a /MissingWidth"; list and raster identical |
| `fb3699c3` (ADR 3046) | `poppler-90-0-fuzzed` | raster, list | 764 glyph translations move right by 0.3586, which is 250/1000 of the 1.43462 glyph scale: one code outside a descriptor-less Times font's `/FirstChar 1 /LastChar 255` (code 0, the only one) advanced 0 and now advances by the drawn face's glyph, §9.2.4. The raster changes only in the page's last row, 553 pixels |
| `594ae80f` (ADR 3074) | none | — | the golden at `3b78a396` and at `594ae80f` is identical over all 974 pages, so the stroke's field moved no first page |

ADR 3024 said "No golden moves". That held of the pixels of every page without the entry and of none of the lists:
the field is in the list's `Debug` text, which the golden digests, so every first page holding a shading moved its list.

## 3. The decision

All 63 are explained by a deliberate change and nothing else, so the update mode rewrote the file and the diff is
exactly those 63 lines; no line was written by hand. `issue19517` moved at `fa29fe4c` and returned at `5fac2b49`; it is
held at `594ae80f` and is not among the 63.
