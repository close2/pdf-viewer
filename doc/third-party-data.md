# Third-party data and dependency decisions

Status: **record** — what was read, off copies on this machine, and what each decision cost.
Read by: whoever is about to vendor data or take a dependency. The licence obligations this
lists are met by `/NOTICE` and checked by `viewer-ui/tests/notices.rs`.

`doc/HANDOVER.md` §1 is the pointer to this file. ADR 0133 cites "`HANDOVER.md` §1" and means
this.

**This project is MIT** as of the hundred-and-thirtieth session (relicensed from MPL-2.0; one
author in the whole history, so nobody else's consent was needed). `deny.toml`'s allow-list
dropped MPL with it.

**All three items shipped**, in the order this section used to prescribe, and the table stays
because it is the record of what was read — off copies on this machine, not recalled:

| data | source examined | terms |
|---|---|---|
| Adobe predefined `CMap`s | `poppler-data`'s `COPYING.adobe` (1990–2019), `doc/pdf.js/external/bcmaps/LICENSE` (2009), `hayro-cmap`'s `assets/LICENSE.txt` (2023) | **BSD-3-Clause** |
| Foxit standard-14 programs | `doc/pdf.js/external/standard_fonts/LICENSE_FOXIT`, from PDFium | **BSD-3-Clause** |
| Liberation Sans | `LICENSE_LIBERATION` | **SIL OFL 1.1** (reserved font name: ship and use freely, do not modify and keep the name) |
| poppler's `cidToUnicode`, `nameToUnicode`, `unicodeMap` | `poppler-data`'s `COPYING` | **GPL-2 or GPL-3** — Glyph & Cog's, *not* Adobe's |

`BSD-3-Clause` costs three obligations: reproduce the notice and disclaimer "in the documentation
and/or other materials provided with the distribution", keep them in source, never use Adobe's or
Google's name to endorse this. **The surface for that is `/NOTICE`** — at the repository root,
`include_str!`d by `pdf-viewer --licences`, put over the page by `?` as the About panel, set in
§9.6.2.2's own Courier and deliberately *not* re-wrapped, because re-flowing text a licence
obliges this program to reproduce would be editing it. `viewer-ui/tests/notices.rs` checks that
every `.pfb` and `.ttf` under `data/` is named **by file name**, that the required sentences are
verbatim, and that the bytes still hash to `SHA256SUMS` — that test exists because `cargo deny`
reads Cargo metadata and **cannot see vendored data**.

**The trap is the last row.** `poppler-data` is two data sets under two licences and says so. A
`CMap` gets code → CID; getting a CID to a glyph in a **non-embedded** CJK font needs CID →
Unicode, which is the GPL half. The permissive equivalent is Adobe's own `Adobe-Japan1-UCS2`,
`Adobe-GB1-UCS2`, `Adobe-CNS1-UCS2`, `Adobe-KR-UCS2` — BSD files inside the `cMap` directory. For
an *embedded* CIDFont none of it is needed: the font's own charset or `/CIDToGIDMap` answers.

**What shipped**: `data/standard-fonts/` holds §9.6.2.2's fourteen font programs, 804 KB, so
those pages reproduce on any machine and `substitute.rs` is no longer the only machine-dependent
code in the tree (ADR 0133); `data/cmaps/` holds all 239 `CMap`s Adobe publishes, deflated one at
a time by `build.rs` into a 3.9 MB blob and inflated only when a document names one, so nothing is
decompressed at startup (ADR 0140). §9.10.2's third method came with the second, and it is where
the gates moved: 15 documents left the incomplete list, 9 more oracle pages agree, the readback
went 97.9% → 98.2%.

**Three things worth keeping out of the first**: PDFium's `.pfb` files are bare CFF programs
(`01 00 04 02`), a name-keyed substitute is addressed by glyph *name* and so needs no Adobe Glyph
List step — which is why `Symbol` and `ZapfDingbats` work — and a *composite* font cannot use them
at all, because §9.7.4.2 leaves it reachable only through `/ToUnicode`, which addresses by
character.

**What none of it fixed** is the 40 fonts naming an `Identity` ordering, where the codes index a
font nobody supplied: [todo 21](todo/21-font-substitution.md).

**And a todo file's claim decays exactly as a ledger row's does, with no sweep watching it.**
That file named two documents whose "characters no single face on this machine has" and the
two-hundred-and-fifty-sixth session opened the pictures: both draw every character, and had since
ADR 0153's coverage rule landed seventy-three sessions earlier. The claim was a *prediction* about
that rule which nobody re-checked after it shipped. `doc/todo/01`'s five sweeps read `ledger.toml`
and `crates/`; **`doc/todo/` is a third population and is watched by nothing**.

**And one dependency decision, taken in the three-hundred-and-seventy-sixth session** —
`accesskit` 0.24.1 and `accesskit_unix` 0.22.1, both **MIT OR Apache-2.0**, with **61 packages**
behind them, every one MIT, Apache-2.0, Zlib or Unlicense-or-MIT. `cargo deny check` is clean on
all four with no new exception in `deny.toml`. Two things about it are worth keeping here rather
than in ADR 0214: **`memchr` is not among them at runtime** — it arrives through `winnow` and
`proc-macro-crate`, which are a proc macro's build dependencies, so the rule ADR 0186 refused
`quick-xml` over is intact and `cargo tree -e normal` is how that was checked; and **the one async
runtime in this tree is `accesskit_unix`'s**, which lives on a thread of its own, is named by one
crate, is `cfg(target_os = "linux")` in that crate's manifest, and is not created until the first
frame has been presented.

**And a second, in the three-hundred-and-seventy-seventh** — `sha1` 0.11.0 and `ripemd` 0.2.0, both
**MIT OR Apache-2.0**, from the same RustCrypto family as the `sha2` and `md-5` §7.6 already brought
in, **two new packages between them** and no transitive dependency this tree did not already build
(`cargo tree -e normal` shows only `digest`, `cfg-if` and `cpufeatures`, all of which `sha2` already
pulls). They exist because §12.8.3's Table 260 and Table 256 name six digest algorithms between them
and the four already here are not all six; implementing five and being silent about the sixth is the
failure this project spends its rounds removing. `cargo deny check` clean on all four with no new
exception (ADR 0215).

**And in the three-hundred-and-ninety-second, a dependency decision that came out `no`** — the
first in this record that adds nothing. Answering §12.8.1's *second* question needed an X.509
certificate reader and an RSA verification, and `rsa`, `p256`/`p384` and `x509-cert` were all
declined: `rsa` 0.9.10 brings **31** packages including a second `digest` 0.10 hash stack beside
this tree's 0.11, `rsa` 0.10 is a release candidate, `p256`+`p384` bring **28** and cover two of the
five curves RFC 5480 names, and `x509-cert`'s strict DER would refuse the four corpus signatures
that state indefinite lengths. Both modules are in tree — `pdf_model::x509` at 213 lines of code and
`pdf_model::pkcs1` at 356 — under `#![forbid(unsafe_code)]`, and **`cargo deny check` is clean on
all four checks with no package added and the licence position unchanged**. ADR 0229 has the
argument, including what would change the answer: an ECDSA or DSA signature arriving in a real file.

**And in the four-hundred-and-eighth, `gtk4` 0.11.4 — a *toolkit*, which is a first for this
record.** `doc/todo/30`'s order made GTK4 the first native host, and the crate is the GNOME project's
own GIR-generated binding. **41 packages** arrive with it — `glib`, `gio`, `gdk4`, `gsk4`, `pango`,
`cairo-rs`, `graphene-rs`, their four `-sys` crates, and the `system-deps`/`pkg-config` machinery
their build scripts use — and **every one of them is MIT**, which is this project's own licence, so
the answer to "what may I do with a build of this?" is unchanged. `cargo deny check` is clean on all
four checks with no new exception in `deny.toml`.

Three things about it belong here rather than in ADR 0244. **It is confined to one crate**: `gtk4`
is named by `crates/viewer-gtk`'s manifest and by no other, nothing that touches PDF bytes gains a
toolkit dependency, and `viewer-ui`'s and `viewer-confined`'s graphs are byte-for-byte what they
were. **It costs no `unsafe` in this tree**: `viewer-gtk` and its binary both keep
`#![forbid(unsafe_code)]`, because `gtk4-rs` is a safe binding and the `unsafe` is inside
`gtk4-sys` and `glib` — which is the property `doc/todo/30` chose GTK4 for and the property the Qt
host will not have. And **it is linked against the platform's own libraries** rather than vendored:
a native host depending on the desktop being installed is the point of a native host, and it is why
`viewer-gtk` is excluded from the two cross-target checks by the same rule that makes
`viewer-accessibility` Linux-only in its own manifest — `glib-sys`'s build script wants a
cross-compiling `pkg-config` and there would be no GTK 4 development files for the target anyway.

**And in the four-hundred-and-tenth, `cxx` 1.0.199 and `cxx-qt-build` 0.9.1 — a *C++ bridge*,
which is a second first for this record.** `doc/todo/30`'s order put Qt second precisely because it
costs one, and this is what the one costs. **23 packages** arrive, and **three of them reach a
shipped binary**: `cxx`, `cxxbridge-macro` and `link-cplusplus`. The other twenty are build-time
only — `cxx-qt-build`, `cxx-gen`, `qt-build-utils`, `clang-format`, `which`, `codespan-reporting`
and the rest — and appear in no `cargo tree -e normal`. Every one of the 23 is MIT, Apache-2.0, or
Unlicense-or-MIT, all already on `deny.toml`'s allow list, so the answer to "what may I do with a
build of this?" is unchanged again; `cargo deny check` is clean on all four checks with **no new
exception**.

Four things about it belong here rather than in ADR 0246. **It is confined to one crate**: `cxx`
and `cxx-qt-build` are named by `crates/viewer-qt`'s manifest and by no other, nothing that touches
PDF bytes gains either, and `viewer-ui`'s, `viewer-gtk`'s and `viewer-confined`'s graphs are what
they were. **`cxx-qt` itself was declined**, which is the part worth recording as a decision: the
crate that makes a Rust type into a `QObject` is built for QML and links two initialisers a Widgets
host never calls, and only the *build* half — finding Qt through `qmake6`, running `moc`, linking
QtCore/QtGui/QtWidgets — is wanted, and it is available on its own. **It costs one hand-written
`unsafe` token**, the `unsafe extern "C++"` block header, which is `cxx`'s way of asking the author
to assert that the C++ declared there exists and is safe to call; `viewer-qt` holds
`#![deny(unsafe_code)]` with one exemption on `mod bridge`, and `tests/unsafe_position.rs` asserts
the file and the token and that no other crate in the tree lifts the denial. **And it is linked
against the platform's own Qt** rather than vendored, for the reason `viewer-gtk` links against the
platform's GTK: that is what a native host is, and it is why `viewer-qt` is excluded from the three
cross-target checks — `cc-rs` wants `lib.exe` for a Windows target and there would be no Qt 6
development files there anyway.

**Provenance is a principle-4 question**, and the tree has one precedent — `pdf-spec`'s Arlington
tables, built by `build.rs` from a pinned submodule. Vendored data arrives the same way: a
checked-in tool, a pinned upstream revision recorded beside the bytes, the licence file verbatim
next to what it covers.
