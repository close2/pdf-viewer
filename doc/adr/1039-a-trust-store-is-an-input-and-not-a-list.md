# 1039 — A trust store is an input, and not a list

Session 1022. Status: **accepted**.

§12.8.1 divides a signature into three questions and this project has answered the first two since
the three-hundred-and-seventy-seventh and three-hundred-and-ninety-second sessions (ADR 0215, ADR
0229). The third — *is the signer anyone to believe* — has stood open ever since, and the ledger
records it as "a trust store and a network" in nineteen of §12.8's twenty-seven non-`implemented`
rows. This ADR decides what a trust store **is** for this program, so that no later round has to
decide it again, and builds RFC 5280 section 6.1's path validation on top of that decision.

## 1. The decision

**A trust store is an input to path validation, supplied by the host, and this program ships no
certificate list and reads no platform store.**

`pdf_signature::trust::validate` takes `&TrustAnchors` and an `Instant`; `pdf_signature::signature::
Signature::trust` passes them through. The crate holds no root, opens no file, opens no socket and
asks no clock. The empty set is the default and answers `Trust::NoAnchorSupplied`, which is a
statement about this program rather than about the document.

Two documents say the same thing from two directions and neither is a convenience argument.

RFC 5280 section 6.1 makes the anchors input (d) of nine and says whose choice they are:

> The selection of a trust anchor is a matter of policy: it could be the top CA in a hierarchical
> PKI, the CA that issued the verifier's own certificate(s), or any other CA in a network PKI.

`CLAUDE.md` principle 3 says the policy is asked once, in a place a host can supply, never
hard-coded at the point of the operation, and that "a refusal that cannot become an *ask* is the
thing to avoid". A trust decision is exactly such a policy, and it is the sharper case rather than
the softer one: the four levels there are about what a *document* asserts over its reader, while
this is about what the *reader* asserts over a document, and a program that chose the answer for
the reader would have made the same mistake in the opposite direction.

## 2. The two alternatives, priced

**A compiled-in root list.** About 150 roots, some 250 KB of `static` data — affordable by principle
2's rule, since compiled-in data costs no parse time at launch. It is refused for three reasons, and
the third is the one that would still hold if the first two were solved.

- It is a policy hard-coded where no host can reach it, which principle 3 forbids outright. It could
  be made overridable, and then it is a *default* policy this project chose on a reader's behalf.
- The only maintained lists on crates.io are TLS **server-authentication** root programs —
  `webpki-roots` is Mozilla's, and Mozilla's program admits a CA for the web. The trust lists that
  actually govern PDF signatures are the EU Trusted List and Adobe's AATL, neither of which is a
  package and both of which are somebody's regulatory or commercial programme rather than a
  specification. Shipping a TLS list as a document-signing store is a category error that would
  produce confident wrong answers in both directions.
- It would put this project in the business of maintaining a root programme, which is a continuous
  obligation with a security consequence and no end date.

**The platform's trust store.** `rustls-native-certs`, or `/etc/ssl/certs` directly. Refused as a
*default*, and explicitly **admitted as a host's choice**: a host that wants it reads the store and
passes the anchors in, which is what the shape above is for.

- Same category error as above. A platform's store is its TLS store.
- It is a filesystem read, and principle 3 gives the renderer no filesystem. It would have to happen
  in a host process — which is where a host-supplied input already lives, so the architecture points
  at the same answer the policy rule does.
- It makes a verdict a function of the machine. This tree's oracle rests on interpretation being a
  function of the bytes; a signature report that says one thing on a developer's laptop and another
  in CI is not reproducible, and nothing would tell a round which of the two it was reading.

## 3. What was built, and what it refuses rather than assumes

`crates/pdf-signature/src/trust.rs` is RFC 5280 section 6.1, reduced under the RFC's own permission
— "Clients that do not support these extensions MAY omit the corresponding steps in the path
validation algorithm" — and the reduction is made safe by the sentence beside it: "clients MUST
reject the certificate if it contains an unsupported critical extension."

Done, in section 6.1.3's and 6.1.4's lettering: **(a)(1)** the issuer's signature over each
`tbsCertificate`, verified with the same five modules question 2 uses; **(a)(2)** the validity period
against the caller's instant; **(a)(4)** issuer-name chaining; **(k)** `basicConstraints` with `cA`,
with version 1 and 2 intermediates rejected under the step's own permission; **(l)** and **(m)**
`max_path_length` and `pathLenConstraint`; **(n)** `keyUsage` asserting `keyCertSign`; **(o)** every
unrecognised critical extension, as a refusal.

Not done, and said in the type rather than in a comment:

- **Revocation, section 6.1.3 (a)(3).** `Trust::Anchored` carries `Revocation::NotChecked` and there
  is no second variant to carry. Nothing this module returns says *valid* or *trusted*, which is the
  discipline `Authenticity` already keeps.
- **The policy tree** — 6.1.2 (a), 6.1.3 (d)–(f), 6.1.4 (h)–(j), 6.1.5 (g). The module fixes
  `user-initial-policy-set` to `any-policy` with no initial policy inhibits, under which 6.1.5's
  success condition holds for every path. **The argument that makes this sound, and not merely
  convenient**: the only thing that can drive `explicit_policy` to zero is `requireExplicitPolicy`
  in a policy constraints extension, and section 4.2.1.11 says "Conforming CAs MUST mark this
  extension as critical" — so a certificate that would have needed the omitted step is refused by
  (o) instead of waved through. Section 4.2.1.10 says the same of name constraints and 4.2.1.14 of
  inhibit anyPolicy. Three omissions, one guard, and the guard is the standard's own.
- **Name matching is byte comparison**, not section 7.1's. It can fail a path that should chain and
  can never join two names one authority did not, which is the direction to be wrong in.

Bounds, because a chain is a stranger's: `MAX_PATH_LENGTH` 8, `MAX_CANDIDATES` 64, `MAX_STEPS` 256,
no certificate twice in a path — which is also section 6.1's own rule. A certificate written with
X.690's indefinite lengths is refused by name: `der` accepts them for the reason ADR 0215 records,
and a `tbsCertificate` whose extent depends on scanning for an end-of-contents marker is not an
encoding its issuer signed in any checkable sense.

## 4. What this does not close, and why the rows stay where they are

**No host in this tree supplies an anchor**, so nothing a person sees changes and every sentence
`viewer_core::notes` prints about the third question is still true. That is the shape
`doc/todo/02`'s list calls "a capability that reached the crate and never reached the program", and
it is written here rather than left to be discovered: what the next round owes is the host end — who
supplies the anchors, and what the four levels of principle 3 mean for a signature whose path did
not validate.

Nineteen ledger rows name this debt and **not one of them becomes `implemented`**, because every one
of them also names revocation, a network, or both. What changes for them is that the missing half is
now half as large and is named precisely: §12.8.3.4.5 (b) is done and (c) and (d) are not.
