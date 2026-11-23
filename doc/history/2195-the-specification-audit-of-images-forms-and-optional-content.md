# Round 2195 — the twelfth specification audit: §8.9 to §8.11, images, forms and optional content

An audit round in the `audit` worktree, in the shape of review 2038 and audits 2128 to 2184:
every `shall` and `should` of §8.9 to §8.11 (Tables 87 to 101) against `pdf-model`'s image
decoding, inline images, form XObjects and reference XObjects, optional content and the layer
panel's reading of it, probed on the CPU oracle. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2195-specification-audit-8-9-8-11.md` and its directory: 3 B, 0 T, 2 D.
- B01 a `/VE` counts a group with "no effect on visibility" (outside the `/Intent`, or unlisted)
  as ON, so `/Or` and `/Not` let it decide where the `/P` route leaves it out; B02 an imported
  page's groups stay at the target's `View` state when the containing page is printed (and at
  magnification 1.0); B03 an appearance stream's own Table 93 `/OC` is never asked.
- D01 two comments in `evaluate`/`operand` state what the code does not do; D02 a `[/Not]` with
  no operand is reported as the depth bound.
- Four failing tests with controls in `crates/pdf-model/tests/xobjects_oc_8_9_8_11_spec_audit.rs`
  (`content_suite`); listed in the audit's `expected-failures.json` and appended to
  `expected-failures-current.json`.
- The image half held under about forty probes: colour keys before `/Decode`, masks, `/SMask`
  precedence, inline images' ignored entries and abbreviations, `/L` past a false `EI`.
- Readings left: CCITT under a stated `/BitsPerComponent 8`; `RunLengthDecode`'s "8-bit samples";
  a soft-mask group's own `/OC`; `/FormType` other than 1.

## Ledger

§8.9 to §8.11 rows before and after: 40 `implemented`. B findings contradict 8.11.2.3, 8.11.2.2,
8.11.4.5, 8.10.4.1, 8.10.2 and 8.11.3.3.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model -p viewer-qt` exit 0.
`pdf-sandbox` 0. nextest `--lib --bins --tests` of the two (`xvfb-run -a`): 3016 run, 3012 passed,
4 failed, 24 skipped — exactly this audit's four; `audit-diff.py` prints "failed 4, expected 4".
rustdoc `-D warnings --document-private-items -p pdf-model` exit 0. `cargo test -p conformance
--no-fail-fast` exit 0 (101 first: the review named a scratch path, and this paragraph was
unwritten). Fuzz not owed: no public type changed.

## Habit

A rule keyed on an object's *kind* binds every route that draws one of that kind, not only the
operator that names it: an appearance stream is a form XObject though no `Do` invokes it.
