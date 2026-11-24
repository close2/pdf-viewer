# 2221 — A clip's window of its own, and the monitor it goes on

Round 2221. ADRs 3585, 3586, 3587.

## What changed
- `pdf_model::media::Screens` (ADR 3585): a host says whether it opens a clip's window, places one,
  and which Table 304 values it finds; `Command::OpensMediaWindows` carries it. For a clip with
  pictures played here, Table 294's `/W` 0 and 1 are read into `Clip::window` (`media/screen.rs`),
  each Table 295 entry judged alone, `MH`'s `/F` first; unhonourable entries are non-viable in `MH`
  and set aside, said, in `BE`. A sound, and Qt and the C ABI, keep the old judgement.
- `viewer_host::video::screens` (ADR 3586): Table 304's seven values over a toolkit's monitors,
  value 2's "treat as case 0", ties to the document window's monitor; `/RT`, `/P` with the frame,
  `/O`'s three answers. `VideoPlayer::window_picture` draws `/B` and the `/F`-fitted picture.
- GTK (ADR 3587): a `gtk4::Window` with a `GtkPicture`, sized by `/D`, resizable only for `/R` 2,
  full screen on the chosen GDK monitor; GTK 4 places none, said. `quorra`: a second winit window
  with a software surface, placed by `/RT`/`/P`/`/O` off Wayland, full screen on the chosen winit
  monitor. Both keep the title bar and close (`/T`, `/UC` false ask nothing), title `/TT` then the
  clip's label; Escape or close stops the clip.

## Tests
`pdf-model` three in `media_viability.rs`; `viewer-host` five in `video/screens.rs`, one in
`video/tests.rs`; GTK two under Xvfb in `shown_here.rs`; `quorra` one headless; protocol round
trips. Not exercised: a second physical monitor, the `quorra` winit window itself (no event loop
in the headless test), a window manager's placement, a person watching. The floating raster was
rendered to a PNG and looked at: picture met at the left, `/B` blue at the right.

## Ledger
§13.2.7.5 `out-of-scope` → `implemented`. §13.2.6.1, §13.2.7: notes, code and tests.

**Gates.** Over `pdf-model`, `viewer-core`, `viewer-confined`, `viewer-host`, `viewer-ui`,
`viewer-gtk`, `viewer-qt`, `viewer-ffi`: fmt 0; clippy `-D warnings --all-targets` 0; `cargo build
-p pdf-sandbox -p pdf-media-worker --bins` 0; nextest `--lib --bins --tests` under `xvfb-run -a`
4583 of 4583, none against the manifest; rustdoc of the six touched crates 0; fuzz `cargo check
--bins` 0; `cargo test -p conformance` 0.

## Left, in order
1. §13.2.6.3's timespans for the pictures: a section's begin and end and a duration cut them.
2. The `quorra` winit window driven by a test with an event loop; a second monitor, by hand.
3. Table 307's controller bar (round 12), the poster (13), §13.4 in place (14).
