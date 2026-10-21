# 1667 — A poisoned press table said as itself, a launched document at the reader's level, three readings

Session 1667, slot 5 of batch one hundred and three, a clause round; 2026-10-21. ADRs 2170, 2171; Q562.
The four defects ADR 2154 named and its two open readings. No status moved; six notes did.

| item | outcome |
|---|---|
| press lock | `Presses::press_for_profile` and the process cache answer `Result<_, NoPress>`; `Poisoned` is said as the process's fault in all three consumers, never as §11.7.2's budget |
| launch | `--launched-documents=` (refuse, ask, warn, open; `refuse` default): a program refused at every level, a document handed to `xdg-open`; four windows' paths; Q562 asks the line and the default |
| §12.5.2 | third option (the face named unembedded) priced against §9.7.4.2 and declined; row note |
| stale text | §10.4.2.4's `to_cmyk_under` path; `ink_search.rs`'s five `[ink_table]` links; `transparency_groups.rs`'s bullet; four launch sentences (`action.rs`, `command.rs`, `state-of-play.md` twice) |
| §8.7.4.5.7/8 | the `shall` is Table 52's flatness, "[t]he precision with which curves shall be rendered on the output device", via §8.4.1; stays `departed` |
| §11.7.5.3 | the report fired over an `ICCBased` group, where "only during conversion from DeviceRGB to DeviceCMYK" is carried out (trap 11); now silent where `cmyk_group` holds one |

**Premise.** The brief's §12.6.4.5 is §12.6.4.6; `press.rs` is `colour/press.rs`, lines 536 and 550. The
hypothesis held for the lock (one enum, three variants: `Unsampled` also read as "the ninth press") and
for the readings (notes, not statuses); the launch level was not ADR 1291's shape alone — it needed a
classification the standard does not state, hence Q562.

**Not fixed, named.** `Presses::rgb_route`, `named` and `presses_cached` read a poisoned table as empty or
routeless (ADR 2170 §1). §11.7.5.3's report still fires without asking whether a `DeviceRGB` colour was
painted under the pair. The level has no menu row (`ActLevel`) and no `QUORRA_POLICY_*` constant
(slot 4's `viewer-ffi`); `Policy::LaunchedDocuments` exists for both.

**Drive.** New step 94 (`launch_levels`): refuse, ask yes and no, open, and a `.desktop` refused at open,
judged by the sentence and by the drive's own `xdg-open`; with steps 90 and 91 beside it, 44 of 44 work
in `quorra`, `quorra-gtk`, `quorra-qt` and `quorra-confined` (the confined window first refused the word,
so it now carries it). No pixel moves: the poison and the report are sentences, so no arms. Calibrated:
the poison test fails with `guard` mapping to `Spent`, the black-generation test without the narrowing.

**Shared files.** `doc/conformance/ledger.toml`, `doc/state-of-play.md`, `tools/drive-windows.sh`,
`crates/pdf-model/tests/transparency_groups.rs`, `crates/viewer-core/src/{interact,lib,command}.rs`,
`crates/viewer-host/src/{lib,native_window}.rs`, `crates/viewer-qt/src/host/tests.rs` — `-U0` hunks.

**Gates.** `cargo fmt --check` 0 on pdf-colour, pdf-model, viewer-core, viewer-host, viewer-ui, viewer-qt,
viewer-gtk; clippy `-D warnings` 0 on pdf-colour `--all-targets`, pdf-model `--lib --tests`, and 0
warnings in viewer-core, viewer-host, viewer-ui, viewer-qt, viewer-gtk `--all-targets` (pdf-transform's
and pdf-render's mid-edit warnings are slots 1's and 3's); nextest pdf-model 2 122/2 122, pdf-colour +
viewer-core + viewer-host 828/828, viewer-ui 70/70; `cargo test -p conformance` 0; `--bin quotations`
ledger diverging 3 → 3; `state.sh docs` 0; drive 44/44 works; 5 600 s.
