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
| Clipboard | `arboard`, **in `viewer-ui` only** — the two native hosts ask `gdk::Clipboard` and `QClipboard`, and the C ABI hands a caller the text (ADR 0519) |
| Accessibility | `AccessKit` (AT-SPI on Linux) |
| Parallelism | `rayon` |
| Deflate | `flate2` with `zlib-rs` backend (pure Rust, ~C speed) |
| Spec model | Arlington PDF Model → generated validation layer |
| FUSE | `fuser`, pinned `=0.18.0`, default features — its pure-Rust `/dev/fuse` path, no libfuse and no C linkage (ADR 0861) |
| KIO | CMake, extra-cmake-modules, `Qt6::Core` and `KF6::KIOCore`, for `kio/` alone — **outside the cargo workspace**, no `Cargo.toml`, named by no manifest, so a machine with no KDE builds and tests the whole workspace unchanged and the one test that reaches it skips saying which package is missing (ADR 0869) |

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
- **Elliptic curves are taken, and what refuses now is four curves rather than a family** (ADR
  0532, measured 2026-08-23; supersedes the refusal ADRs 0314 and 0331 recorded). `p256`, `p384`
  and `p521` 0.14.0 plus `ed25519-dalek` 3.0.0, all stable on this tree's `digest` 0.11 line, with
  `ecdsa` named directly so one generic verification serves three curves. **23 new locked
  packages** plus two patch bumps, `cargo deny` green on all four sections with no exception added
  — `BSD-3-Clause`, which `ed25519-dalek` and `curve25519-dalek` are under and nothing else here
  is, was already in the allow list. `default-features = false` everywhere, and
  `precomputed-tables` is deliberately **off**: it trades image size for scalar-multiplication
  speed, and a signature is verified a handful of times per document off the launch path while
  every byte of a table is paged in at launch.
  - **What is not takeable, re-measured rather than quoted**: `bp256` and `bp384` are
    0.14.0-**rc.15** and their stable 0.6 is the old hash line; **`bp512` does not exist on
    crates.io**; `ed448-goldilocks`'s stable 0.9.0 has the field arithmetic and **no signature
    scheme**, on `rand_core` 0.6, and its 0.14 line is a pre-release. So four of ISO/TS 32002's
    eight curves are refused by *package availability*, each named at runtime by its own
    identifier.
  - **`const-oid`'s `db` feature is the identifier supply, at zero new packages** — it is already
    here through `digest` — and it is what keeps a dozen object identifiers out of this project's
    memory. It also gave `cms::Digest` a second reading for six of its ten, retiring ADR 0390's
    "SHAKE256 has no second reading" as a claim that had outlived its reason.
  - **`curve25519-dalek`'s SIMD backend carries `unsafe`**, selected by its build script on
    x86_64. This is the same shape as `sha2`, `cmov`, `block-buffer` and `hybrid-array`, all
    already on `pdf-model`'s path, and `#![forbid(unsafe_code)]` is unchanged: it is
    compiler-enforced over *this project's* source. Written down beside the dependency in
    `Cargo.toml` rather than left to be discovered.
- **The one dependency that was owed is spent, and cost twice what this file predicted** (ADR
  0390, 2026-08-17). ISO/TS 32001 section 5.1.4 adds SHA3-256, SHA3-384, SHA3-512 and SHAKE256 to
  Table 260 — **not** to Table 256, whose section 5.1.3 Errata Collection 3 deletes outright — and
  `cms::Digest` computes all four. The line test was the deciding one and both
  candidates passed it — `digest` 0.11, the same trait stack as the `sha2`, `sha1`, `md-5` and
  `ripemd` already here, because a *second* hash stack is exactly what ADR 0229 declined `rsa` 0.9
  for and ADR 0348 declined `ttf-parser` for. What this file got wrong was the package count:
  **`sha3` 0.12.0 removed `Shake128` and `Shake256` into a separate `shake` crate**, so the current
  line is **two** packages and **four** compiled ones (`sha3` 0.12.0, `shake` 0.1.0, and the shared
  `keccak` 0.2.1 and `sponge-cursor` 0.1.0), all `MIT OR Apache-2.0`, MSRV 1.85, all RustCrypto.
  The two-package alternative was `sha3` 0.11.0, which still has both and is superseded; a dead
  minor was judged the more expensive of the two. **The lesson generalises past this row**: a
  sentence here naming a *version* of somebody else's crate is a prediction, and a prediction is
  re-measured at the moment of spending rather than read.
