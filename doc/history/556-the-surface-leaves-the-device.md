# 556 — The surface leaves the device, and the clock finally gets a picture

2026-08-17. The round the previous eight were building toward: quorra answered
`doc/QUORRA_NONBLOCKING_RENDER.md` with **yes**, and this session was its caller. ADR 0391.

## The upstream range

`a4380e2c` → `eada81ec`, five commits, `cargo update -p quorra-gpu -p quorra-scene`.

| | |
|---|---|
| `44d7acf` | their release matrix; nothing in `src/` |
| `3073c7e` | `recording` subdivided with callgrind — the answer to our §9. No `src/` change |
| `bf5044e` | **the surface leaves the device** (their ADR 0056): `Presenter`, `Layer`, `PresentCost`, `detach_presenter`/`attach_presenter`, `present.wgsl`, three new `RenderError` variants |
| `aead796` | `examples/present_thread/` under Xvfb in their CI, verified able to fail three ways |
| `eada81e` | the reply to carry across |

**The bump itself cost no line.** The API change is purely additive — nothing removed, renamed or
resignatured; `quorra-scene` untouched — and `check --workspace --all-targets` passed against the
new pin before a single edit. `doc/QUORRA_UPGRADE.md` has the section.

## What was built

A render thread owns the device; the event thread owns the presenter. A job crosses one way, a
finished frame the other, and the texture pair travels back and forth so no frame allocates a
window's pixels twice. Three layers a present: the medium (one opaque texel scaled over the window),
the page under `settled⁻¹ ∘ asked`, the chrome at the identity.

- **`render-quorra`**: `QuorraPresenter` → `QuorraWindowRenderer`, which no longer presents.
  `render(frame, WindowTextures)` draws into two host textures; `layer_texture`, `medium_texture`,
  `detach_presenter`, `attach_presenter`. Two scene lanes, one per texture, because two scenes
  sharing one resource cache would evict each other's outlines. `Captured` and the whole readback
  path deleted.
- **`viewer-ui/src/bin/pdf-viewer/renderer.rs`**, new: the thread, the channels, the pool, the
  three-layer present.
- **`stale.rs`**: the base is the rendering's own texture; `reproject` hands back a *placement* and
  no pixels; rule 5 gains an *observation* beside its prediction; rule 4 and `Refusal::TooDear`,
  `Refusal::NoPixels`, `Refusal::NoDevice`, `Stale::measured` and `Base` are deleted.
- **`surface.rs`**: one tick is now *adopt, ask, place*.

## The measurement, on the owner's own 120 Hz display

`tmp/Entwurf.pdf`, 58 009 commands, 1275×1594, RADV, `120.0 Hz stated by the surface`.

| | session 549 | paced (1.5 s apart) | held (0.12 s apart) |
|---|---:|---:|---:|
| presents | 24 | **533** | **309** |
| median interval | **167.4 ms** | **8.4 ms** | **8.3 ms** |
| on the next refresh | 4.3 % | **88.0 %** | **94.2 %** |
| a rendering | 17 (70.8 %) | 17 (3.2 %) | 3 (1.0 %) |
| refusals | 2 | **0** | **0** |

A present costs **0.51 ms** at the median (a reprojection) and 0.23 (a rendering put up), against
6.2–16.2 ms for the same thing with a readback in it. The launch path did not move: `graphics
device` 32.3 ms against 32.7.

**3.2 % correct is the ceiling being met, not missed.** quorra's §9 computed 2.9 % from the frame's
own cost with `encode` at zero. The other 516 refreshes used to get nothing.

## Two things this round decided rather than inherited

- **Rule 4 is deleted by argument.** Its premise — a reprojection delays the real frame by what it
  costs — is false once the render is on another thread. A bound on a structurally zero cost is not
  a bound. ADR 0391 §4.
- **`Plan::Render` no longer answers for a view already approximated.** That was rule 1's old
  mechanism, and under the split it would put the *old* view up unmoved, which is a jump backwards.
  Caught by reading the two against each other rather than by a gate; the arrangement re-presents
  the same three quads instead, which is what a picture every refresh means.

## Gates

`fmt --check` clean; `clippy --workspace --all-targets` silent; **2038 tests run, 2038 passed, 15
skipped** (2040 before: seven tests deleted with the readback route, five added for what replaced
it); doctests pass; conformance passes; the corpus gate, the oracle, both text gates and the
sandbox and hayro builds all pass. The three quorra lanes on the real Radeon:

| | `a4380e2c` | `eada81ec` |
|---|---|---|
| scale 1, `cpu` | 931 / 23 / 2 / 18 | **931 / 23 / 2 / 18** |
| scale 1, `gpu` | 929 / 25 / 2 / 18 | **929 / 25 / 2 / 18** |
| scale 4, `gpu` | 937 / 10 / 4 / 23 | **937 / 10 / 4 / 23** |

Every judged line character-identical, which is what `doc/todo/37` rule 2 needs and what the
`Option<Color>` medium in `build` had to not disturb.

## For the next round

- **`PresentCost::reconfigured` is the thread to pull if the tail matters.** The present's p90 of
  6.10 ms is an acquire waiting for the presentation engine, and that field is where a present
  unlike its neighbours declares itself. Nothing here reads it yet.
- **The chrome is now exactly as stale as the page during a stand-in**, where it used to be redrawn
  per reprojection. Written down as a cost in ADR 0391 §7 rather than discovered later.
- **`doc/todo/37`'s processor path inherits a changed premise**: rule 4 is gone because a stand-in
  costs the render nothing, and a resample *on the processor* would not be free. A round that
  builds that path owes an argument for whatever it puts back.
