# Round 2142 — the fourth specification audit: §7.4, filters

An audit round in the `audit` worktree, made in the shape of review 2038 and audits 2128, 2131
and 2136, turned on where compressed untrusted bytes are expanded: §7.4.1 to §7.4.10 and Table
5's stream entries, against `pdf_syntax`'s filters, `pdf-model`'s image codecs and the sandbox's
confined decoders. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2142-specification-audit-7-4.md` and its directory. Ten findings: 7 B, 0 T, 3 D.
- B01 a `/Filter` element that is not a name is dropped, so the stream decodes by fewer filters
  than it states; B02 ASCIIHex and ASCII85 data without its EOD marker is reported whole; B03 a
  Table 8 entry of the wrong type is its default, so `/Predictor 12.0` hands back the PNG rows as
  data; B04 the PNG predictor runs at a depth Table 8 does not list; B05 a predictor runs after
  RunLength; B06 `/F` naming an undefined object makes a stream external; B07 a Crypt filter naming
  no defined filter, in an unencrypted document, passes its ciphertext through.
- D01 one `/DecodeParms` dictionary serves every stage; D02 ledger 7.4.10's "commutes"; D03.
- Seven failing `*_spec_audit` tests in `crates/pdf-syntax/tests/filters_spec_audit.rs`, each with
  a control; listed in the audit's `expected-failures.json` and appended to
  `2038-specification-audit/expected-failures-current.json`.
- The brief's 2038-B56 is the Warichu split; the indirect-parameter shape it described is B54.

## Ledger

Eighteen rows audited: 17 `implemented`, 7.4.9 `departed`, before and after. B findings
contradict 7.3.8, 7.3.8.2, 7.4.1, 7.4.2, 7.4.3, 7.4.4.3, 7.4.4.4, 7.4.6 and 7.4.10, and the
aggregate 7.4.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over `-p pdf-syntax` exit 0 on the
second run; the first found four unbackticked filter names in a test's doc comment.
`pdf-sandbox --bins` exit 0. nextest `--lib --bins --tests` of `pdf-syntax` (`xvfb-run -a`): 423
run, 416 passed, 7 failed, 1 skipped, the manifest's 7 ("failed 7, expected 7"). rustdoc `-D
warnings --document-private-items` exit 0. `cargo test -p conformance --no-fail-fast` exit 0 on
the second run; the first wanted this paragraph and no round number in the test's module comment.
Fuzz not owed: no public type changed.

## Habit

A parameter has a type as well as a place: a resolved value of the wrong type still becomes the
table's default in silence (grep `unwrap_or(<default>)`).
