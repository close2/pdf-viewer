# 1589 — The zune-jpeg fork is prepared for the owner to create

Session 1376. Status: **accepted**. Carries out `doc/questions/A227` up to the step only the owner
can take; amends nothing.
Context: `doc/questions/Q227`/`A227`; ADRs 0014, 1447, 1463 (the `hayro` fork's patches), 1495,
1520; `doc/patches/zune-jpeg-*.patch` and their `.md` reports.
Code: `Cargo.toml` (the comment above `zune-jpeg`), `tools/main-checkout.py` (`patch_header`,
`patches`), `crates/pdf-model/tests/banded_decodes.rs`
(`a_dc_prediction_past_i32_is_decoded_rather_than_aborting`, ignored).

## 1. The base is a commit, not a tag

Upstream tags no `zune-jpeg` 0.5.15. The published crate's `.cargo_vcs_info.json` names
`31d81fed7551c8ccea456d9d8e2b1fd8bebb6995` with `path_in_vcs = "crates/zune-jpeg"`, and that
commit's `crates/zune-jpeg/src/` is byte for byte the published `src/`. Both patches apply there,
alone and together, with `git apply --directory=crates/zune-jpeg`, and neither had drifted, so
neither was regenerated: their paths stay the crate's own, which is the form an upstream reviewer
of the crate reads. The preamble now states `Base:` as that commit, `Directory:` as the crate's
place, and `Fork:` as `https://github.com/close2/zune-image` — the name is this round's choice,
on the `close2/hayro` precedent, and the owner may choose another by editing the two preambles.

## 2. Measured against the patched decoder

In a scratch export of the tree with `zune-jpeg` patched to the clone, `cargo test -p pdf-model
--no-fail-fast` passed 1784 tests and failed one, the grey-row guard ADR 1520 wrote to fail on this
day; the ignored DC test passed there and panics on 0.5.15 here (`bitstream.rs:400`). A git
dependency on the repository brings `zune-core` from the same commit, because `zune-jpeg` names it
by path; nothing else in the tree takes `zune-core`, so one source remains.

## 3. Whose step is what

The owner's: fork, apply both patches on the base, push, and put the pushed commit in the `rev` of
the stanza `Cargo.toml`'s comment writes out, in place of the line under it; and file the two
reports beside the patches upstream, since a round holds no account to file them with. A round's,
after: `deny.toml`'s `allow-git` gains the fork beside `close2/hayro`, both locks move, the DC test
loses its ignore, and the grey-row test loses its guard for the frame's equality. `tools/main-checkout.py` prints the owner's step
as one sentence while the answer is on the disk and the manifest pins no fork, and counts the
patches as applied the day it pins `Fork:`.
