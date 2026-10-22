# 1697 — Fresh zoom evidence precedes display attribution

Host instrument/integration round, batch107, 2026-10-22; ADRs2228–2229.
The user's ISO document remains the primary Qt lag/GTK blur-and-drift witness.
A reusable parser rejects stale/ambiguous callbacks at a pre-input byte boundary,
separating host receipt, core command and synchronous Qt accumulator evidence.
A finite driver owns one viewer/window, validates focus ancestry and page readiness,
and tests two calibration points against an independently requested third point.
Toolkit logical-to-core scale stays separate from observed XTest-to-core mapping.

**Gates.** Parser: 9 tests passed, including malformed/nonfinite/stale/reset,
foreign-window, modifier, suppression and span controls. An empty-discovery
negative exits1 at the intended guard without a success marker. Final Rust
conformance wrapper: 2 passed; corrected history records also pass (exit 0).
Driver CPU Xvfb rehearsals pass on ISO page20 in GTK and Qt at actual scales1/2;
all four arms observe3fresh pointer positions and4coreZoom commands. Qt additionally
observes4accepted callback/spans. Scale2 maps logical-to-core2.0 on both axes,
with XTest-to-core approximately1.0 plus separately measured offsets.
Missing-page negative exits1 before any input phase; expected-refusal wrapper passes.

Rehearsal holds: 1x6s/.29GiB,2x5s/.44GiB,page-negative25s/.25GiB; queue time separate.
The final driver was tested at2x and against the readiness negative. Exact commands,
logs, binaries and source hashes: /home/AI/toolkit-rehearsal-1695 and scratchpad/r1695.
Root reviewed agent driver changes before copying the exact tested file into tools.
A sibling review added the empty-discovery guard; production parser logic unchanged.
No owner-loop GPU job was published during these CPU instrument rehearsals.

**Limits.** Actual X11 backends are GdkX11Display/xcb; Qt XI2 is disabled in CPU
rehearsal. Scale2 core viewports differ: GTK1218×2038 versusQt1238×1994, so these
are not matched pixel/timing arms. No native-Wayland, scanout, settled-sharpness,
deep-zoom drift or responsiveness result follows. User symptoms remain unresolved.
Combined workspace, fixed corpus arms and canonical gates remain pending.
Quoted holds are measured; total round wall duration is not inferred from queue time.
