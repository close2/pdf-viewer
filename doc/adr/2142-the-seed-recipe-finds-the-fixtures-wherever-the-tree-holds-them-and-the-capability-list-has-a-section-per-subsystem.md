# 2142 — The seed recipe finds the fixtures wherever the tree holds them, and the capability list has a section per subsystem

Session 1653, batch one hundred and one's slot 7, a docs round. Status: **accepted** and
**applied**. Context: ADRs 2110 and 2126 (the two compactions, whose readers found the claims
below outside their slots), 1023 (a correction is applied, not appended), 0229 and 0751 (the
`x509` recipe and its routes), 1439 (a fuzz target arrives seeded). Cites no clause.

## 1. The `x509` arm reads every Rust source under `crates/`

`fuzz/seed_x509.py`'s third route reads the certificates the signature crate's `fixtures` modules
state in hexadecimal — the only inputs that reach `dsa::verify` and the curve arms of `ecdsa` and
`eddsa` (`doc/verify.md`, under `x509`). `fuzz/seeds.sh`'s arm handed the seeder
`crates/pdf-model/src/*.rs`, and the certificates are in `crates/pdf-signature/src/`, so a re-seed
reached **none** of them: twelve certificates (`x509.rs`'s two, RSASSA-PSS, DSA, P-256, P-384,
P-521, brainpoolP256r1, P384r1, P512r1, Ed25519, Ed448), and the summary line printed `0 out of a
fixture` with nothing failing. The brief's hypothesis said five routes; it was twelve.

**The decision is that the arm names no crate.** It feeds the seeder every `.rs` file under
`crates/` beside the documents, on the same NUL-separated list, and the seeder's own `FIXTURE`
pattern and `is_certificate` decide what is a certificate. Naming the crate again would be the
same hand-written population trap 25 is about, waiting for the next move; reading the whole tree
costs 0.37 s. `tools/conformance/tests/fuzz_seeds.rs` runs the arm from a byte copy of `fuzz/` in
a temporary tree whose `crates` links to this one, so no document is walked, and holds the
summary's three routes to what the tree states: the fixture route equal to the distinct
`…CERTIFICATE…: &str` constants the test counts itself, the other two zero, and one seed per
fixture. With the old glob planted back it fails naming the five files and the twelve
certificates (trap 13).

## 2. `doc/state-of-play.md` has one section per subsystem

The file was some 1 170 lines under one heading, so a round asking one question read all of it.
It now has twenty-six `###` sections under `## The state of play`, from *What a page draws* to
*Accessibility*, inserted at paragraph or sentence boundaries by a script that asserted, at each
write, that the file's words less the heading's were the words it read; over the whole pass the
multiset of words equals the old file's plus the headings'. **No word changed**, and the long
sentences stay whole: *What a person does with an open document* is one sentence of a hundred
and fifty lines from the password to the save, and a heading inside it would have split it.
`doc/HANDOVER.md`'s row says to read the section the question is about. `tools/round.sh` named
no section of this file, so it had no pointer to re-point. A capability added later goes under
the heading of its subsystem; a heading is a lookup key and states no capability.

## 3. The claims found false, each corrected where it stands

- `tools/drive-windows.sh`: the toolkit windows "draw no page through a graphics device" — they
  do (ADR 2070); the step reads `quorra`'s words only, and its "not offered" now says so.
- `doc/todo/15`: a quotation of `doc/ui-boundary.md` the file no longer holds — now
  `doc/PLAN.md` section 6's sentence, paraphrased.
- `pdffs` in `pdf-fuse`'s description, usage, messages, mount name and thread name, and in
  `doc/crate-map.md`: the binary is `quorrafs`; `lib.rs` keeps the RFC's name as the RFC's.
- `NOTICE`: `pdf-viewer --licences` and three `pdf-viewer` windows — it is `quorra --licences`,
  and the three that show it are `quorra`, `quorra-gtk` and `quorra-qt`.
- `Cargo.toml`: SHAKE256's identifier "has no second reading" — `const-oid`'s database is it;
  `cms.rs`'s "all ten" is the nine that database holds.
- `doc/todo/README.md`'s row 57: "all five writing verbs" and "three things" — eight writers write a
  file, and what is owed is the todo's sections 1, 3 and 4.
- `RestrictKind::On` "The default" — `Off` is (ADR 1144).
- `pages_corpus.rs`: the structure tree "checked to be absent" — it is carried and judged against
  §14.7's clauses; `foreign_corpus.rs`: "five writers exist" — five of the eight are read back.
- `spelled_ordinals.rs`: "the three are todo 56's" under a ceiling of zero; `bridge.rs`: a
  sentence about what the line used to say, and the list missing the three workers; `doc/PLAN.md`:
  the same list of three; `quorra.h`: two session ordinals.
- `unsafe_position.rs`'s `only_the_three_named_crates…` asserts six — renamed
  `only_the_named_crates_in_the_tree_lift_the_denial`, its three citers re-pointed.
- `gesture.rs`: "a measurement rather than a gate" over a module of five gating tests, and a
  `-p viewer-ui --bin quorra` command for a module now in `viewer-device`'s library.
- `press_cost.rs`: the press table "a process-wide `static`" whose scope a round might ask about —
  a cache of eight beside the interpretation's budget (ADR 0417).
- `awkward_classes.rs`: an ignore reason sending the reader to `doc/verify.md`, which sends them
  to `doc/todo/02` section 2; a "`--bins` line" that section does not have; an unlocked wrapper.
