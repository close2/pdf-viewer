# 1561 — A movie's poster is fitted into its rectangle, and the playing is the departure

Status: **accepted**. Builds what `doc/questions/A33` approved and ADR 1548 left owed; moves §13.4
from `reported` to `departed`. Context: ISO 32000-2 §12.5.6.17 (Table 189), §13.4 (Tables 306 and
307), §12.5.6.19 and §12.7.8.3.2 (Table 250), §8.9.5; `doc/questions/A33`, `doc/questions/A72`;
ADRs 0906, 1299, 1548; `crates/pdf-model/src/appearance.rs`'s `movie_poster`.

## 1. What is built

A `Movie` annotation with no appearance stream is constructed from Table 189's required `/Movie`
and that dictionary's Table 306 `/Poster`, each of whose values the table states an outcome for:

- **a stream** — "it shall contain an image XObject (see 8.9, "Images") to be displayed as the
  poster" — is painted with `Do`, through the same `Stream::form` a push-button's Table 192 icon
  uses, inside `/Rect`;
- **`false`, or no entry** (the default) — "no poster shall be displayed" — draws nothing and
  owes nothing;
- **`true`** — "the poster image shall be retrieved from the movie file" — is refused and reported:
  a frame decoded out of a movie file is the media engine `CLAUDE.md` principle 5's clause 13
  exclusion names, which A33 left excluded.

A stream that is not an image `XObject`, an image with no positive `/Width` and `/Height`, and an
annotation with no movie dictionary are each refused by name.

## 2. The choice: where in `/Rect` the image goes

Neither table states it. Table 307's `/FWScale` says that without it "the movie shall be played in
the annotation rectangle", which puts the poster *in* `/Rect` and says nothing of its size there.
§8.9.5 paints every image on the unit square, so an image has no size of its own in user space —
only an aspect ratio, its `/Width` to its `/Height`. Three readings were weighed:

| reading | what it does | why not / why |
|---|---|---|
| the unit square mapped onto `/Rect` | fills the rectangle | distorts any poster whose shape is not the rectangle's — a shape the file never stated |
| `/Aspect`'s pixels as user-space units | a "native" size | a pixel is not a unit, and `/Aspect` describes the movie being played, not the poster |
| **Table 250's defaults** | scaled proportionally until it meets the width or the height, centred | **chosen**: the standard's own default for fitting an image-like mark into an annotation rectangle, and it keeps the image's shape |

So `IconFit::DEFAULT` places the image — `/SW A`, `/S P`, `/A [0.5 0.5]` — fitted into `/Rect`
itself, since a movie annotation draws no border to inset by. This is a placement the clause
withholds and the program supplies, so under A72's rule the report says so — `a placement this
program chose` — as ADR 1299's caption share does. Table 306's `/Rotate` and `/Aspect` are not read:
both state how the movie is played, and the poster is an image the producer wrote for the page.

## 3. The status, and what the departure costs

What is withheld is the playing — "When the annotation is activated, the movie shall be played"
(§12.5.6.17), Table 307's activation and Table 306's boolean `true` — on the clause 13 exclusion as
A33 bounded it; everything a still page can show is drawn. That is `departed`, not `out-of-scope`
and not `partial`: one named withholding, decided against, nothing owed. The cost: a movie whose
producer relied on the poster being taken from the movie file shows nothing on the page and says
why. §12.5.6.17's own row stays `out-of-scope`, since its one `shall` on a processor is the playing.
Fixtures: `crates/pdf-model/tests/movie_poster.rs`, one per value of `/Poster` and one for a stream
that is not an image.
