# 0462 — The first configure is the one that can kill you

Status: accepted.
Session: the six-hundred-and-twenty-eighth.
Subject: the project owner's viewer aborted on launch — `Surface::configure` failed, this program
noted it and presented anyway, and `wgpu` panicked inside the acquire that followed. Two defects,
one dependency's panic, and a claim in `requirements.rs` that had been false for two hundred and
thirty-five sessions.

## The sequence, established rather than guessed

The project owner's session, on the Radeon 890M under RADV and X11:

```
target/pdf-viewer tmp/pi.pdf
tmp/pi.pdf: 1 page(s)
note: the graphics device reported: Validation Error

Caused by:
  In Surface::configure
    Failed to wait for GPU to come idle before reconfiguring the Surface

thread 'main' panicked at .../wgpu-30.0.0/src/backend/wgpu_core.rs:4036:26:
Error in Surface::get_current_texture_view: Validation Error

Caused by:
  Surface is not configured for presentation
[1]    abort (core dumped)
```

The second attempt, seconds later, worked. `tmp/pi.pdf` is 3325 bytes and one page, so nothing
about the document is at fault.

Read against wgpu 30.0.0's own source, the sequence is four steps and every one of them is
somebody's decision:

1. **`quorra_gpu`'s `SurfaceState::acquire` configures.** The surface has never been configured
   (`configured: None`), so `wgpu::Surface::configure` is called. It returns `()`.
2. **wgpu-core's `Device::configure_surface` waits for the device to come idle**, and answers
   `ConfigureSurfaceError::GpuWaitTimeout` when the wait leaves a non-empty queue behind
   (`wgpu-core-30.0.0/src/device/resource.rs:5341-5352`). Its own comment says what that means:

   > After the wait, the queue should be empty. It can only be non-empty if another thread is
   > submitting at the same time.

   The name is a misnomer — nothing timed out. And wgpu documents the condition in the API it is
   raised from (`wgpu-30.0.0/src/api/surface.rs`, `Surface::configure`):

   > **Validation Errors**
   > - Submissions that happen _during_ the configure may cause the internal wait-for-idle to
   >   fail, raising a validation error.

3. **The failure reaches only the uncaptured-error handler.** `CoreSurface::configure` takes the
   failing branch, reports the error to the *device's* sink — which this program had print
   `note: the graphics device reported: …` — and, crucially, **does not** install the surface's
   own error sink (`wgpu-30.0.0/src/backend/wgpu_core.rs:3979-3985`). quorra then records
   `configured = Some(size)` regardless and acquires.
4. **The acquire is fatal rather than refused, and only because of step 3.**
   `CoreSurface::get_current_texture` gets `Err(NotConfigured)` and chooses between two answers by
   whether the surface has an error sink (`ibid.:4023-4037`): with one it returns
   `CurrentSurfaceTexture::Validation`, which quorra maps to `SurfaceProblem::Validation` and this
   host already handles; **without one it calls `handle_error_fatal`, which panics**. Under
   `panic = "abort"` that is a core dump.

So the fatal branch is reachable **only on the first configure of a process**, and it is transient
by construction: whether it fires depends on whether another thread's `queue.submit` lands inside
the configure's wait.

### And the race is this program's architecture rather than bad luck

ADR 0391 put the device on a render thread and the surface on the event thread, which is the whole
reason a heavy page no longer holds the only path to the screen. `App::on_the_device` is *adopt,
ask, place*: it dispatches a render to that thread and then, microseconds later, presents on this
one — and the present is what configures. Two threads, one device, submission and configure a
hair apart, by design.

### Reproduced here, on this machine

`doc/environment.md` says a *window* needs a session the agent user does not have, and that is
about the owner's display rather than about a window: `Xvfb` and `lavapipe` give a real window, a
real event loop and a real swapchain. Under them the unfixed binary produced the owner's output
**byte for byte**, including the panic's file and line. It is rare — the numbers are in
`doc/history/628-*.md`, taken from a script that launches the binary a hundred and fifty times and
counts — and it is real.

