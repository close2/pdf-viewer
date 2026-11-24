# 3585 — A host says what its windows can do, and Table 294's windows are judged against it

Session 2221. Status: accepted. Context: RFC 0015 section 6 round 10; ADRs 1960, 3350, 3407, 3538,
3545; ISO 32000-2 §13.2.2, §13.2.6.1 Tables 293, 294, 295.

## Decision

1. **`pdf_model::media::Screens` is a fact a host states, never one read off the machine** —
   whether it opens a window of its own for a clip (`windows`), whether it can place one
   (`placed`), and which of Table 304's seven monitor specifiers it can find (`monitors`). It is
   `Processor::screens`, held per document in `ViewState::set_screens`, and reaches the core as
   `viewer_core::Command::OpensMediaWindows`, sent beside `ShowsVideoHere` on its rule (every open
   document and every later one). Every field false, the default, judges `/W` 0 and 1 as before.
2. **Only a clip with pictures, played here, by a host that opens windows, is read for one.** A
   sound shows nothing, and a host that opens no window (Qt, the C ABI) keeps the old judgement:
   non-viable in `MH`, set aside in `BE`.
3. **Each Table 295 entry is judged on its own**, as the clause says ("individually evaluated for
   viability"), `MH`'s `/F` before `BE`'s; an unrecognised `/R`, `/P`, `/RT` or `/O` is non-viable
   in `MH` and its default in `BE`; `/T` or `/UC` that is not a boolean likewise. `/D` is required:
   unusable in `MH`, the rendition is non-viable; a floating window `BE` asks for without a usable
   `/D` cannot be made, so the clip plays in its annotation's rectangle, said. A zero side is not
   honourable (no window of no pixels shows a picture).
4. **Entries a host's windows cannot honour** — `/RT`, `/P` and a floating window's `/M` for a host
   that places nothing, a monitor value it cannot find, an `/O` below 1.0 (a transparent floating
   window; this program's windows are opaque) — are non-viable in `MH` and set aside, said, in `BE`.
5. **`/R` has no default in ISO 32000-2's table row**; ISO 32000-1's, 0, is taken as the
   unchanged meaning, and recorded as such in `Resize`'s comment.

## Costs

`Clip::window` makes `MediaRequest::Play` and the sound thread's `Order::Play` large enough that
both box their clip. The wire carries the window in `Event::Media`'s sound and the host's answer
as a new command (57).
