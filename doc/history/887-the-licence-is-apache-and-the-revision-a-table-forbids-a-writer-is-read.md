# 887 — The licence is Apache, and the revision a table forbids a *writer* is read: `LICENSE` and every place the tree names it move to Apache-2.0, and `/R 5` — the deprecated Adobe extension iText keeps writing — authenticates, unwraps its key, checks its `/Perms` block and opens 33 of the 41 documents this machine holds

Date: 2026-09-03.
ADRs: [0819](../adr/0819-the-licence-is-apache-2-0.md),
[0820](../adr/0820-the-revision-a-table-forbids-a-writer-and-a-reader-still-meets.md).
Touched: `LICENSE`, `NOTICE`, `Cargo.toml`, `deny.toml`, `.github/workflows/ci.yml`,
`crates/pdf-syntax/src/crypt.rs`, `crates/pdf-syntax/tests/encryption.rs`,
`crates/pdf-model/tests/corpus.rs`, `crates/pdf-model/tests/oracle.rs`,
`doc/conformance/ledger.toml`, `doc/state-of-play.md`, `doc/third-party-data.md`,
`doc/HAYRO_MERGE.md`, `doc/traps/instruments-and-reports.md`,
`doc/todo/03-more-corpora.md`, `doc/todo/38-a-documents-restrictions-have-levels.md`.
Both items are the project owner's, asked for on the day.

## The licence

> please switch the license of pdf-viewer and quorra to apache.

`quorra`'s half was taken first in its own tree (`6043deb`, awaiting the owner's push); this is
`pdf-viewer`'s. `LICENSE` is the verbatim Apache License 2.0 — copied from
`~/.cargo/registry/src/…/thiserror-2.0.19/LICENSE-APACHE`, `diff`-identical to the one `quorra`
took, never retyped — under the copyright notice and boilerplate naming Christian Loitsch, 2026,
and keeping the closing paragraph about `NOTICE`. `Cargo.toml`'s workspace `license` is
`"Apache-2.0"` and all twenty-six members still say `license.workspace = true`, so the identifier
is in one place.

`git grep '\bMIT\b'` was read hit by hit rather than replaced by machine. Six more files named the
old licence: `NOTICE`'s opening sentence, `deny.toml`'s allow-list rationale and its BSL-1.0
comparison, CI's packaging comment, `doc/third-party-data.md` in three places — including a
dependency remark reading "every one of them is MIT, **which is this project's own licence**" —
and `doc/HAYRO_MERGE.md`, which priced both merge directions against our MIT. `doc/adr/` and
`doc/history/` were deliberately left alone: ADR 0232 §2.

Nothing in the dependency graph objected and nothing had to be added — `Apache-2.0` has been
`deny.toml`'s *first* allowed licence since the beginning, because dependencies were under it long
before we were. `cargo deny check`: **advisories ok, bans ok, licenses ok, sources ok**. No GPL or
MPL package is in the graph; the one GPL item this tree's records name is poppler's `cidToUnicode`
data, examined and not taken. One obligation is new and a file that already existed meets it:
Apache-2.0 §4(d) asks a redistribution to carry the work's `NOTICE`, which `--licences` prints,
`?` shows and CI packages, so `NOTICE` now says so in a paragraph of its own.

## The revision

> the deprecated encryption algorithms are still used quite a lot because the itext library
> generates them. please implement them in one of the following rounds.

`crypt.rs` refused `/R 5` by name and had since the twenty-second session, on Table 21's "Shall
not be used. This value was used by a deprecated proprietary Adobe extension" read as *the
standard states no algorithm*. **The refusal outlived its reason and three things say so.** That
sentence binds a *writer* choosing a value to store, and `CLAUDE.md`'s statement of done is that
every PDF that *exists* renders as its producer specified. §7.6.4.1 states a requirement about the
revision rather than a silence — "[i]f a security handler of revision 4 or 5 is specified, the
standard security handler shall support crypt filters" — and deprecates it in the same breath as
revisions 2, 3 and 4, all of which this module has always read. And Table 21 *points* at the
algorithm: the Adobe Supplement to ISO 32000-1, `BaseVersion` 1.7, `ExtensionLevel` 3, which eight
of these files declare in their own catalogues.

