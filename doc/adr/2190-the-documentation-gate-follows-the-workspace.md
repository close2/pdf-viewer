# 2190 — The documentation gate follows the workspace

Session 1677, docs. Status: **accepted** and **built**.
Context: ADR 1938. No clause is read anew and no ledger row changes status.

The held list covered nine of the workspace's 44 crates. Its success did not say
whether a link in any of the other 35 crates resolved. Cargo metadata supplies
that population: 88 doc-enabled targets map to 84 distinct output names, because
four binaries share a library name. The script-worker binary also requires the
optional engine feature. The claim here is 44 packages under native default
features, checked against their generated indexes. The baseline uses
`--workspace --keep-going`, so an earlier failure does not suppress unrelated
crates' diagnostics. A dependent binary can still wait for its library to pass:
four more CLI link cases appeared only after the libraries' errors were repaired.

The baseline reports 966 located diagnostics in 28 crates. The kept instrument
answers 849: 564 editorial brackets and 285 private-item links. Its remaining
117 cases need a reading of the named item. Answers include qualified paths after
module splits, function disambiguators, corrected method names, code spans for
private, test-only, feature-gated and nested items, and field names where an enum
variant's field is not an associated item. No visibility is widened. A removed
stale link retains its explanation; an ADR reference still carries the reason.
Three later CLI links have no source location in rustdoc's output: the instrument
cannot edit them, so their quoted sentence locates them for a manual correction.
The whole build's exit, not the instrument's edit count, decides completion.

The gate uses `--workspace`: all 44 packages are at zero, with all 84 generated
indexes present, and new members are covered automatically. This covers the native
default-feature build, not every optional feature or foreign platform. It retains
private-item documentation
and `--keep-going`. The large lane stays at 12 GiB and Cargo gets two jobs: the
first rebuild after comment edits hit the wrapper's ceiling at 12.63 GiB after
43 seconds (exit 137), whereas the two-job retry peaked at 2.20 GiB in 71 seconds.
A warm baseline's 4.27 GiB was not a safe bound for the rebuilding graph.

The behavior proof compares the isolated comment edits with snapshots taken after
the sibling implementation rounds became stable. Every changed line is a comment,
and the multiset of non-comment lines is identical; parallel semantic changes are
not mixed into that proof. The patch and full file population are kept beside the
round's logs. The C++ comment population and raw-string reader are ADR 2191's.
