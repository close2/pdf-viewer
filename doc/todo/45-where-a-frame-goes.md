# Where a frame actually goes, now that the trace can say

Status: **open**, opened by the three-hundred-and-ninetieth session, which built the instrument
(ADR 0227) and then ran it.
Priority: 45 — performance, and each item below is *measured* rather than suspected. `doc/todo/44`
was the instrument; this is what it found, and it is the successor to that file rather than a
restatement of it.
Corpus: —, the witness is the project owner's own `tmp/windows/NorthAmerican.30MB.pdf` (65 pages,
30 MB), which is outside the corpus
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`App::attend`, `App::speak`, `App::place_window`),
`crates/render-quorra/src/scene.rs`, `doc/QUORRA_FEEDBACK.md`

## The measurement everything below comes from

40 frames of the owner's document, `Xvfb` at 1200×1500 on `llvmpipe`, `--trace`. **This machine's
software adapter is not the owner's Intel UHD through DX12, so every ratio here is about shape and
the absolute numbers are this machine's.** Re-run before acting:

```sh
Xvfb :78 -screen 0 1200x1500x24 &
DISPLAY=:78 ./target/pdf-viewer --trace tmp/windows/NorthAmerican.30MB.pdf
```

```
40 frame(s), milliseconds:
                median      p90      max       sum
 frame            29.6     35.5     75.3    1225.8
 host              0.0      0.0      0.0       0.3
 scene             1.5     12.8     35.2     226.2
 device           23.7     33.3     73.0     998.9
   encode         11.4     19.2     30.8     478.6
   transfer        2.5      4.0     15.0     127.8
   execute         5.3      7.0     19.5     233.2
   elsewhere       3.4      4.8     18.2     159.3
 settle            0.0      0.0      0.0       0.3
```

## 1. The accessibility publication costs 2 ms a page turn and, off Linux, publishes to nothing

**A defect, and it is the one `doc/todo/44`'s last section predicted.** `App::attend` runs after
every frame that presents; on a page turn it does real work — `Query::AccessibilityTree`,
`Query::Reports`, `App::place_window` and `Bridge::publish` — and on a repeat of the same page it
returns on one comparison, which is what `App::spoken` is for and that part is right.

What it costs on a page turn, measured: **2.04 ms on average, 3.9 ms at worst, 81.7 ms over 40
frames**, against 1225.8 ms of frames. It was inside the frame's reported number until ADR 0227
took it out.

The defect is not the cost. It is that **`viewer_accessibility::Bridge::shortfall()` says this build
has no bridge on Windows and macOS, and the publication runs there anyway**: `Bridge::new` builds an
`accesskit_unix::Adapter` under `#[cfg(target_os = "linux")]` and, elsewhere, a struct with no
adapter in it. So off Linux the tree is built, stored in a `Mutex` that nothing on that platform
reads, and dropped — two to four milliseconds of a page turn spent on a consumer that does not
exist. The document in the witness answers `0 element(s)`, so this is the *empty* case costing 2 ms.

Three things to settle before touching it, and the third is why this is a todo rather than a fix:

- **Where does the 2 ms go?** `Query::AccessibilityTree` walking §14.7 and finding nothing, or
  `place_window`, which asks winit for `outer_position` and `inner_position` — two synchronous X11
  round trips on this platform. The instrument cannot tell them apart; one more `Instant` inside
  `attend` can.
- **What should a build with no adapter do?** Not "skip it": `todo/31` is going to wire the other
  two adapters in, and a skip written today is a skip somebody has to find and remove. The honest
  shape is probably `Bridge::publish` returning early where there is no adapter — the decision
  belongs in the crate that knows whether it has one.
- **And a *third* platform is coming**: `viewer-confined` publishes across a process boundary, where
  "is there a consumer" is a different question again (ADR 0218).

## 2. Our display-list translation is bimodal, and it is not the command count

`scene` — `render_quorra::present::build`, the walk that turns display lists into a
`quorra_scene::Scene` — is 18% of the session (226 of 1226 ms). Most pages cost 0.5 to 1.6 ms. But:

| page | commands | scene | uploads |
|---|---|---|---|
| 26 | 3675 | 1.0 | 128 |
| 3 | 2822 | 2.3 | 793 |
| 13 | 2009 | 11.3 | 151 |
| 28 | 1627 | 13.0 | 133 |
| **19** | **388** | **15.9** | 98 |

**A 388-command page costs sixteen times what a 3675-command page costs.** So whatever is expensive
in that walk is paid per *resource* and not per command, and the two candidates are both in
`scene.rs`: `Image::area_averaged` (minification, on the host, per placement) and the row-reversal
that copies a shading's samples one pixel at a time (§8.9.5's unit square puts the first row at
y = 1). Neither is cached across frames — the area-averaged form is deliberately transient, because
it belongs to the placement rather than to the image.

Note what this refutes: the Windows trace's reading was "frame cost tracks the display list's
command count". It tracks it *through* quorra's encode, and our own share of the frame goes the
other way on exactly the pages a person would not expect.

## 3. Four fifths of a frame is inside `Device::render`, and the largest part of it is CPU

`device` is 999 of 1226 ms. Inside it: `encode` 479 (CPU, turning the scene into device commands),
`execute` 233 (the passes, from the adapter's own timestamps), `transfer` 128, and **`elsewhere`
159 — the swapchain acquire, the present and the timestamp readback**, which is 13% of every frame
and which nothing measures more finely than that.

Two of those three are quorra's rather than ours, and the right instrument is
`doc/QUORRA_FEEDBACK.md` with a measurement attached, not a change here. What is *ours* to decide
first is whether `elsewhere` is the swapchain waiting for a compositor — in which case it is not a
cost at all, it is the frame rate — or the timestamp readback, in which case `--trace` is paying
for its own numbers and should say so. `Device::render` takes the readback after the present
specifically so the person sees the frame first; whether that is free is not established here.

## 4. There is no second machine

Everything above is `llvmpipe` under `Xvfb`. The owner's figures — median 60.4 ms, p90 157, max 514,
and **eight budget refusals** that fell to the processor — do not reproduce here at all: `fallback`
is zero in every column of every run. A refusal is a fact about an adapter's resource budget, so
the eight are the Intel UHD's and cannot be chased from here. **The next run of this file wants the
owner's own machine**, with `--trace=frames` rather than `--trace`, which is 64 lines against 453.
