Status: complete
Given: 2026-09-22, in conversation — transcribed by the round
Owes: the mode built in `raster/`, as a round of this project — `raster_scene::Compose` gaining destination-over and a per-channel selection, the WGSL shaders' arithmetic — from Table 146 and §11.3.6 alone, so that `doc/verify.md`'s cross-backend run is the first time the two sides meet; `QUORRA_FEEDBACK.md` §49's ask closed (acting round)

> I agree with your recommendations.

Reading: the word was given for all three open questions at once, in a conversation that
restated each `Q` file's recommendation. For this one, the recommendation of the `Q` file,
adopted as written — **build it in `raster/`, as a round of this project, and keep the
oracle's independence by construction rather than by authorship**: build it from Table 146
and §11.3.6 alone, do not look at `render-cpu`'s `Computed::Overprint` while doing so, and
let `doc/verify.md`'s cross-backend run be the first time the two meet. Together with what
the restatement added and this answer ratified: the independence that protects the
comparison is derivational rather than one of authorship — neither side's output is derived
from the other's — and `render-cpu`'s side is already pinned by ADR 1182's identity test, so
a quorra side built from the clause alone meets it as a genuine second reading; the
precedent is the trees' own, ADRs 0085 and 0086 mirrored statement for statement and held
together by the pixel gates; and the census bounds the work — `Compose::DestOver` alone is
1711 of the 1788 documents, the per-channel selection the remaining 77.