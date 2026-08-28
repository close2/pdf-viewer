# 781 — The prompt the confined window owed its first verb

**Subject**: the confined boundary after its first host (round 775, ADR 0713). The next honest
step among that host's refusals-by-name was the sharpest of them: an encrypted document was
refused in front of *open*, the first verb of the window's own charter. This round gives
`pdf-viewer-confined` ISO 32000-2 §7.6.4.1's prompt, entirely out of the shared pieces, and
writes down the design question the briefing raised — which way the password goes. ADR 0718 has
the argument; this file is the bookkeeping.

## What landed

- `crates/viewer-ui/src/bin/pdf-viewer-confined.rs` — the whole diff of the round's code. The
  `PasswordRequired` arm prompts instead of stopping; `viewer_host::password` is the policy
  (three attempts, an empty entry is a decline, `Exhausted` leaves the window open);
  `viewer_ui::chrome::PasswordCard` is the card, drawn over the surround through
  `SoftwareSurface::present`'s overlay parameter; `Chrome` loads on the first prompt and never
  on the launch path. While the card is up it has the whole keyboard; Escape with the card up is
  the decline, Escape with it down remains ADR 0713's abort. A retry re-reads the file (rule 2,
  the flagship's arrangement) and re-sends `Command::Open` to the **same** worker, which
  survives an open it could not finish (ADR 0597).
- **The password crosses into the confinement, and that is now argued rather than incidental**
  (ADR 0718 §1): decryption happens where the document's bytes are, and the confinement is what
  bounds where the password can go from there. The protocol needed no change at all —
  `Command::Open` has carried the `Secret` since the transport existed — so no wire format
  moved and `confined_wire` was owed nothing beyond §2's check line.
- Documents brought along: the ledger's §7.6.4.1 note (its "three hosts prompt" cardinal, now
  four with the crossing named), `doc/todo/15`, `doc/state-of-play.md`, and the binary's own
  module documentation, which had stated the refusal.

## Trap-13 calibration of the four new tests

Each defect injected separately, watched fail, then reverted; the suite is green as committed.

| injected defect | failed |
|---|---|
| the `PasswordRequired` arm stops the document as it used to | `an_encrypted_document_is_prompted_for_not_refused` |
| `Ask::Exhausted` closes the window | `exhausted_attempts_leave_the_window_open` |
| Escape with the card up calls `abort` | `escape_declines_the_prompt_without_aborting` |
| a file missing at the retry is fatal | `a_file_gone_before_the_retry_is_said_not_fatal` |

## Proof under Xvfb

Release builds from this worktree, display `:181`, on `issue6010_1.pdf` (whose password pdf.js's
manifest records): the card up over the surround at 0.043 s saying *attempt 1 of 3* with the
clause number in the question; a wrong password re-sends `Open` (no password legible in the
trace) and brings the card back as *attempt 2 of 3*; the right one answers
`Opened { pages: 1 }`, `frame: 1 page(s), 1 as marks`, and the decrypted page on the screen.
Then, in one sitting: `q` through the card quits nothing (modality); Escape declines — worker
alive, window open, the CANCELLED sentence in the title; Escape again aborts — worker gone, no
zombie; `q` exits. ADR 0718 §"Proof" has the full account.

## Gates — the change→gate map's row for `viewer-ui`, plus what the documents demand

Not a fifth round (`tools/round.sh`: four more until the full sequence), the change cannot move
a pixel (no gate-rasterising crate is reachable from `viewer-ui`), so the core plus the
conformance gate:

- `cargo fmt --all --check` — clean.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — exit 0.
- `cargo nextest run --workspace` — **2721 tests, 2721 passed, 18 skipped** (1 slow: the C ABI
  gate), no flake this round.
- `cargo test --workspace --doc` — exit 0.
- `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` — exit 0.
- `cargo test -p conformance` — 200 passed, exit 0 (the ledger note and the new ADR are read by
  it).

## Sweeps, against the pristine baseline

`parts`, `pointers` and `quotations`, run in this worktree and on `main` at `28ed2239`, all
exit 0 both sides. Every delta accounted: `parts` **identical** (586 cardinals; the ledger
append added no counted cardinal); `pointers` +3 live and nothing else moved (the round's new
files); `quotations` +1 quotation in +1 document, in the not-a-spec-quotation noise category
(the new ADR's one dialogue phrase).

## Deliberately not done

- §5's install into the main tree's `target/` is the merge round's (775's precedent, trap 15's
  shape); this round's proof ran from the worktree's own release build.
- The worker retaining the bytes so a retry crosses only the password — priced and refused in
  ADR 0718 §2: a protocol message and worker-side state for one crossing per attempt, on
  documents that are kilobytes in the corpus.
- The warn-before-abort input for the three established windows (`viewer_host::keys`) and the
  quorra surface behind the confined window stay in `doc/todo/15` — neither shrank nor grew.
- CI on `main` was already failing when the round began (`tools/round.sh` said so); nothing here
  touches it.

## A mistake made and paid inside the round

The first trap-13 calibration reverted its injected defect with `git checkout -- <file>` before
anything was committed, which took the round's own uncommitted work with it. Everything was
reapplied from the transcript and re-verified, and the remaining calibrations ran above a WIP
commit so a revert had a floor. The rule that generalises: **calibrate above a commit**, because
`git checkout --` restores HEAD, not "the state before the injection".
