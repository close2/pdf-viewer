# 1062 — The anchor a host supplies, and the word it makes sayable

ADR 1039 decided a trust store is an input and named what it had not built: the host end. Forty rounds later
nobody had, and every §12.8 row saying *the anchor and nothing else* meant the same missing thing. This round
built the passing-in and the type it makes reachable. ADR 1076.

**The seam.** `pdf_signature::trust::Supply` — the DER, each certificate under the name its host knows it by,
RFC 5280 section 6.1.1's input (b), and the sentence saying where they came from. `TrustAnchors` could not be
it: every anchor borrows from bytes somebody has to own. `Supply::read` lends, naming each certificate it will
not take rather than counting them (trap 5).

**The host surfaces.** `Command::Trust(TrustPolicy)` in `viewer-core`, the one thing that forgets
`Open::about`'s `OnceCell` — a report is a function of the file *and* of whom the reader believes.
`viewer_host::policy::trust_anchors` is the seventh decision in that module and reads a directory of PEM or
DER; `quorra` takes `--trust-anchors <dir>` and `--accept-unknown-revocation`, on the document's thread rather
than the launch path. `viewer-confined` carries the DER *into* the confinement (kind 28).
`quorra_trust_anchors` is the C ABI's and takes no struct by value, so `QUORRA_ABI_VERSION` did not move. **A
host supplying none gets today's answers word for word**, which `notes.rs`'s test asserts both ways.

**The rule as a type.** `verdict::Valid` has no public constructor and no public field; `Verdict::reached` is
the only thing that makes one and it takes an `Anchored`, which `Anchored::of` makes from a `Trust::Anchored`
and from nothing else. `revision::Judgement` deliberately gained no variant — it ranks objects, not people.

**Two things found on the way.** §12.8.3.2's path could not be built at all: its `/Contents` is a PKCS #1
object with no CMS in it, so `Signature::trust` found no `SignedData` and answered `NoPathToAnyAnchor {
examined: 0 }` about a file carrying exactly the certificate the clause puts in `/Cert`;
`Signature::chain_trust` builds it from there now. And §12.8.4.2's clock was built and unused:
`notes::established_over` takes the innermost document timestamp covering a signature whose own authority
reached an anchor, and validates the signer's path at *that* instant — which is what lets a signature outlive
its certificate.

**Calibration (trap 13), and what the corpus answers.** The positive witness is a document this round issued
the hierarchy for: `openssl cms -sign` over a `/ByteRange` whose bytes do not depend on the value filling the
hole, so the digest was takeable before the signature existed and a test re-walks the provenance. Its root →
valid; another root → no path; none → `NoAnchorSupplied`; one page byte moved → the document changed.
`xfa_filled_imm1344e.pdf` under a supplied anchor answers *no certification path reaches any anchor supplied*:
its authority is real and nobody here holds its root (trap 8). `signature_algorithm_census` prints what the
crawl reaches with each file's **own** self-signed root, for signatures and for §12.8.5's four steps — a file
vouching for itself, an upper bound rather than a verdict, and the figures are the command's. Ten §12.8 rows
moved to `implemented`; §12.8.3.4.8 stayed, its note now naming the clause sentence that holds it rather than
the anchor.