## Decision 1 — the surface is configured where nothing can race it, and the type enforces it

`crate::renderer::Window::split` no longer returns a `Window`. It returns an **`Ungrounded`**, and
the only way to a `Window` is `Ungrounded::ground`, which puts the window's own background — one
opaque texel of `Medium::surround` scaled over the window — on the surface. That configures it.

The argument is an ordering, and the type is what holds it:

- **`Window::ask` is what spawns the render thread**, and therefore the only way anything on this
  device can ever submit.
- **`ask` is a method on `Window`**, and a `Window` cannot exist until `ground` has returned one.
- Therefore the first configure of the process happens with the device's queue provably empty:
  nothing has submitted, because there is nothing yet that could.

That is the difference between a rule somebody keeps and a state nobody can construct. Once one
configure has succeeded, wgpu's surface holds an error sink for the rest of the process and step 4
above can no longer take its fatal branch — so **every later configure failure is already the
typed refusal this host handles**, at any size, on any thread, whatever the cause.

### Why this is not the probe frame `CLAUDE.md` forbids

`CLAUDE.md` is explicit: "No `wait_until_warm` before the first present, no probe frame, no
pipeline pre-compilation 'to be safe'." Grounding is none of those, and the distinction is
measurable rather than rhetorical:

- Nothing waits for warmth. `Presenter::present` takes whatever the pipeline store has and reports
  a compilation it absorbed, exactly as any first frame of any lane does.
- It is not an extra swapchain. It is the *same* configure and the *same* first acquire the page's
  own present would have paid a moment later, moved to the one moment they cannot fail for the one
  reason that is fatal.
- It is a picture the window should have had anyway: the surround a moved page reveals at its
  edge, rather than whatever the compositor left in the buffer.

Measured, five launches apiece on this machine under `Xvfb` + `lavapipe`, `--trace=launch`,
process start to first present: unchanged inside the run-to-run spread, with the grounding itself
costing tens of milliseconds that came off the first page's present. The numbers are in this
round's history file, because a number in an ADR is a number a later round reads instead of
running (ADR 0281).

### And a failed grounding is a launch decision, not a lost frame

A device that cannot put one opaque texel on its own window cannot show a page on it either. So
`ground`'s refusal returns to `App::bring_up`, which says what happened, says what it asked for,
and **draws on the processor instead** — the path `no_device` has taken since ADR 0221 for a
device that would not come up at all, and `CLAUDE.md`'s second job for the CPU backend. Out loud,
because a fallback that happens silently is a different program from the one that was asked for.

## Decision 2 — the note becomes a value, and the value becomes a decision

`render-quorra` installed a handler that printed and returned. That is not handling. Two of wgpu's
calls on a window's path report *only* this way — `Surface::configure` returns `()`, and so does
`Queue::write_buffer` — so a host that cannot ask afterwards has no way to know that the call it is
about to make will meet a device that refused.

`render_quorra::UncapturedErrors` is the record: the handler says the device's sentence at once, as
it always did, **and** folds it into an `Arc`-shared value the host takes with
`QuorraWindowRenderer::uncaptured`. It is folded rather than queued — a device that has begun to
fail produces these by the frame, and what a decision needs is how many and the most recent words.

The host takes from it in exactly one place, `App::put_up`, immediately after the present that
could have provoked one, and then:

- **A present that succeeded anyway** says so out loud, because no other line will mention it and
  a device raising errors on its own is precisely what goes unnoticed until a window stops
  updating.
- **A present that refused** has its refusal classified by `surface::swapchain`, a free function
  over `SurfaceProblem` *and* what the device said. `Outdated` and `Lost` ask again; `Timeout` and
  `Occluded` wait; `Validation` refuses with a sentence that now carries the device's own account
  — where it used to say `swapchain validation failed`, four words naming no cause and suggesting
  no action — and names the two things a person can do, once per run rather than once per refresh.

