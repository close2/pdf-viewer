# 825 — The same recipe one target down, and a pattern that could only under-report

Date: 2026-08-29. Branch `round-825`, from `main` at `3c259925`. Parallel round, worktree `r825`,
beside 824.
ADR: [0754](../adr/0754-the-same-recipe-one-target-down.md).
Touched: two new files under `fuzz/` — `seed_cms.py` and `seed_der.py` — plus `fuzz/seed_x509.py`,
`doc/verify.md`, `.gitignore`, `crates/pdf-model/src/cms.rs`'s `fixtures` doc comment, two
`doc/conformance/ledger.toml` notes, and two new files: `doc/adr/0754` and this one. No Rust *code* changed, `tools/fuzz.sh` is
unchanged, and no ledger status moves.

ADR 0751's last paragraph named this round's subject and this round took it. What it found is that
the defect had three parts: a recipe that named one submodule, two claims in the tree counted over
that same submodule, and — found by the calibration rather than by the work — **a candidate pattern
that matched nothing anywhere and reported it as a zero**.

## Machine and load

Four rounds share 24 cores. The one-minute load average moved between 0.7 and 41 over this round's
runs. **No conclusion below rests on a rate**: every coverage figure is libFuzzer's `cov:` or `ft:`,
which are cumulative sets. Wall clock is labelled where it appears; the harvest and the fuzz runs
were `nice`d and the fuzz runs were sequential, on a machine at a load average under 3.

`tools/fuzz.sh --list` read `cms` at 788 seeds and `x509` at 1530 at the start of this round, so no
sibling had moved either corpus under it.

## 1. The population

| | |
|---|---|
| PDFs the three corpora hold | 67 460 |
| of those, `grep -alr /ByteRange … \| grep '\.pdf$'` | 706 |
| documents `signature_algorithm_census` finds a signature dictionary in, over all 67 460 | 681 |
| signature dictionaries between them | 811 |

`doc/verify.md` asked for "the eleven `/Contents` blobs the nine signed corpus documents hold". The
nine are `doc/pdf.js`'s, and running the new seeder over `doc/pdf.js` alone reproduces exactly those
eleven — plus two nested timestamp tokens the sentence could not have described, because reaching
one means walking a CMS object rather than scanning a file.

## 2. The harvest

`fuzz/seed_cms.py` over the whole tree, one process, `nice -n 15`, **1 h 30 m** wall clock:

> 1854 distinct CMS object(s): 878 a signature value, 224 inside a signer's attributes, 752 stated
> by a document

23 MB. Median object 6266 octets, p99 59 332, two over a megabyte.

**Most of that wall clock is one member of the inflation gate**, and it is here on a measurement
rather than on a guess. `/TS` is two octets where `/DSS` and `/ByteRange` are five and ten, so far
more files name it than hold one: 325 of a 6000-document sample state `/TS` and none of the other
three, inflating those costs about 34 minutes over the tree — and it finds **599 CMS objects on that
sample alone** that no other route reaches, because a document whose catalogue and `/DSS` are inside
an object stream states nothing else a scan can see. `seed_x509.py` declined a comparable trade for
118 certificates; this one is five times the return for one run of a recipe.

## 3. What the seeds bought

`cms` is one of the eight targets ADR 0751 priced as saturated — its whole search half was half a
second and bought +2 edges. Every figure is a pair read inside one run.

| corpus | seeds | `INITED` | `DONE` at the documented 50 000 | the run gained |
|---|---|---|---|---|
| the corpus this round found | 788 | **495 / 1227** | 496 / 1228 | +1 / +1 |
| with the harvest merged | 2642 | **511 / 1280** | 519 / 1290 | **+8 / +10** |
| the same, capped at 256 KiB a seed | 2632 | 511 / 1278 | 515 / 1284 | +4 / +6 |

**The seeds alone added +16 edges and +53 features.** The documented run against the old corpus adds
one of each; against the new one it adds eight and ten, and two repeats of it gave +9/+14 and +5/+7.

**That is an order of magnitude less than what the same recipe bought one target up, and it is the
result rather than a disappointment.** ADR 0751's harvest more than doubled `x509`'s edge coverage —
+1342 edges on a target reaching 955. This one moves a target reaching 495 by sixteen. The reason is
visible in the two numbers: `pdf_model::cms` reads a `SignedData` for six fields and stops, so its
whole reachable surface is a few hundred edges and 788 seeds had already found most of it. **A
saturated target is saturated against its own reader, not against its corpus**, and 1854 real
signatures cannot add coverage to code that does not exist.

