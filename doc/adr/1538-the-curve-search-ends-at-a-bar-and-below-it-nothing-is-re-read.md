# 1538 — The curve search ends at a bar, and below it nothing is re-read

Status: accepted. Session 1351. Applies the swap condition of the owner's answer A170 as ADRs 1385
and 1386 record it; does not answer `doc/questions/Q192`.
Context: `doc/todo/65`'s curve paragraph, `doc/stack.md`'s curve paragraph, RFC 8032 section 5.2.7.
Code: none. The re-check is the command block in `doc/todo/65`.

## 1. What went wrong

Three rounds judged fourteen packages that `cargo search ed448`, `brainpool` and `bp512` list —
licence, owner, repository, published source, RFC 8032 coverage — and each search at a larger
`--limit` lists more: the last round left eight names unjudged. None was ever a candidate, and the
reason was the same every time: not RustCrypto's, not reviewed, or reviewed for something else.
A search ranks names by text, and its tail is unbounded; reading it is work without an end.

## 2. The rule

A170's condition is *a stable, reviewed crate covers the curve*. Two things meet it, and the
re-check asks only for them:

1. **A release on RustCrypto's own line**: `ed448-goldilocks` leaving `0.14.0-pre` for a stable
   version, or a `bp512` published from `RustCrypto/elliptic-curves`. RustCrypto's line is the one
   `p256`, `p384`, `p521`, `bp256` and `bp384` came from, reviewed as a project, on the tree's own
   `digest` generation — so a stable release there is the swap `doc/stack.md` already describes.
2. **A package with an audit on record** whose subject covers the curve's arithmetic and, for
   Ed448, RFC 8032 section 5.2.7's cofactored verification. `frost-ed448` is the measure of why
   both halves are needed: its audit is real and its verification rejects the torsion the
   cofactored equation admits.

A package below that bar is not read again, however a search ranks it. The re-check prints the two
RustCrypto versions and any package whose crates.io description names an audit without denying
one — nothing else — and a round reads further only when it prints something new.

## 3. Why it is a round's command and not a `tools/state.sh` section

A section counts the tree and must answer offline and at once; `tests/read_only.rs` reads every
section for writes, and a section that waits on a registry would make `quick` as slow as the
network and fail where there is none. So no section calls the network, `tools/state.sh` gains
nothing, and the re-check stays the command block in `doc/todo/65` beside its last date.

## 4. What it does not decide

`ed448-goldilocks-plus` (Q192) is the owner's: its README says it is unreviewed, so under this rule
it is below the bar, and the rule does not pre-empt the owner saying it counts. The fourteen
judgements stay in `doc/todo/65` as one dated list, so a later round meeting a name can see it was
read and why it failed.
