# 1929 — A custom document information key is carried, or stated

Session 1929, a single archive round in the rounds worktree. ADRs 2680 and 2681, Q779. No trap or habit.

**Why.** Round 1928 left `241573_ftp` refused at part 4 by the two ISO 19005-4 section 6.1.3 rows
for custom `/Info` keys. Those rows' `unmapped` key had only `discard` and `stop`.

**The premise, measured.** The number was not one. Round 1928's census refused 60 pdf.js and 80
corpus documents per stating profile at part 4 on these rows. Most of the keys are XML names; four
documents state a key holding a space.

**Built.** First, the elegant remedy (ADR 2680). §14.3.3 routes other metadata to the metadata
stream, and part 4, unlike part 2, asks for no described schema. So `unmapped = "carry"` writes the
value unchanged as `docinfo:<key>`, in a project namespace that states only provenance (the URI is
Q779's). The key is spelled by an injective `_HH_` escape. A value that is not a text string, or
that XML cannot hold unchanged, is not carried. Second, the fallback (ADR 2681): `keeps_per_object`
now covers these rows, so under `state-every-loss` such a value is removed and listed on the
statement page with the loss `document-information-entry`, the key, the value's type and the reason,
and also in the report and `xmpMM:History`. Both stating profiles name both rows. `no-data-loss`
names no second step and refuses an uncarriable value. The page of `241573_ftp` was rendered and
looked at.

**Tests.** `tests/archive/carried_information.rs`, five: the escape round trip, carry under both
profiles, removal and statement under `state-every-loss`, `no-data-loss` refusing, and
`unreadable = "discard"`. Planting the profile back to `stop` failed the carry tests. Planting the
predicate back without these rows failed the statement test.

**Census.** ADR 2669's command against round 1928's: 151 new outputs at part 4 (`state-every-loss`
97, `no-data-loss` 54), none gone. 4346 of the 4347 shared outputs match once masked. The one that
differs is `preEnc_20141025_105451`, where the history names the next reason an embedded PDF stops.
veraPDF 1.30.3 passes 148 of the 151. The 3 failures are defects outside metadata: `AIAA-2005-6173-712`
fails 6.2.10.5 (and 6.2.11.5 at 2b already), and `issue14438` fails 6.2.2-3 on an annotation
appearance. Ledger: §14.3.3 stays `implemented`, and its note names ADR 2680.

**Gates.** `cargo fmt --check`, `clippy -D warnings --all-targets` (`pdf-transform`): exit 0.
`nextest` of `pdf-transform` and `pdf-archive`: 1358 passed, exit 0. Archive tier behind the lock
(`corpus`, `archive_corpus`, `witnesses`): exit 0 each. `cargo test -p conformance`: exit 0.
`state.sh docs`: exit 0. veraPDF 1.30.3 over the 151 new outputs: 148 pass, 3 fail (above).
