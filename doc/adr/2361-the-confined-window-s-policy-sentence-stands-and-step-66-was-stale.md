# 2361 — The confined window's policy sentence stands, and drive step 66 was stale

Status: implemented. Session 1763, 2026-10-26, the host slot of batch one hundred and sixteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 §12.8.3.4.4 (a signature policy's published copy); no row moves. Context: ADRs 1738, 2076,
2345.

## 1. What was read

Round 1759 found `66-signature-policy` `wrong` in `quorra-confined` on HEAD as well as on its own
tree. The step waits for, and greps, the sentence "was not fetched — this window has no network".
The window says, for every policy a signature publishes, that its document "was not fetched — this
window holds one document and opens no copy beside it, so it fetches none, at any level"
(`quorra-confined/events.rs`, the `SignaturePoliciesPublished` arm, through
`viewer_host::policy::signature_policy_declined`).

## 2. Which was stale

The step. Round 1620 gave this window a sender (ADR 2076): it composes a form's request in the
worker and transmits it from the host side, and fetches an import named by an absolute URI under
the reader's level. From then on "this window has no network" was false, and ADR 2076 section 2.6
gave the reason that is left for declining a policy's copy: what a fetched copy is for is a document
opened beside the one that named it, and this window holds one. The round changed the window's two
sentences and left the drive's expectation for one of them. The window was right and the step's
premise was older than ADR 2076, as step 88's was older than ADR 2146 (ADR 2345 section 2).

## 3. Decision

The step waits for and greps the window's sentence, and still asks that the loopback server was not
asked for the copy. Its comment says why the window declines. Nothing in the window changes.
