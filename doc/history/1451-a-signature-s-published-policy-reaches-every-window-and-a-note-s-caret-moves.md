# 1451 — A signature's published policy reaches every window, and a note's caret moves in `quorra`

Slot 2 of batch seventy-one, 2026-10-08, a host round. ADRs 1738, 1739; no row moved, no question.

**Premise.** Held: `doc/history/1445`'s five parts were each missing and `policy.rs`'s five
functions were in place and called by no window; ADR 1728's thirteen-signature census stands, so
the drive needed a signature of its own (no corpus file names a loopback URL). Held for the caret:
`press_on_note` put it at `text.len()` and `typed_into_note` appended only.

**Built.** (1) `Event::SignaturePoliciesPublished` from `Command::Report`, after its `Reported`
(`notes::About`, deduplicated); `Submitter::fetch_policy` and `Asked::Policy`, whose reply is
`signature_policy_fetched`; `Subject::Policy`; `policy_fetch_note`; `PublishedPolicy` re-exported.
GTK, Qt and `quorra` ask the submissions level and open a bound copy beside; the two one-dialogue
windows decline a policy while another question waits; `quorra-confined` declines. Wire kind 26,
`PDFVCF12`; C ABI kind 26, count 27, no binding offered. `SUBMITTING` names the fetch (ADR 1738).
(2) `chrome::popup_caret`/`popup_offset` over the plain layout `draw_plain` draws; a press, the
arrows, Home and End place `quorra`'s note caret and edits splice at it; an `/RC` window keeps the
end (ADR 1739); the caret is drawn over the popup windows. `react`'s `OpenFailed` arm moved to
`open_failed` in both toolkits, for the line cap.

**Drive.** Step 66 (fixture: a CAdES signature by the drive's signer under policy
`2.16.724.1.3.1.1.2.1.9`, URL on the loopback server, verifying under `openssl cms -verify`):
bound and the magenta copy opened beside at `send` (748 840, 479 283, 495 819 px), the altered
digest told apart with 0 px, `refuse` not asking the server, `quorra-confined` declining — 13
works, photographed and looked at. Step 67 (a plain note: press, Home, "A ", End, Left, "!"):
"A Plain words!." saved in all three, `quorra` placing the press at 0 of 12. Its first passing run
drew no caret — the caret overlay was under the windows — which only the photograph showed; drawn
over them, the zoomed shot has it between "!" and ".".

**Unfinished.** A C caller cannot bind a fetched copy (ADR 1738); a rich note's caret stays at its
end (ADR 1739); `launch_path` not run, no open path changed. The wire is `PDFVCF12`, kind 26 new.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy` on the
seven host crates `--all-targets`: exit 0. `cargo nextest run` on them: 934 passed, 0 failed (viewer-ui
159 after the overlay fix). Planted: an identity collapse mapping failed the caret round trip; the
core and host tests carry controls (no qualifier; a 404). `cargo test -p conformance`: exit 0, 421
passed. `cargo check --manifest-path fuzz/Cargo.toml`: exit 0. Lock, `--tree 6`: steps 64, 66, 67
in four windows exit 0, 13 works and 3 wrong (67's press found the icon, fixed) (173 s); 67 in
`quorra` after the overlay fix: works (10 s). `--tree 12`: the whole drive exit 0, 239 works, 0
wrong, 0 to look at, 6 not offered (738 s).
