# 1895 — a signature policy copy a window fetched is enforced by the document's validation

Batch 130, slot 5, a "clause" round on ISO 32000-2 §12.8.3.4.4, in two sessions. ADR 2624 written;
ADR 2625 and Q771 unused.

**What.** A copy of a signature policy's document fetched at EN 319 122-1 clause 5.2.9.2's URL is
enforced as a stored copy is, under the three syntaxes. The window that fetched a 2xx answer hands
it back as `viewer_core::Command::PolicyCopy` (command 47 across the confined protocol), because
only the core holds the CMS; `SignaturePolicy::fetched_binding` binds it from the signature's own
identifier and digest, `policy_rules::enforce_fetched` reads it, and its findings reach
`Verdict::under_policy`. A copy not matching the signed digest is `FetchedCopyNotBound`, said, and
never enforced. The closing sentence of every policy note says where the constraints are enforced
from: the document's validation, no copy from that URL, or — for a C caller's copy — none, because
the C interface has no call that hands one on.

**The premise that did not hold.** The brief said the fetch is off by default; it is under A98's
`Submissions` level, default `ask`, and `refuse` is its off. The hypothesis of one call where the
copy is bound did not hold either: the host never holds the CMS, so the copy travels back.

**The finish line and the plant.** `viewer-host/tests/signature_policy.rs`: a loopback serves a
TS 119 172-2 policy to a viewer opened on a signed fixture; at `send` three constraints are
enforced and the `signingCertOnly` rule is broken; one octet changed inside the XML comment is said
not to hash and nothing is enforced; at `refuse` the listener sees no connection and the note says
none was enforced. The fixture's ended signing period is said and not evaluated (no covering
timestamp), and the word valid withheld is proven over `pdf_signature`'s verdict fixture, since no
viewer fixture verifies to an anchor.

**The row.** §12.8.3.4.4 stays `partial`: human-readable policies are a limit, the canonicalised
XML digest is work (ADR 2624 section 4).

**Gates.** `cargo fmt --check` exit 0 on pdf-signature, viewer-core, viewer-confined, viewer-host,
viewer-ui; clippy `--no-deps --all-targets -D warnings` exit 0 on all five; `cargo test`
pdf-signature 286 passed, viewer-core 435, viewer-confined 118, viewer-host 315, all exit 0;
viewer-ui 137 passed, all exit 0; `--bin cited` exit 0, `--bin quotations` exit 0;
`cargo test -p conformance --no-fail-fast` 467 passed, 3 failed, exit 101, every red a sibling
round's work in progress: `records` (round 1892's Gates paragraph), `evidence` (round 1894's record
naming a scratch path), `fuzz_workspace` (the new `type1_write` target's recipe lines).
