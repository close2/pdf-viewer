# 3314 — A shipped command draws a page on the device: `quorra-draw`

Session 2153. Status: **implemented.** Context: CLAUDE.md principle 2 (page one goes to the device);
RFC 0016 section 5; the owner's A948 (recommendation 3); ADRs 2664, 2675 and 3093 (the shared
adapter's lock, held by the code doing device work).

## The problem

The portable suite judged this program's pages through `quorra-transform render`, which draws on the
processor's oracle, `render-cpu`. The window draws on the graphics device through `render-raster`,
and no shipped command did, so the pixels a person sees were never judged by the clause.

## Decision

A crate of its own, `crates/quorra-draw`, whose one binary draws one page to a PNG on the device:
`quorra-draw <file.pdf> [--page <n>] [--dpi <d>] -o <out.png>`, installed by `tools/batch.sh install`
with the other programs a person runs.

- **Its own crate**, not a verb of `quorra-transform`: `pdf-transform` would otherwise link `wgpu`
  into every one of its programs, the confined `quorra-transform-worker` among them, and its
  dependencies are the ones a document's bytes reach in a confined process.
- **The same contract as the CPU half.** One-based `--page` (as `--pages` is), 150 dpi by default,
  the scale §8.3.2.3's 1/72 inch makes, `TargetSpec::for_page` under the transform's 2^28-pixel
  budget, and an 8-bit RGBA PNG through the same encoder. Exit statuses as the transform's: 0, 1
  (read, write or no device), 2 (usage), 4 (declined by name).
- **The window's view state** (`ViewState::of`, §8.11.4.4's `View`), where the transform renders under
  `Export`: what is judged is what the screen shows. No suite test has optional content, so the two
  adapters' states agree on all 17.
- **No processor fallback.** A page the device refuses exits 4 naming the refusal. The window may draw
  such a page on the processor and say so (principle 2); this command exists to show what the device
  draws, and a quiet fallback would make it judge the oracle twice.
- **The lock is the program's.** `gpu_lock_hook::install()` runs before the device exists, through the
  same `#[path]` modules the tests include, so on the server the device's whole life is one queued hold
  (measured: one hold of 77 ms for one page, `ticket=` set) and on a machine without the lock file
  nothing is taken. The device is dropped before the PNG is encoded.

## Cost

The lock module's failures are panics: a wait past `GPU_LOCK_TIMEOUT` (600 s) or a standing hang
marker stops the command with the lock's name and status 101, not with a typed failure. That module is
written for tests, and making its failures typed would change every test that includes it; the
command's own failures are typed. The cost is one exit status a script reads as "error", on the one
machine where the lock exists.
