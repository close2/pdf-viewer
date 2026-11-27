# 3820 — Rich media's `Sound` and `Video` configurations follow the players, and are planned

Session 2279. Status: **accepted**; planned in
`doc/todo/80-rich-media-sound-and-video-and-a-movie-played-backward.md`, not built. Clauses: ISO
32000-2 §13.7, Tables 334 to 344. Finding: 2278-D03. Builds on RFC 0015 section 10.1, A946, A1006.

## Context

The rich media rows were `out-of-scope` on "a media engine's runtime — rich media's, which no round
has planned beside the video player". The ratification (`doc/scope-ratifications.md`) says rich
media follows the parts it is made of and carries no trigger of its own, and RFC 0015 section 10.1
makes a `Sound` configuration Part 1 (audio, built) and a `Video` configuration Part 2 (video, built
by A1006). Only the `3D` configuration stays excluded, with U3D and PRC rendering.

## Decision

1. **The reason is restated, the status kept.** The rows stay `out-of-scope` until a round builds them
   (the ratification's rule) — for the `Sound` and `Video` configurations that is now a plan, not a
   trigger; for `3D` instances and `/Scripts`, the exclusions of U3D and PRC rendering and of
   ISO 21757-1's 3D API.
2. **They do not fall out of the players directly.** The players take a clip and a rectangle; rich
   media needs §13.7.2.2's activation and deactivation (Tables 335, 336), Table 338's presentation
   (`Embedded` required, `Windowed` within the presentation area), Table 342's configuration choice
   and Table 343's instance-to-asset match, and Table 337's animation. RFC 0015 section 10.1 prices
   these at five rounds; the todo orders them.