What did *not* happen is equally worth writing down: **nothing regressed, and the search got eight
times more productive**. That is the same shape ADR 0751 found — a fuzzer mutates what it has, so a
corpus that states more is a corpus a run can go further from — at a tenth of the scale.

**The two megabyte-sized objects stay, and that was measured both ways.** Capping the harvest at
256 KiB a seed, which is the ceiling `seed_page.py` and `list_over_the_wire` use, drops ten seeds and
costs 2 features at `INITED` and 4 edges of search, for half a second of wall clock. The whole
harvest is 23 MB, so none of the bulk argument that justifies the ceiling elsewhere applies here.

`tools/fuzz.sh cms`, which runs `doc/verify.md`'s own invocation, on the shared corpus after the
merge: `INITED cov: 511 ft: 1280` → `DONE cov: 514 ft: 1285`, seeds 2642 → 2668, `cargo-fuzz` exit 0.

## 4. The census, and the two claims it falsified

`cargo run --release -p pdf-model --example signature_algorithm_census` over all 67 460, which is
`doc/todo/51`'s own invocation. It ran in seconds and answered a question two sentences in this tree
had answered over one submodule.

**811 dictionaries, 796 read as RFC 5652 `SignedData`, 15 refused** — "the signature value is not a
CMS `ContentInfo`", which is what §12.8.3.2's `adbe.x509.rsa_sha1` puts there and is a refusal worth
fuzzing, so route one keeps those blobs too.

| `/SubFilter` | |
|---|---|
| `adbe.pkcs7.detached` | 531 |
| `ETSI.CAdES.detached` | 190 |
| `adbe.pkcs7.sha1` | 55 |
| `ETSI.RFC3161` | 20 |
| `adbe.x509.rsa_sha1` | 14 |
| `urn:pdfsigfilter:bka.gv.at:binaer:v1.1.0` | 1 |

`cms.rs`'s `fixtures` module opens: "Four of the six signature formats §12.8.3 defines have no
witness in the 974 — no document timestamp, no `PAdES` signature, no `adbe.x509.rsa_sha1`, and
nothing using four of Table 260's six digests". **All six formats have witnesses on this disk**, and
three of the four named have them in tens or hundreds. §12.8.5's ledger row says "**No corpus
document carries a document timestamp**" and had that *re-derived* in the six-hundred-and-forty-first
session by running this same census over the 974 and getting zero — a correct measurement of the
wrong denominator. There are **20 document timestamps in 15 documents**, every one of them in the
crawl rather than in the submodule corpora.

Neither fixture is retired. Trap 8 read from the other side is why: a witness found in a crawl is a
file nobody wrote for this purpose, so it can rank a format and cannot define one. What is corrected
is that both sentences now name their denominator.

Three more figures the wider population moves:

- **186 signature values state X.690's indefinite length**, where §12.8.3.4.2's row says "four corpus
  documents write" it and names them. Both are right and the row now says which population is which.
- **338 signature values carry §12.8.3.3.2's `adbe-revocationInfoArchival`**, where that row named
  one witness by file name.
- **145 certification signatures**, by Table 257's `/P`: 122 stating none, 18 `FormFilling`, 5
  `FormFillingAndAnnotation` — where §12.8.2.2's row says "[t]he corpus's one certification signature
  states `/P 2`".

And what the census *confirmed* rather than moved, which matters because `doc/todo/51` asks for it
not to be re-opened: the two signatures stating BSI TR-03111's `0.4.0.127.0.7.1.1.4.1.3` are outside
what ISO/TS 32002 section 5.1.3's NOTE 2 admits and are correctly answered by their identifier; the
one brainpoolP256r1 key is refused by a package rather than by a clause; and no document states a
SHA-3 digest. `Signature::authenticity` verifies 775 of the 796 and names every one of the rest:
17 `NotUnderThatKey`, 6 `RangeNotInThisFile`, 3 `UnknownDigest 1.2.840.113549.1.1.5` — a file putting
an RSA *signature* identifier in the `digestAlgorithm` slot — 2 `AlgorithmNotVerifiable` and 1
`NoSignatureValue`.