`Aes256Hash` is the whole of the difference — Algorithm 2.A runs once for both revisions, with
§7.6.4.3.4's iterated hash for 6 and one SHA-256 for 5, and `hash_2b`'s own first line is now the
function revision 5 calls. The data path is not the extension's at all: Table 20's `/V` 5 sends it
to §7.6.3.3, which was already there. The crypt-filter pairing gains revision 5 from Table 20
rather than from §7.6.4.1, whose sentence names 4 and 6 and skips 5. `/Perms` is read at revision 5
as at 6. And Algorithm 2.A steps (d) and (e) now check their own arithmetic for **both**: an `/OE`
or `/UE` that unwraps to anything but 32 bytes is a wrong password at the door instead of a stream
quietly declining several layers down.

**What could not be obtained is in ADR 0820 rather than glossed.** The supplement is not in
`doc/md/` and there is no network here; PDFBox — the owner's suggested source — is not on this
machine at all. What is here is pdf.js's `crypto.js`, whose `PDF17` is one line, and that is
evidence about a reading in principle 5's one direction, cited nowhere in the code. The
confidence comes from a real document instead: `issue21579.pdf` decrypts to
`(repro1a: AES-256 R5, password 'passwoert' with umlauts)`, its `/Perms` block gives back exactly
the `/P -1084` and `/EncryptMetadata true` beside it, and a 256-bit hash equality does not hold by
accident. ADR 0820 has a row per step saying which rest on a normative sentence and which on
evidence — **one step rests on a reading**: SASLprep before UTF-8, which §7.6.4.3.3's preamble
binds to the algorithm revision 5 shares and pdf.js applies only at revision 6, invisible on every
empty or ASCII password and on the one witness there is.

## The census, and the fixture

41 documents state `/Filter /Standard` with `/R 5` among the 90 535 in `doc/pdf.js`,
`doc/corpora/` and `corpus-cache/` — 20 in the crawl, 20 in the tracker, 1 in `doc/pdf.js`, 1.7% of
the encrypted files. Every one is `/V 5`, `AESV3`, `/UE` and `/OE` 32 bytes, `/Perms` 16. All 41
were refused before, the refusal preceding authentication so none could reach it; **33 open now**
— 32 on the empty password, 1 on the one `encryption.rs` records — 8 are locked wanting a password
nobody here has, and 0 are refused. 19 of the 33 withhold at least one of the two operations this
program has, through the encrypted block rather than the plaintext `/P`. Two ratchets move:
`MAX_UNREADABLE_ENCRYPTION` 2 → 1 and `MAX_LOCKED` 8 → 9, which is one document going from
*refused* to *one sentence away*.

**8 of the 41 write `/U` and `/O` as 127 bytes** — 48 significant followed by NUL — and they are
exactly the 8 declaring `ExtensionLevel` 3. Algorithm 2.A reads three fixed sections off the front
and salts the owner branch with "the 48-byte U string", so this reader was already right; there is
a test for it now, because a reader that demanded 48 would refuse a fifth of the population.

`tests/encryption.rs` argues against fixtures for this clause, and the fixture the owner asked for
does not break the argument: it is assembled around `issue21579.pdf`'s own `/U`, `/UE`, `/Perms`
and page-one ciphertext, so what is fabricated is the catalogue, the page tree and the
cross-reference table, which no cipher touches. Six tests, including the owner branch — whose `/O`
and `/OE` were computed *outside this tree* and wrap the **real** file key, so that branch is shown
to reach the key the real ciphertext was encrypted under — and a `/Perms` block overruling a
plaintext `/P` of −1. Two refusals keep tests: `/R 7`, and a `/R 5` naming `AESV2`.

Trap 27's illustration is now the tree's behaviour and the trap says so without losing its
incident.

## The gates

The full `doc/todo/02` §2 sequence, run in the worktree, all green: `fmt` (clean after two
rustfmt diffs and six `doc_markdown` lints in the new prose), `clippy --workspace --all-targets`
under `RUSTFLAGS="-D warnings"`, **3027 tests** in `nextest`, the doctests, both `fuzz/` lines, the
sandbox worker, `corpus` (9 locked, 1 encryption we do not implement), `pdfref-hayro`, `oracle`
(61 contradicted pages, every one held by a group), `text_extraction`, `selection_census`,
`accessibility_census`, `dates`, `xmp`, `jpeg2000`, `render-quorra --test corpus`,
`fixed_documents` (59 rows, 0 absent), the transform gate, `writer_corpus` (940 attached and read
back), and `cargo test -p conformance` (218). Beyond the sequence: `cargo deny check` for the
licence, and the `crypt` fuzz target at 300 000 runs plus `document` at 29 128, no crash. §5's
binaries are not owed — not a fifth round, and nothing was measured.
