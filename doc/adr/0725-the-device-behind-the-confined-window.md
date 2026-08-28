# ADR 0725 — The device behind the confined window, and the identity that had to survive the pipe

Status: accepted, 2026-08-28. Session 790. Takes the piece `doc/todo/15` has called "the natural
next piece" since ADR 0713: `pdf-viewer-confined` presents through `render-quorra`, so a page
whose content arrived through the confined pipe is drawn by the graphics device. Cites no clause:
this is ADR 0607's architecture reaching its consumer, and the ledger is untouched.

## What was owed

ADR 0607 established that the device is the host's by necessity — a confined process holding one
dies on its first `ioctl` — and everything since was built so that a host with a device could
draw a page the worker never rasterised: the marks cross (ADR 0633), the worker draws nothing it
ships (ADR 0640), the host-side CPU draw is stoppable (ADR 0650). And then the only host on the
boundary presented through the processor, so the tier change's whole payload arm ended in
`render-cpu`. The marks were crossing for a device no window had.

## Finding: the `Arc` identity two documents promised did not survive the pipe

`Payload::List`'s documentation says the list is "[s]hared because a host keeps it", and the
confined screen's scroll arm reuses a drawing when `Arc::ptr_eq` says it is the same list. Both
were true only inside unit tests that shared the `Arc` by hand. In the live path every
`Query::Frame` decoded a fresh `Arc<DisplayList>` — `decode_list_payload` called
`Arc::new(decode(bytes))` per reply, and `Confined::query` kept nothing between calls — so an
unchanged page recrossing on a scroll carried a new identity every time, the screen's
same-drawing-moved arm never fired, and every scroll re-rasterised every marks page on the
drawing thread. The interning the wire format does per message (ADR 0626) never spanned two.

This mattered twice over for the device: `render-quorra` keys its retained scenes by the pages'
`Arc` addresses (ADR 0351), so without a surviving identity every scroll would have rebuilt and
re-uploaded the scene, and ADR 0702's viewport-only zoom — the cheap path the whole zoom
architecture leans on — could never engage across the pipe.

**Decision: `Confined` holds the identity, keyed by the bytes themselves.** The worker holds one
*encoded* list per page and re-sends those bytes unchanged until a re-render replaces them
(`Marks` stores them encoded), so byte-identical re-crossings are the ordinary case — a zoom
re-encodes the *same* marks under a new target, and the bytes match again. `protocol::HeldLists`
keeps, per page, the encoded bytes and the `Arc` they decoded to; a reply whose bytes equal the
held ones hands back the held `Arc` and skips the decode, which is most of the arm's cost on an
unchanged page. Byte equality and nothing weaker, because the worker is the untrusted side. The
store is bounded by the pages of the frame on hand — a page that leaves the frame or crosses as
pixels is forgotten — and it lives on the host side of the pipe, outside the worker's ceiling.
`decode_answer` keeps its stateless form for the callers that decode one message and stop.

## Decision 1: the device path is the window's ordinary path, and `--cpu` is the flagship's flag

