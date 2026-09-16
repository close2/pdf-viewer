# Q66 — Where does §7.6.5's public-key crypto stack live?

Source: round 1139, reading §7.6.5 (public-key security handlers) against the tree.

## The decision that is not a round's to take

§7.6.5's decryption needs three things on top of what `pdf-syntax`'s `crypt.rs` already has
(AES-CBC, SHA-256): a CMS `EnvelopedData`/`RecipientInfo` reader (RFC 5652), X.509 recipient
matching, and RSA **decryption** — RSADP, `c^d mod n`, with the reader's private key as the
exponent. None of the three is a new cryptographic *primitive* for the project: the modular
arithmetic is `pdf_signature::bigint::modpow` over `crypto-bigint`, and `pdf_signature`'s `der`,
`cms` and `x509` already read a certificate's `subjectPublicKeyInfo` and a `SignedData`.

**They are all in `pdf-signature`, which depends on `pdf-syntax`.** `crypt.rs` is in `pdf-syntax`,
so it cannot call any of them without a dependency cycle, and `pdf-syntax` carries no `der`,
`pkcs1`, `x509` or `crypto-bigint` of its own. No `EnvelopedData` parser and no RSA-decryption
path exist anywhere in the tree — `cms.rs` reads `SignedData` only. So §7.6.5 is buildable only
after a structural choice, and `doc/stack.md` is explicit that the CMS/X.509 stack stays in **one
place** — which is exactly the invariant either option below touches.

## The two shapes

- **A public-key stack added to `pdf-syntax`.** `crypt.rs` grows its own `EnvelopedData` reader,
  RSADP and cert matching, pulling `crypto-bigint`/`der`/`pkcs1`/`x509` into the untrusted-input
  crate. Duplicates what `pdf-signature` holds and puts a second ASN.1/RSA surface on the attack
  boundary — against `doc/stack.md`'s one-place rule.
- **A shared crypto crate both depend on.** Extract the `der`/`cms`/`x509`/`bigint`/`pkcs1` seam
  into a crate below both `pdf-syntax` and `pdf-signature`, add `EnvelopedData` and RSADP there,
  and let `crypt.rs` and the signature code both call it. One place preserved; a real refactor of
  where `pdf-signature`'s guts live.

## Why it is low urgency

The census is five documents in ~89 000 (`/Filter /Adobe.PubSec`, every one `adbe.pkcs7.s5`), and
**none of the five has a recipient certificate that could belong to this reader**: four are one
Apache bug report's attachments and the fifth is encrypted to a device (`zune-tuner://…`). So a
finished implementation would still refuse all five for want of a match, and §7.6.5.1's one reader
`shall` — scan the list and "attempt to find a match with a certificate that belongs to the user"
— is satisfied vacuously by the refusal today. The robustness gain is zero documents; the coverage
gain is the clause.

## What the tree does meanwhile

The refusal now names §7.6.5 (was §7.6.4) and quotes the handler and subfilter (`crypt.rs`, and a
calibrated test with a control). `doc/adr/1134` records this deferral and the host-supplied
private-key shape a build would use.

## Recommendation

**The shared-crate shape**, when it is built at all, because `doc/stack.md`'s one-place rule is the
reason the CMS/X.509 parsing is in the tree and duplicating it onto the untrusted boundary is the
thing that rule forbids. Not now: no document needs it, and the refactor is larger than the
robustness it buys. The question is whether the owner agrees the shared crate is the shape, and
whether §7.6.5 waits behind a document that would actually decrypt.