## 5. Trap 13 — every route run against what it is for, and what that caught

**The first run reported zero for the route that scans a document's own bytes**, on plants built out
of objects the other routes had just harvested. `id-signedData` opens on `0x2A`, which is `*`, and
the candidate pattern had been assembled by concatenating those octets into a byte regular
expression — so it said *zero or more of the preceding length octet* and matched nothing anywhere.
A harvest would have run for an hour and reported a smaller number with no error and no way to tell
it from a corpus that held none. `re.escape` is the fix, and the comment beside it says why.

Two hundred objects sampled at random (seed 825) from the 1854, planted into synthetic one-object
documents:

| plant | the route says |
|---|---|
| an object placed in a `/Contents` hexadecimal string beside a `/ByteRange` | found **200 of 200**, bytes identical 200 of 200 |
| the same with 2048 octets of the producer's reserved space after it | identical **200 of 200** — the trim is exact |
| the same with one non-hexadecimal octet in the string | **0 of 200** |
| an object as an uncompressed `stream` body under a `/TS` | found **198 of 200** — the 2 are `/Contents` blobs that are not CMS at all |
| the same inside a `FlateDecode` stream | raw scan **0**, inflated scan **198** |
| the same in a file naming none of `/ByteRange`, `/DSS`, `/VRI`, `/TS` | **0 of 200** — the gate holds |
| `id-signedData` retagged `id-data` | **16 of 200**, and those 16 are *exactly* the 16 carrying a nested token whose own identifier the single replacement left alone |
| the same objects three octets short | **0 of 200** recovered whole |
| the 148 certificates `seed_x509.py` harvests from the same files | **0** — the two seeders' populations do not overlap |
| planted as an *unsigned* attribute of a rebuilt `SignerInfo` | recovered **198 of 198**, bytes identical 179 |
| ... through the *signed* attribute slot instead | **198 of 198** |
| ... under an attribute identifier nothing enumerates | **198 of 198** |
| an attribute value that is not a `ContentInfo` | **0** |
| a token inside a token | **2 of 2** |

**Routes one and three agree byte for byte on 198 of 200** where one object is stated both ways in
one file, which is what lets the SHA-1 filename deduplicate across them. The 19 that route two
recovers without being byte-identical are exactly the 19 that state an indefinite length outermost,
which is the cost written into `nested`'s doc comment: a token lifted out of the middle of a buffer
has its header rebuilt definite. Route one, which keeps the file's bytes, is where the corpus's
indefinite-length BER comes from.

**And the refactor was calibrated before anything used it.** `seed_x509.py` at `3c259925` and after
the move of its X.690 walk into `seed_der.py`, over the same 4974 documents: **148 certificates on
both sides**, the same 85/54/9 route tally, the same file names, byte-identical contents.

## 6. Crashers

**None.** `fuzz/artifacts/cms` was empty before this round and is empty after it. Principle 3's
"every crasher found becomes a permanent regression test" had nothing to bind on, and that is
reported as a result rather than an absence because the `INITED → DONE` pairs above are what make it
a claim about the code rather than about an exit status (ADR 0747).

## 7. Gates

`tools/round.sh` says this is not a fifth round. The change is two Python scripts, two documents, a
doc comment and two ledger notes — no Rust code, nothing a gate rasterises with. So §2's core, both
`fuzz/` lines, and the line the change→gate map adds for a documents-only change plus
`doc/conformance/ledger.toml` and "a doc comment citing a clause", which is `cargo test -p conformance`.

**One judgement is recorded rather than assumed**: `crates/pdf-model/src/cms.rs` is in the map's
first row, which is "everything". What changed in it is one doc comment above a `#[cfg(test)]`
module, and the map's own last row names "a doc comment citing a clause" and sends it to the
conformance gate. A doc comment cannot move a pixel, which is the property trap 1's rule is about, so
the narrower row is the one this round followed and this paragraph is here so that a merge can
disagree with it.

