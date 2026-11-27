# 2284 — A movie played backward shows its pictures last first

Lane `gwg`: `doc/todo/80` section 2, step one (ADR 3818's plan). ADR 3837; no question.

- **The worker** (`pdf-media-worker`): a video run carries a direction (`wire::VideoRun`, flag bytes
  2 and 3). A backward group's first run begins a session of its own from the track's parameter
  sets; every picture is held in a `video::Group`, and the run that ends the group writes it to the
  ring last picture first. Bound `MAX_GROUP_BYTES`, 96 MiB of planes (thirty-two 1080-line
  pictures), refused by name by the host before sending and by the worker while decoding. The
  system-call set, the ring and its descriptor are unchanged.
- **The host** (`MediaWorker::decode_mp4_video_backward`): groups from sync sample to sync sample,
  the last first; each picture's time counts down from the section's end (or the track's) to the
  picture's end; a resume skips the groups wholly before its place.
- **The hook**: `pdf-model`'s `told` sets `Playing::backward` for a negative `/Rate` (crossing the
  confined protocol) and says the sound is not reversed; `viewer_host::video` plays the pass on the
  clip's own clock (`Backward`, `Timeline::end`), and `sound::Player::play` declines the movie's
  sound by name. `Palindrome` still says its alternation is not built.
- **Tests**: a fixture of eight pictures carrying their own index in two `stss` groups, played whole,
  from a section's end and resumed; a group past the bound refused before any picture; `Group`'s
  order and bound; `VideoRun`'s flags; `played_video` through the window's player; `told`'s three
  rates; the protocol round trip with `backward`.
- **Measured** (ADR 3837, debug worker): a 1080p group of 32 peaks at 174 MiB (`VmPeak` 178 440 kB)
  under the 256 MiB ceiling; first backward picture 191–212 ms after the request; the fixture's group
  12 288 bytes, 1.5–1.9 ms. The ring needs no resizing. A 1080p group longer than 32 is refused.
- Ledger: 13.4 stays `partial` (the backward sound and `Palindrome` remain); its note, code and
  tests restated.
- Left, in order: section 2 step two (the sound reversed per block, `Palindrome`); the ceiling for
  1080p groups past 32 pictures; then section 1.

**Gates.** Over `-p pdf-model -p pdf-media-worker -p viewer-core -p viewer-host -p viewer-qt -p
viewer-confined` (clippy also `-p conformance`): fmt 0, clippy `-D warnings --all-targets` 0,
`pdf-sandbox`/`pdf-media-worker --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 4523 of 4523,
failures equal to the manifest (none), rustdoc `-D warnings --document-private-items` (four crates,
private directory) 0, fuzz `cargo check --bins` 0, `cargo test -p conformance --no-fail-fast` 101 before this paragraph was written (only `records`,
on this record's placeholder; 76 suites ok), `--test records` 0 after. Launch (`tools/state.sh --round
2284 launch`) 0: 36 figures banded, none outside, first page 88.8 ms.
