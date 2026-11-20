# Round 2147 — DCT under the in-process budget, raw deflate reported, a later DSS revision admitted

A round in the `gwg` worktree for three leftovers of audit 2142 and round 2132.

## What changed

- ADR 3289: `pdf_sandbox::run_here_within` runs any work on the in-process path's kept threads under
  its budget (`REQUEST_TIMEOUT` of the thread's processor time, `Exhausted` the image's, the
  ten-fold wall ceiling `Overran` the machine's); `pdf-model`'s `decode_jpeg` decodes through it and
  stays in process. A caller on a `rayon` thread runs the pool's work while it waits, so a banded
  decode never waits for a pool its callers hold. Page one, before and after, in the ADR: no cost.
- ADR 3290: `Damage::Unframed` — a `FlateDecode` stream decoded as raw deflate to its end is drawn
  and reported (§7.4.4.1 names RFC 1950's format), by the whole route and the pump alike.
  `Damage::questions_the_bytes` keeps every consumer that refused damaged bytes (fonts, ICC
  profiles, media, re-encoding, object streams) taking these; `Damage::in_a_chain` lets a later
  stage's damage stand in front of it.
- ADR 3291: the document-timestamp chain reads "the entire PDF file" as §12.8.1 does
  (`covers_its_revision`), and material in a revision after the outermost timestamp is listed, not
  refused. The old calibration test became the two sides.

## Premises

Item 1's held: DCT ran with no budget. Item 2's held. Item 3's was half the story: the material
refusal fired on the same shape and is answered too.

## Ledger

No status moves; 7.4.4.1 and 12.8.5.2 notes extended, three rows' test lists renamed.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over pdf-syntax, pdf-font, pdf-colour,
pdf-model, pdf-transform, pdf-signature, pdf-sandbox exit 0. `pdf-sandbox --bins` exit 0. nextest
`--lib --bins --tests` of the seven (`xvfb-run -a`): 5991 run, 5988 passed, 3 failed, the
manifest's 3 ("failed 3, expected 3"). rustdoc `-D warnings --document-private-items` exit 0 for
six, pdf-sandbox exit 0 on the second run (a link to a private item). Fuzz `check --bins` exit 0.
`cargo test -p conformance --no-fail-fast` exit 0 on the third run; the first wanted the ledger's
test lists renamed, the second this paragraph's walk line. The `corpus` walk exits 101 as round 2143's did before and after: 66 incomplete
against 65, `issue17929.pdf` joined (a font with no `/FontDescriptor`). Its damage lines: one
`/Contents` damaged (`ForeignCharacter`), two undecodable, no `Unframed` in 974 documents — so
item 2 moves no count, and the before walk, which can differ from this only by that variant, was
not run.
