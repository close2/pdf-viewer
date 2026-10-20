Status: complete
Given: 2026-10-20, in conversation — transcribed by the round
Owes: ADR 2074 section 4's after-steps (acting round) — point the three manifest stanzas at `close2/hayro` at `7e39af411b807cc267fb7a72545083f10ca2077f` and add the fork to `deny.toml`'s `allow-git`; `cargo update -p hayro-jbig2 -p hayro-jpeg2000 -p hayro-ccitt`; delete the three patch files; delete `decode::extended_template_region`'s refusal and turn its test into a decode; move §7.4.7 and its headings to `implemented`; compare the six arms by digest

> I have executed the sudo command and agree with your recommendation.  Please push and answer both.

Reading: the recommendation adopted is the question's own, quoted and attributed: *"Push it."* —
and the push is done, in the same sitting as the answer, so the owed work carries only the steps
that follow it.

**The branch is on the fork.** `quorra/t88-patches` is pushed to `close2/hayro` at
`7e39af411b807cc267fb7a72545083f10ca2077f` — the tip the question prepared, verified with
`git ls-remote` after the push; the bundle kept the commit hashes, so the rev the manifest will
name is this one. Three commits on the pinned base `ea9c81dc` (upstream's
`hayro-jbig2-v0.3.1`): the symbol-dictionary loop bounds (ADR 1447), the generic region's
extended-template decode (ADR 1459), and the T.88 Annex H.2 arithmetic-decoder test (ADR 1485).

Why push rather than wait: upstream implemented the same clause on its own (#1485) but the
commit is unreleased and carries 118 others; the branch carries the fix today, and the refusal
in `pdf_sandbox::decode` comes out with the steps above rather than at some future release's
pleasure.