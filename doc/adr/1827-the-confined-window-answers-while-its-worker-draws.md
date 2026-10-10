# 1827 — The confined window answers while its worker draws

Session 1495. Status: **accepted** and **built**. Asked by the project owner on 2026-10-10 as part of
"improve performance — loading and display time": the window a person reads hostile documents in
answered no key for the whole of a heavy page's draw, where the other three windows answer beside
theirs (ADR 1811). Context: ADRs 0241, 0607, 0640, 0657, 0713, 1539, 1811, 1815; trap 141. Code:
`crates/viewer-ui/src/bin/quorra-confined/exchange.rs`, and `Host::dispatch`, `press`,
`take_the_opening`, `reopen` and `about_to_wait` in `quorra-confined.rs`. Instrument:
`tools/drive-windows.sh`'s step 80, `key-wait` and `key-delivered`. Cites no clause.

## 1. Where the window waited — the brief's hypothesis, and the half that did not hold

The brief placed the wait on the host's `Query::Frame`. **It is the command before it.** The
worker performs a command whole before it answers — `viewer_confined::worker::perform` runs every
`NeedsRender` the command raises through `CpuRasterizer` and only then writes the events frame — so
a page that crosses as pixels (ADR 0607; the owner's page does, its list many times its raster) is
drawn *inside* `Confined::handle`. Round 1489's owner's-display trace says so in its own stamps: the
open joined at 2.538 s, `-> Resize` at 2.538 s, its events back at 11.016 s, and the frame that
followed at 11.021 s. The window's thread was inside one `handle(&Command::Resize)` for 8.5 s, and
`Query::Frame` cost five milliseconds of it. The other half held: the fix is ADR 1811's shape.

## 2. The decision

**Every command goes to the worker on a thread of its own and is waited for no longer than
`drawing::WARN`** — a `viewer_host::Loading`, the type the open already uses, holding the
`Confined` for the exchange and handing it back with the answer. A command answered inside `WARN`,
which is every command of the launch gate and of the drive's other steps, is then handled exactly
where and as it was. A command still out is left to its thread; the title says what the worker is
doing — `loading::reading`'s share where it is reading a page, `status::still_drawing` where it is
drawing — and `about_to_wait` looks every `loading::TICK` and takes the answer in when it has come.

**What the reader does meanwhile is held, in order, and done once the worker is free.** Three keys
act at once because they are about the wait: Escape, the abort (ADR 0241: the kill reaches a worker
the window's thread is no longer inside); `q`; and the close, which now also kills a busy worker,
since a process that leaves does not run the drop of a handle another thread holds. A key is held as
the key, with the modifiers held at its press, and decided when it is done — what a key means can
depend on a question to the worker (a field's value is read back before each character), and a busy
worker answers no question. A resize is held as its newest size only. Tab's question of which field
the keyboard is now in is held behind the move that went out. **What an answer causes goes in front
of what was held**: the events of the command that came back and the commands they ask for, an
open's first size, a restart's view — the order those had when the window's thread did all of it
inside one call.

## 3. Priced against

- **A worker that answers while it draws** — `viewer_host::drawing`'s arrangement inside the
  confinement, so that the worker takes a command while a page is out. It is what would make a held
  key *reach* the viewer sooner rather than be *taken* sooner, and it is the larger change: the
  worker's one loop becomes two, and `Query::Frame` must answer for a page still being drawn. Not
  this round's; the instrument's `key-delivered` is the figure it would move.
- **One long-lived exchange thread fed by a channel.** The same behaviour with a type of its own; a
  thread per command is the existing `Loading` and costs a spawn per command, against a pipe round
  trip already per command.
- **Dropping what is pressed while busy**, which the window did during the open before this. Every
  key would be answered and some would be lost, and a loss is the worse surprise.

## 4. What it does not do

The worker still performs one command at a time, so a held key reaches the viewer when the draw it
waited behind has ended, and ten zoom presses held behind a heavy page are ten draws after it, as
they were. They are not merged: `Zoom` is relative and the viewer clamps it, so only the viewer
knows what ten of them come to.

**The instrument says both.** `key-wait` is now the longest a key waited for the line on which the
window took it — handed to the viewer, or held — and `key-delivered` the longest it waited for the
line handing its `GoTo(Next)` to the viewer, which is `key-wait` in the three in-process windows and
was the confined window's `key-wait` before this ADR. A held key's two lines are `-> held behind the
worker: key Named(ArrowRight)` and `-> GoTo(Next) (held N s)`.
