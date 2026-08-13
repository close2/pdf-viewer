# The stack, and what is deliberately not in it

Moved out of `CLAUDE.md` so that the principles file holds only principles. The *rationale*
for most of these — why Rust, why not raw Vulkan, why CPU first, what the image codecs cost —
is `doc/PLAN.md` §1; `doc/crate-map.md` says which crate each choice lives in.

## Stack

| Area | Choice |
|---|---|
| Language | Rust |
| Rasterizer | GPU first, for page one and every page after it; `tiny-skia` as the correctness oracle and as the fallback for a frame the device refuses — behind one trait |
| Fonts | `skrifa` (+ Type1/Type3 handled in-tree) |
| Windowing | `winit` |
| Dialogs | `ashpd` (XDG desktop portal — native KDE dialogs, any toolkit) |
| Accessibility | `AccessKit` (AT-SPI on Linux) |
| Parallelism | `rayon` |
| Deflate | `flate2` with `zlib-rs` backend (pure Rust, ~C speed) |
| Spec model | Arlington PDF Model → generated validation layer |

**Not used:** `rustybuzz`. PDF content streams carry already-positioned glyphs; shaping
them again would move glyphs away from where the document specifies. It may return later,
scoped strictly to text *we* generate (annotations, form fields with non-embedded fonts).

## The public-key cryptography is in the tree, and the reason is not "not invented here"

§12.8's signature verification takes **no dependency at all**: `rsa`, `x509-cert` and the
RustCrypto elliptic-curve packages were each measured and declined, and `crates/pdf-model`'s
`bigint`, `pkcs1`, `dsa`, `x509`, `cms` and `der` are what runs instead. ADRs 0229 and 0314 have the
arguments; three sentences of them belong here because they are what a round choosing a dependency
needs.

- **The argument ADR 0031 made for taking the ciphers has no instance in a *verifier*.** That
  argument is about defects that leak a secret — timing and cache side channels on key material —
  and a signature verification has no secret in it: every number is out of a file a stranger wrote.
  The one class of defect that does matter, a verifier that accepts a forgery, is ruled out by
  construction rather than by review (RFC 8017 section 8.2.2 compares whole encoded blocks; FIPS
  186-4 section 4.7 step 1 bounds `r` and `s`).
- **A general ASN.1 decoder would lose real documents.** `x509-cert` sits on a strict DER reader,
  and four of the corpus's ten signature values begin `30 80` — X.690 clause 8.1.3.6's indefinite
  length, which DER forbids and Adobe's handler emits. `openssl pkcs7` refuses exactly those four.
- **Elliptic curves are where that argument *does* have an instance, and the blocker is different
  from the arithmetic.** ISO/TS 32002 names eight curves across two group laws, and their domain
  parameters are in no document this tree holds — so taking them means taking somebody's constants,
  which is a dependency decision rather than a coding one, and the available packages cover two of
  the eight. ADR 0314 says what would change that answer. **The `elliptic-curve`, `p256`, `p384` and
  `ecdsa` packages are therefore declined-for-now rather than declined**, which is a different thing
  from `rustybuzz` above.
- **One dependency is owed and is not an argument, only unspent**: ISO/TS 32001 adds SHA3-256,
  SHA3-384, SHA3-512 and SHAKE256 to Table 260, and this tree computes none of them. It needs a
  SHA-3 implementation on the same `digest` 0.11 line as the `sha2`, `sha1`, `md-5` and `ripemd`
  already here — the *second* hash stack is exactly what ADR 0229 declined `rsa` 0.9 for, so the
  line matters more than the package.