The classification is a free function precisely so that it can be tested without a graphics
device, and its tests use the project owner's message verbatim.

## What is still quorra's, and it is an ask rather than a workaround

Two lines of `quorra-gpu`'s `surface.rs` are the defect proper, and neither is reachable from this
tree:

- `SurfaceState::acquire` records `self.configured = Some((width, height))` after a configure it
  never checked, so a surface whose configure failed is remembered as configured — and, because
  `needs_reconfigure` is cleared in the same breath, **never reconfigured again**. A window in
  that state refuses every present for the rest of the run.
- It calls `get_current_texture()` on that surface, which is the call wgpu answers fatally.

`doc/QUORRA_FEEDBACK.md` section 35 carries the ask with wgpu's own line numbers: wrap the
configure in a validation error scope, leave `configured` alone and set `needs_reconfigure` when it
fails, and return `SurfaceUnavailable { reason: Validation }` rather than acquiring. Until that
lands, decision 1 is what makes the fatal branch unreachable here, and it makes it unreachable by
construction rather than by luck.

## Decision 3 — the spec-driven half: a reason that had outlived its capability by 235 sessions

`requirements::Kind::unmet` answers §12.11's Table 275 per type, and its own doc comment says the
answers are "a claim about this tree rather than about the standard, so it decays exactly as a
ledger row does" — naming the shape and predicting its own failures. §12.11.2's ledger row records
two such decays already.

This is the third. `Kind::Transitions` read "no transition player: §12.4.4's timing is obeyed and
the animation between two pages is not drawn". The animation has been drawn since the
three-hundred-and-ninety-third session — `viewer_core::transition` shapes the frame at a fraction
of the way through and both backends draw it (ADR 0230) — and **§12.4.4's own ledger row has said
so ever since**, beside a source sentence saying the opposite.

The two documents disagreeing is the finding, and it says where the sweep has to point: a ledger
row has a gate and a reason in `requirements.rs` does not, so the row was corrected on the round
the capability arrived and the arm was not. What the arm names now is what is genuinely missing —
four of Table 164's twelve styles state no quantity a frame could be shaped from and are reported
by name rather than drawn as a cut — and Table 275's other half, transition actions, is
§12.6.4.15's `Trans`, which is read and performed.

> **This paragraph said *five* until the six-hundred-and-sixty-third session, and so did the arm
> it describes.** The fifth style is `R`, which Table 164 defines as the cut, so it is shaped by
> nothing and reported by nothing; §12.6.4.15's ledger row had retired that wording in the
> five-hundred-and-fifty-third and this document, three source comments and two other ledger rows
> went on carrying it. Which is this ADR's own finding one level up: the sweep has to point at the
> place a claim is *repeated*, and an ADR is the most durable of those places.

## The sweep, run against the defect first

Trap 13: `crates/render-quorra` and `crates/viewer-ui` were swept for the same shape — a graphics
call whose failure is noted and stepped past. Run first against the defect itself, which it finds
(`present.rs`'s handler, the only `on_uncaptured_error` in the tree). Beyond it: `render-gpu`
creates no surface at all and says so in its own module documentation; `viewer_ui::software`'s
softbuffer surface returns `Result` from every call including `resize`; and the four dropped
results in these two crates are each documented at their site and none is a device call. The shape
is singular, which is why the fix is one type rather than a convention.

## What could not be tested here

The owner's own swapchain, compositor and session. A timing fault on a real RADV swapchain under a
real compositor may present differently from one on `lavapipe` under `Xvfb`, and the *rate* the
history file records is this machine's under this load, not theirs. What is established
independently of any machine is the branch: wgpu's fatal path requires a surface with no error
sink, an error sink is installed only by a configure that succeeded, and after `ground` one always
has.
