# 1020 — The signature stack leaves `pdf-model`, and two call sites keep its packages there

Session 1000. Status: **accepted**. Takes the first half of ADR 1005 §1, the extraction
`doc/reviews/984-direction-and-boundaries.md` finding 1 ranked first and priced.

Context: `crates/pdf-signature/` (new), `crates/pdf-model/src/lib.rs`, `Cargo.toml` (workspace and
both crates'), `doc/conformance/ledger.toml`, `doc/crate-map.md`, `doc/stack.md`,
`doc/todo/02-every-round.md` §2 rule 2, `fuzz/`. ADRs 0215, 0229, 0314, 0322, 0331, 0390, 0532,
1005. `doc/PLAN.md:329-337`, which records the argument for a crate boundary and had applied it to
CMaps and not to this.

## 1. What moved

Ten modules — `signature.rs`, `cms.rs`, `x509.rs`, `der.rs`, `bigint.rs`, `pkcs1.rs`, `pss.rs`,
`dsa.rs`, `ecdsa.rs`, `eddsa.rs` — by `git mv`, so `git log --follow` still reaches every decision
above. With them went the file that tests them (`tests/signatures.rs`) and the census that
measures them (`examples/signature_algorithm_census.rs`), because a crate whose tests live in
another crate is not independently testable and that was half the argument for moving it.

The new crate depends on `pdf-syntax` and on **no other crate of this tree**. That is the rule
worth stating in one line, because it is what makes the boundary checkable rather than merely
drawn: *a signature is read out of the object graph, never out of a page.* §12.8.1's `/ByteRange`
is a range of the file's bytes and Table 255 is a dictionary; neither needs a page tree, and
`cargo metadata` now says so.

Nothing is re-exported from `pdf-model`. A re-export would have cost the four external consumers
nothing and bought the tree a second name for every module, which is exactly the accumulation the
review was commissioned to find; the consumers changed an import instead —
`viewer-core/src/notes.rs`, `viewer-core/tests/headless.rs`, `pdf-transform/src/lib.rs`,
`pdf-archive/src/table/interaction.rs`, and a fifth the review's count missed,
`pdf-transform/src/archive/signatures.rs`.

## 2. What it bought, measured, and it is not what the review expected

The review priced this extraction on one number: "`tools/spec-errata`, which reads errata out of
PDFs, compiles `p521`". **It still does.** `cargo metadata --filter-platform x86_64-unknown-linux-gnu`,
non-dev closure, before and after:

| | before | after | lost | gained |
|---|---|---|---|---|
| `pdf-model` | 110 | 111 | none | `pdf-signature` |
| `spec-errata` | 112 | 113 | none | `pdf-signature` |

The reason is two call sites, and they are not an oversight anybody left behind — they are the
clause:

- `crates/pdf-model/src/restriction.rs:394,407,413` asks `signature::permissions(document).doc_mdp`,
  `signature::field_locks` and `signature::field_mdp`, because §12.8.2.2's `/DocMDP` level and
  §12.8.2.4's field locks are *restrictions on the reader* and `Restriction` is where this tree
  collects those, beside Table 22's `/P` flags.
- `crates/pdf-model/src/view.rs:3026` asks `signature::permissions(document).usage_rights` for
  §12.8.6's `/UR3`, for the same reason.

A third, `icc.rs`, was an accident and is gone: it reached into `cms::Digest::Md5` for a hash, and
ICC.1 section 7.2.18's profile identifier is a content hash with no signature anywhere near it. It
now calls `md-5`, which `pdf-model` already depends on for Table 45's `/CheckSum`.

**What a second step would buy, measured rather than guessed.** Take the closure of everything
`pdf-model` depends on *except* `pdf-signature`, and subtract: **33 packages** leave the page
tree's graph — `autocfg base16ct crypto-bigint curve25519-dalek curve25519-dalek-derive der ecdsa
ed25519 ed25519-dalek elliptic-curve ff group hmac keccak libm num-traits p256 p384 p521
primefield primeorder rfc6979 ripemd rustc_version sec1 semver serdect sha1 sha3 shake signature
sponge-cursor wnaf`. Three of the thirteen cryptographic packages the review named do **not**
leave, and the measurement is the only way to know which: `const-oid`, `sha2` and `md-5` are all
reachable from `pdf-syntax`, which needs them for §7.6's standard security handler.

**What blocks the second step is one type.** `signature::Permissions` carries
`doc_mdp_signature: Option<Signature>` and `usage_rights_signature: Option<Signature>` — the
dictionaries `/DocMDP` and `/UR3` point at — so the §12.8.2.2 reading `restriction.rs` wants
cannot be lifted away from the verification stack without deciding what `Permissions` is. That is
a decision about §12.8's own shape, not a move, and it is not taken here. It is the third
extraction, after `pdf-colour`.

## 3. What it did buy

- **One responsibility per crate, for two crates rather than one.** `pdf-model/src` is 84,046
  lines in 66 files where it was 93,413 in 76; `pdf-signature/src` is 9,968 in 11. A student asked
  "where is the document model" no longer opens a directory and finds `pss.rs`.
- **Independently fuzzable, and now literally so.** `fuzz/`'s `cms` and `x509` targets name
  `pdf-signature` and not `pdf-model`; the fuzz manifest gained the crate and the two targets
  their imports.
- **A boundary a command can check.** `cargo metadata` will fail the day something in
  `pdf-signature` reaches for a page; before today the same reach was a `use crate::`.
- **The dependency table reads.** `doc/stack.md`'s twelve cryptographic packages now have one
  crate to be *in*, so a round asking "who compiles `p521`" gets a one-line answer —
  `cargo tree -p spec-errata -i p521` prints `p521 -> pdf-signature -> pdf-model -> spec-errata`,
  which is section 2's finding in four words.
- **A round editing §12.8 recompiles 9,968 lines rather than 93,413.** Two extracted trees at
  `af11dd09`, differing only by this change, each with its own target directory, `pdf-model`'s
  `lib.rs` given a comment and rebuilt three times: **8.36 / 8.48 / 8.88 s** before,
  **7.38 / 7.61 / 8.62 s** after — about a second, or 11%, on a machine carrying five concurrent
  rounds, so the spread is the machine's and the direction is the 10.6% of source that left.
  Rebuilding `pdf-signature` alone is **1.30–1.43 s**, and nothing downstream of `pdf-model`
  rebuilds with it unless `restriction.rs` or `view.rs` is what changed.

## 4. The ledger, which is why this was safe

`tools/conformance` verifies that every `code` and `test` site a ledger row names exists on disk,
and a `test` site naming a function verifies the function is in that file. **72 rows name a path
under the moved `src/` modules** — the number the review predicted — and two more name only
`tests/signatures.rs`, for 74 lines rewritten by one `sed`. Twenty-two prose references to
`pdf_model::cms` and its siblings inside row notes were rewritten with them, because a note that
names a module path is making a claim about where code lives.
`the_ledger_agrees_with_the_standard_and_with_the_tree` is what holds all of it, and it passes.

## 5. `pdf-signature` is under everything, and that is a cost

`doc/todo/02` §2 rule 2 listed seven crates under every gate; it lists eight. `pdf-model` depends
on the new crate, so a change to `cms.rs` still costs the whole sequence — the second thing the
review hoped this would fix and the second thing section 2's two call sites deny. The rule's own
text now says what would take the list back to seven, so the day the third extraction lands
nobody has to rediscover it.