The window brings up a `QuorraWindowRenderer` on the flagship's own launch shape: the
`wgpu::Instance` on a thread started before the window exists (quorra's ADR 0014; roughly 80% of
bring-up), the surface detached into a `Send` `Presenter` the event thread keeps (quorra's ADR
0056), configured before anything can submit (the flagship's `Ungrounded`, argument carried in
this window's copy), and the render thread spawned by the first job rather than by `resumed` —
`CLAUDE.md`'s startup rule. `--cpu` means what it means on the flagship (ADR 0221): no instance,
no driver loaded, the processor's surface and the drawing thread exactly as ADR 0713 built them.
A machine whose device will not come up falls to that same path by itself, out loud.

What was *not* copied, each absence argued: no retained-page proxies, no supersampled sharpening
pass, no cadence clock — all three serve the flagship's reprojection machinery, and this window
reprojects nothing. It shows the newest finished frame and asks for another when the state
changes (`doc/todo/37`'s show-what-it-had), one job in flight, the newest ask replacing a
waiting one, polled at `viewer_host::drawing::POLL` while busy.

## Decision 2: every payload becomes a device page — the pixels arm wrapped, not composed

A marks payload goes to the device as the list that crossed, placed by composing its page-sized
target with the slot's origin (rounded exactly as the CPU path's `blit` rounds, so the two paths
put a page's corner on the same device pixel). A raster payload is wrapped **once** as a
one-`Command::Image` display list drawing the pixels 1:1, and the wrapper's `Arc` is kept while
the raster's bytes recross unchanged — so a scroll of a photo page is a placement change over a
retained scene, not a re-upload. There is no CPU composition anywhere on the device path;
`PresentFrame::raster` is not used at all. The present is three layers: the surround texel
scaled over the window, the page texture at the identity, the chrome — §7.6.4.1's card — at the
identity on transparency.

The wrapped-image route was chosen over handing the composed full-window raster to
`PresentFrame::raster` because a frame carrying a raster never reuses its scene
(`SceneKey::of` allocates a fresh raster id per frame), which would have cost every mixed frame
its marks pages' retained scene as well.

## Decision 3: a device refusal falls back to the *interruptible* thread, not the render thread

The flagship's render thread answers a device refusal by composing the pages on the processor
where it stands. This window does not, deliberately: the refusal comes home as
`Landed::refused`, and the host hands every marks page to `viewer_host::drawing`'s thread — the
one with `pdf_render::Interrupt` (ADR 0650) — saying so on standard error. On this boundary the
lists are a hostile document's, and a hostile page must be drawn where an interrupt can reach
it; the render thread nothing can take back short of exit is the wrong place. The fallback
pixels return to the device as wrapped pages like any others, and a scroll keeps them rather
than re-offering the device the list it refused. Pages the thread's rasteriser then refuses too
keep the existing `Refused` sentence on standard error (trap 5).

## Decision 4: what Escape covers, and what it now does not

Escape still ends the worker (ADR 0241) and takes back the drawing thread (ADR 0650), blocking
on neither — proven again this round. What it does not reach is a device frame mid-draw on the
render thread: quorra has no interrupt, so exit joins that thread for at most one frame. Stated
as a bound rather than hidden: the frame's encode and draw are of content the wire's own message
budget bounded, and the fixture that holds the CPU rasteriser for 27.6 s (ADR 0650) costs the
device lane about 2.8 s a frame *under llvmpipe* — the flagship has carried the same exposure on
the same thread arrangement since ADR 0391. The window stays responsive throughout, because the
event thread never draws.

## Proof, driven under Xvfb on the release programs (llvmpipe; illustration, not a gate)

- **The marks arm on the device**: `PDF20_AN001-BPC.pdf` — instance at 0.034 s, device up and
  surface configured in 43.6 ms, `frame: 1 page(s), 1 as marks`, first frame presented at
  0.099 s. Page turns, two zoom steps and scrolls follow the keys; the captures show the cover
  and then page 3 magnified 2×, sharp, on the surround.
- **The raster arm on the device**: `personwithdog.pdf` — `0 as marks`, the photo wrapped and
  placed; scrolls cost 2.9–7.1 ms a device frame, which is the wrapper's identity holding (a
  re-upload would dwarf it).
- **§7.6.4.1 on the device**: `issue6010_1.pdf` — the card drawn by the chrome lane over the
  surround, *attempt 1 of 3*; the right password opens through the marks arm with no password
  legible in the trace.
- **The abort**: the 1567-byte amplification fixture at four levels crosses as marks; the device
  digests what held the CPU for 27.6 s at ~2.8 s a frame under llvmpipe, off the event thread;
  Escape kills the worker (no zombie), the abort sentence lands in the title, and the program
  exits on `q` in 260 ms.
- **The control**: `--cpu` renders the same cover through the drawing thread and softbuffer,
  unchanged.

What Xvfb cannot answer is owed rather than guessed: the real adapter's bring-up and present
cadence for this window need the owner's session, and `doc/todo/15` records the question.

## Trap-13 calibration

Every new test failed against its own injected defect before being believed; the suite is green
as committed.

| injected defect | failed |
|---|---|
| the reuse bypassed (every decode fresh) | `an_unchanged_list_recrosses_as_the_same_arc` |
| `HeldLists::retain` a no-op | `a_page_that_stops_crossing_as_marks_is_forgotten` |
| the placement forgets the origin | `a_device_screen_keeps_marks_for_the_device_not_the_thread` |
| every raster re-wrapped | `a_raster_page_wraps_once_and_keeps_its_identity` |
| `fall_back` walks no slot | `a_refused_frame_falls_back_to_the_thread_and_a_scroll_keeps_the_result` |
| `Outdated`/`Lost` mapped to `Waited` | `a_replaced_swapchain_asks_again` |

## What this does not close

`doc/todo/15`'s remainder: the warn-before-abort input for the three established windows through
`viewer_host::keys`, the breach-as-refusal item, and moving the established windows onto the
boundary. The device lane's coverage choice stays quorra's default — the flagship's
magnification-driven lane choice was built on measurements of that host, and this window earns
one when a measurement asks for it.
