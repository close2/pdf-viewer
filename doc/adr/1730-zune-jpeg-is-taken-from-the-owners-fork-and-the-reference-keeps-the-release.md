# 1730 — `zune-jpeg` is taken from the owner's fork, and the reference renderer keeps the release

Session 1447. Status: **accepted** and **built**. Carries out the round's half of
`doc/questions/A227` that ADR 1589 section 3 left, now that the owner has pushed the fork; amends
ADR 1589 section 2's "nothing else in the tree takes `zune-core`", which did not hold.
Context: ADRs 0014, 1495, 1520, 1589, 1714, 1718; `doc/patches/zune-jpeg-*.patch` and their reports.
Code: `Cargo.toml` (the `zune-jpeg` stanza), `Cargo.lock`, `fuzz/Cargo.lock`, `deny.toml`
(`allow-git`), `tools/main-checkout.py` (`patches`, `patch_header`), the two patches' preambles.
Tests: `crates/pdf-model/tests/banded_decodes.rs` (the DC test, no longer ignored, and
`a_complete_last_row_without_its_eoi_is_decoded_as_the_frame`).

## 1. The revision is the release and the two patches, and nothing else

`close2/zune-image` branch `pdf-viewer/0.5.15-with-fixes` is at
`1c7d01b815932b3722766ee88f59b4d61f91771d` (`git ls-remote`, 2026-10-08). Its first parent chain is
`31d81fed` — the commit crates.io's 0.5.15 names in `.cargo_vcs_info.json`, whose `crates/zune-jpeg/src/`
is byte for byte the registry's `src/` — and then two commits. Both patches, applied to `31d81fed`
with `git apply --directory=crates/zune-jpeg` as their preambles say, give a tree whose diff against
`1c7d01b8` is zero bytes; the two commits touch `bitstream.rs` and `mcu.rs` only, and no line of that
diff says `unsafe`. The fork's `crates/zune-core/src/` is crates.io's 0.5.1 byte for byte. So the
manifest's line is the stanza ADR 1589 wrote out, with this `rev`.

## 2. Two packages of each name, not one, and why not `[patch]`

ADR 1589 section 2 said nothing else in the tree takes `zune-core`. **The reference renderer does**:
`tools/hayro-compare`'s `hayro-syntax` 0.7.2 takes `zune-jpeg` `^0.5` from crates.io, and with it
`zune-core`. A git source is a different package from the registry's, so after the move the root lock
holds `zune-jpeg` 0.5.15 and `zune-core` 0.5.1 twice each, one from each source; `cargo deny check`
names both as `warning[duplicate]` (38 duplicates where there were 36) and passes, since
`multiple-versions` is `warn`. The fuzz lock reaches no `hayro`, so it holds the fork's copy alone.

**`[patch.crates-io]` would make it one package and is declined.** It would put this tree's two fixes
into the reference renderer's decoder, and a reference that shares this tree's code is evidence that
agrees for the wrong reason (trap 9; `CLAUDE.md` principle 5). Its JPEG decodes stay the release's.
A `[patch]` in the root manifest would also not reach `fuzz/`, a workspace of its own, while the
stanza does: `pdf-model`'s `zune-jpeg.workspace = true` resolves against the root manifest whichever
workspace builds it, which is why `cargo update --manifest-path fuzz/Cargo.toml -p zune-jpeg` moved
the fuzz lock to the fork; a fuzz lock left on the registry's copy is refused by `--locked`, so
the two locks cannot disagree on the source silently. The cost is `zune-jpeg` compiled twice in a build that selects
`hayro-compare`, about 9 700 lines.

## 3. Features: the worker has no JPEG decoder, and the host's copy asks for what it did

`worker_features.rs`'s question, asked of this package: `pdf-sandbox`'s resolution reaches no
`zune-*` package, before the move or after, because §7.4.8's decode runs in this process and not in
the confined worker (ADR 0014). `cargo tree -e normal -f '{p}|{f}'` gives the fork's `zune-jpeg`
`default, neon, std, x86` under `-p pdf-model` and under `--workspace` alike, which is what the one
package was given before; the registry copy, now `hayro`'s alone, gets `neon, std, x86`. The crate
forbids `unsafe` only when neither `x86` nor `neon` is on (its `lib.rs`), and its `src/` has 96 lines
naming `unsafe` by grep in the release and in the fork alike; that posture is ADR 0014's and ADR
0031's and is unchanged here.

## 4. What it changes on a page

The DC patch changes no sample a release build produces: it makes the dev, test and fuzz profiles
wrap where release already wrapped. The scan patch changes samples, and only on a baseline scan whose
data holds every MCU the frame counts and ends with no `EOI` (ADR 1520): its last MCU row is now the
frame's where it was 128. **Measured, it moves nothing a document holds.** Every `DCTDecode`
codestream `pdfimages -j` takes out of the 974 tracked documents (179) and out of one crawled document
in thirty, taken in sorted order (31 016 codestreams over the 1 428 of 2 977 documents that have one),
decoded by both decoders in one release binary with `jpeg_options`'s settings: identical bytes for
every codestream both read, and no codestream one reads and the other refuses. The census is
calibrated: the 348-byte reproducer passed through it with them is reported as differing at byte
31 200, which is line 104 of 100 RGB samples. `raster_golden` holds all 974 first pages unmoved.

## 5. What retires the pin, and what the owner still owes

The stanza returns to a crates.io version the day a release carries both fixes; the two upstream
reports beside the patches are still unfiled, and `tools/state.sh main-checkout` lists them as the
owner's until each carries a `Filed:` line. With the fork pinned it counts both patches as applied
and no longer lists a fork to create.