| line | result |
|---|---|
| `cargo fmt --all --check` | silent, exit 0 |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | exit 0, and the only `warning:` lines are the documented `proc-macro-error2` future-compat note and the cold-build `cxx-qt` gcc block |
| `cargo nextest run --workspace` | 2795 run, 2795 passed, 18 skipped |
| `cargo test --workspace --doc` | exit 0 |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent, exit 0 |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | silent, exit 0 |
| `cargo test -p conformance -- --nocapture` | 209 passed, 0 failed |

**The conformance gate failed once and it was this round's fault**, which is worth a line because it
is the shape a documents-only round can break `main` in: the §12.8.3.4.2 correction put a pair of
straight double quotes inside a TOML basic string, and both ledger tests panicked with
`TomlError { line: 4424 }` before reading a row. Escaped, re-run, green. The gate did exactly its
job; what it says about the round is that a note is code.

## 8. The §4 sweeps, before and after

Twenty-three sweeps over a pristine `git worktree` of `3c259925` with its own build directory
(`r8250`, closed with it), and again over this branch. **Every exit status is identical** — `quoted`,
`retired` and `unpriced` exit 1 on both sides as they have since ADR 0742's run, those three taking
an argument this run did not give them. **Twelve outputs are byte-identical and eleven differ, and
one of the eleven was a finding until it was fixed:**

- **`parts`** printed a hit this round's own prose created: the §12.8.5 note said the witnesses are
  "in the crawl rather than in either submodule", and `either` presupposes two where the workspace
  states six. Rewritten to name the corpora rather than count them, and `parts` is now byte-identical
  to the baseline — 583 cardinals, 39 the workspace agrees with, 48 on the closest rung, both sides.
  That is the twenty-second sweep catching exactly what ADR 0709 built it for, in prose written by a
  round that had read the rule.
- **`quotations`** reads 6951 in 1125 documents against 6935 in 1123. **2875 verbatim against 2874**
  — the one addition is §12.8.4.4's Table 262 `/TS` sentence — and **38 diverging on both sides**, so
  nothing this round wrote claims to be a quotation and is not one.
- **`tables`** reads 2652 attributed key citations against 2651, and the new one **agrees with the
  table it names** (2484 against 2483). `101 absent` and the six contradicted denials are identical.
- **`counts`** reads 9138 governing sentences against 9132 and 460 attributed counts against 459,
  the new one among the 151 **the family agrees with**. The three other sub-counts are identical.
- **`pointers`** reads 9384 path pointers against 9365, the nineteen being paths this round's files
  cite. **`102 absent` on both sides**, which is the finding-shaped number, and `162 symbol
  pointer(s), 13 undefined` is identical.
- **`owed`** moves §12.8.3.4.2 from 13 stated terms to 14, **every one still named by a source**, and
  its totals are unchanged where they matter: 183 terms named by no source over 113 rows, both sides.
- **`inapplicable`** and **`owed`** both read `todo` as named by 193 files against 192 — the ledger
  note that now cites `doc/todo/51`.
- **`overtaken`** reads 640 decision records against 639 — ADR 0754. The same 137 page-list notes over
  340 documents, the same 45 overtaken and the same three sub-counts.
- **`capabilities`** differs only in two line numbers inside `cms.rs`, which the new doc comment moved.
- **`ledger`** differs only in the absolute path it prints. 875 rows, 0 new, both sides.
- **`errata-applied`** reads 61 675 places against 61 644, and **1033 name an erratum this collection
  carries on both sides**, with the `#NNN` token count identical at 203.

**One caveat that is structural rather than this round's**: several of these sweeps read this file,
so a sweep whose population includes it cannot be reported in it without a one-step lag. The figures
above are the run made on the tree as it stood before this section was written, which is the same
statement ADR 0751's round had to make.

## 9. What this round did not take

- **`0.4.0.127.0.7.1.1.4.1.3` is not a debt**, and a round reading the census's
  `2 AlgorithmNotVerifiable` should read `doc/todo/51` before reaching for it: ISO/TS 32002 section
  5.1.3's NOTE 2 puts BSI TR-03111's plain `r ‖ s` outside what the Technical Specification admits,
  so answering by the identifier is the correct behaviour. The census confirms the count that todo
  states and moves nothing.
- **No fixture is retired and no ledger status moves.** The corrections are to denominators.
- **`cms.rs` gained no code**, so the shapes its `fixtures` module builds still reach no corpus. What
  would change that is the one thing this round deliberately did not do: a route that runs Rust.
