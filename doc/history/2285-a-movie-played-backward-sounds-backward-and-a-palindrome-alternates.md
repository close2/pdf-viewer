# 2285 — A movie played backward sounds backward, and a palindrome alternates

Lane `gwg`: `doc/todo/80` section 2, step two (ADR 3818's plan). ADRs 3841, 3842, 3843; no question.

- **The sound backward** (`viewer-host`'s `sound/reversed.rs`, ADR 3841): a movie's section is
  converted, levelled and cut as a forward sound's is, held, and at its last run handed to the sink
  turned — the runs' order and the order within each reversed, every frame's channels kept. One clock
  with the pictures, from the section's end. Bound `MAX_REVERSED_SAMPLES`, half of
  `MAX_PLAYED_SAMPLES` (87 s of 48 kHz stereo), refused by name. The worker is unchanged.
- **`Palindrome`** (ADR 3842): `Playing::palindrome` crosses the confined protocol; pass `n` runs
  backward where `backward` differs from "palindrome and `n` even" (a negative `/Rate` makes the
  first pass the backward one: a reading, said as one); the pictures decode each pass from the bytes
  in hand, and the sink holds the section in both directions from one decode.
- **The 1080p group** (ADR 3843): parts, not a higher ceiling. A group past 96 MiB is split into as
  few equal ranges of output places as fit, at most four; each range is decoded from the sync
  sample, only its pictures held (`Group::keeping`), decoding stopping at its last place, the last
  range first. `VideoRun::Backward` carries the range. `pdf-sandbox` is untouched.
- **Measured** (debug worker, three runs): 1080p groups of 32 / 60 / 128 pictures in 1 / 2 / 4
  parts peak at 178 424 / 218 708 / 224 320 kB `VmPeak` under the 262 144 kB ceiling; first
  backward picture 244–482 / 229–378 / 338–353 ms; last 0.3–0.6 / 0.5–0.9 / 1.45–1.55 s.
- **Tests**: sixteen samples carrying their place reach the device last first, and a section cut
  then turned; three palindrome passes of sound (both first directions) and of pictures alternate;
  the 8-picture fixture split into halves and into single pictures plays exactly as unsplit;
  `parts`, `Group::keeping`, the wire's range, `Turning`'s layouts and bound. Planting each defect
  back (no second copy, no turn, the range ignored) fails four tests.
- Ledger: 13.4 stays `partial`. Its last unbuilt `shall` is Table 307's `/Synchronous` true
  ("the movie player shall retain control until the movie is completed or dismissed by the user");
  note, code and tests restated, `doc/todo/80` section 2 step 3 names it.
- Left, in order: `/Synchronous` for a movie this program shows (section 2 step 3); then section 1.

**Gates.** Over `-p pdf-model -p pdf-media-worker -p pdf-sandbox -p viewer-core -p viewer-host -p
viewer-confined -p viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox`/
`pdf-media-worker --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 5091 of 5092, the one
`records` on this record's placeholder (manifest: none expected), rustdoc `-D warnings
--document-private-items` (four crates, private directory) 0, fuzz `cargo check --bins` 0, `cargo test
-p conformance --no-fail-fast` 101 on the same placeholder alone (76 suites ok), `--test records` 0
after. Launch (`tools/state.sh --round 2285 launch`) 101 twice: `peak_anon_mib` outside its band on
opt_demo (6.10, 6.69 of 6.0), and on the second run PDF20_AN001-BPC and xfa_filled; round 2283, which
touches no launch path either, read the same three outside. Not cleared against a HEAD build.
