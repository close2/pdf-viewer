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
**And if it does return for that scope, `rustybuzz` is not its shape** (ADR 0348, on
§12.7.4.3's one Arabic witness): it would bring `ttf-parser`, a second sfnt stack beside
`skrifa`/`read-fonts` — the same shape ADR 0229 declined a second hash stack for — while
`read-fonts`, already here, parses the `GSUB` such work would execute. What actually blocks
shaping the one witness is not machinery at all: **no compiled-in face has one Arabic glyph**,
so the shaper question is moot until a glyph source exists, and the ADR prices both.

## The public-key *constructions* are in the tree; the arithmetic under them no longer is

§12.8's signature verification keeps `crates/pdf-model`'s `pkcs1`, `pss`, `dsa`, `x509`, `cms`
and `der` — the schemes, the budgets, the refusal names and the BER-tolerant reader — and runs
their modular arithmetic on **`crypto-bigint`**, RustCrypto's big-integer package, behind the seam
`bigint.rs` keeps (an owner decision, 2026-08-14; ADR 0331 supersedes ADR 0314's in-tree choice
and re-measures ADR 0229's table). The whole-scheme packages stay declined. What a round choosing
a dependency needs from those three ADRs:

- **`crypto-bigint` cost this tree three compiled packages** — itself, `ctutils` and `cmov`
  (`cpubits` and `num-traits` were already here; the lockfile also pins `base16ct` and `serdect`,
  optional dependencies no target compiles) — all `Apache-2.0 OR MIT`, MSRV 1.85, no default
  features (`alloc` only: the default would bring a random-number stack into code that only
  verifies public numbers). Same supplier as every cipher and digest already here.
- **The whole-scheme `rsa` crate is still disqualified by measurement**: its stable line (0.9) is
  on `digest` 0.10 — a second hash stack beside this tree's 0.11 — with ~30 packages including
  `num-bigint-dig`'s `rand`/`lazy_static`/`spin`; and the `dsa` crate, though on `crypto-bigint`
  itself, drags in signing machinery (`crypto-primes`, `rfc6979`, `pkcs8`, strict `der`) that a
  verifier never calls.
- **A general ASN.1 decoder would lose real documents.** `x509-cert` sits on a strict DER reader,
  and four of the corpus's ten signature values begin `30 80` — X.690 clause 8.1.3.6's indefinite
  length, which DER forbids and Adobe's handler emits. `openssl pkcs7` refuses exactly those four.
  This is why the CMS/X.509 *parsing* stays in tree whatever the arithmetic does.
- **Elliptic curves: the domain-parameter objection is answered and the coverage arithmetic still
  refuses.** ADR 0314 declined ECDSA partly because the curves' constants are in no document this
  tree holds; a reviewed curve crate carries them as reviewed constants, which is the same footing
  as `crypto-bigint`'s carry chains, so that premise no longer decides. What still does (ADR 0331,
  measured 2026-08-14): `p256`/`p384`/`p521` 0.14 are now **stable on the `digest` 0.11 line** but
  cover three of ISO/TS 32002's six ECDSA curves and neither Edwards one — the Brainpool pair is
  release-candidate-only on that line (`bp256` 0.14.0-rc) and brainpoolP512r1 has no crate at all —
  at about twenty packages for the first curve; and of the corpus's three ECDSA witnesses, one is
  brainpoolP256r1 and two use BSI TR-03111's *plain* `r ‖ s` encoding, a specification this tree
  does not hold. Taking the packages today would close **one signature in 811**. Still
  **declined-for-now rather than declined**, which is a different thing from `rustybuzz` above.
- **One dependency is owed and is not an argument, only unspent**: ISO/TS 32001 adds SHA3-256,
  SHA3-384, SHA3-512 and SHAKE256 to Table 260, and this tree computes none of them. It needs a
  SHA-3 implementation on the same `digest` 0.11 line as the `sha2`, `sha1`, `md-5` and `ripemd`
  already here (`sha3` 0.12 is that line's package) — the *second* hash stack is exactly what ADR
  0229 declined `rsa` 0.9 for, so the line matters more than the package.
