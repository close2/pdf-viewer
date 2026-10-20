# 1631 — A duplicated profile is named DeviceCMYK, and a profile states its own component count

Slot 1 of batch ninety-nine, 2026-10-20, an archive round. ADRs 2098, 2099; no question.

**Premises and hypotheses.** ADR 2083 section 2's evidence held: the eight duplicate-profile
witnesses draw identically with the array written `/DeviceCMYK`. The hypothesis — the decline rested
on "we cannot know the two profiles are the same" — did not: the validator already compared them by
ISO 19005-4 section 6.2.4.2's two tests. What the decline rested on was siting and §8.6.7, and both
fall. Siting: the rewrite reaches every array naming the profile, so it needs the survey's uses
(the validator's walk, now one shared reading, `pdf_archive::cmyk_profile_uses`) and not the array
each went through. Value: §8.6.7's mode acts only with overprinting on, and ISO 19005-4 section
6.2.4.3 and §11.7.2 read `DeviceCMYK` through the very profile. "Six documents convert" held as
eight: the six of the first row and the second row's two `t01` alternates. Habit 133 kept: site
identifiers off `--remedy-sites`, then `archive_unbuilt --who` named the base-standard row's two
witnesses (`6-2-3-2-t01-fail-b`, `-undefined-a`), both its `N` sentence.

**Built.** ADR 2098: `Rewrite::DuplicateProfileWrittenDeviceCmyk`, `Stated` under both rows, every
`[/ICCBased P]` but a group's `CS`; refused where another intent's or `ICCBased` group's profile is in
force at a use, beside a group space the survey has no profile for, where any graphics state turns
overprinting on, at a `DefaultCMYK` or a shared group array, or after a stopped walk. ADR 2099:
`Rewrite::IccComponentCountRestated`, `N` written from the header's count, which this reader draws
with; refused for a profile it does not draw through or a class or space §8.6.5.5 does not admit.

**Pixels.** All ten witnesses convert to PDF/A-4 and veraPDF 1.30.3 passes each (every source
fails); 20 of 20 pages byte-identical to the source at 72 and 720 dpi, drawn by HEAD's renderer.

**Gates.** In an export of HEAD plus these files while a neighbour's `render-cpu` edit failed
clippy (`tools/batch.sh export 1631 --as mine`): `cargo fmt --check -p pdf-archive -p
pdf-transform` exit 0; `RUSTFLAGS=-D warnings clippy` both `--all-targets` exit 0; nextest both 870
passed, 0 failed. `tools/state.sh --round 1631 archive` exit 0: `--remedy-sites` 4 17 of 56 (20 of
59 at HEAD), 4f 18 of 55, 4e 18 of 54, 2b 23 of 67; validator `missed` 0, `over` 0 every target;
`archive_corpus` 4 243 converted/116 refused (235/124 in round 1623's record), 2b 492/119;
`cross_check` 0 disagreements. `t3-foreign_corpus` under `--clock` exit 0, 146 s held, 0 panicked.
In the worktree: `cargo test -p conformance` 444 passed, 0 failed.

**Unfinished.** None of the brief's; at part 4 no `not-built-yet` site with a document is left. 5 160 s.
