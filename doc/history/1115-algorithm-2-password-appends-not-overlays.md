# 1115 — §7.6.4.3.2 Algorithm 2's password: appends, and the row says so

Contract: §7.6's encryption `partial` rows, principally §7.6.4.3.2 (Algorithm 2, the
password padding, revision 4 and earlier); §7.6.4.1 and §7.6.6 if they shared its
residue. They did not.

## The three residue sentences, resolved against the current code

The row said `crypt.rs` (a) still shares one conversion with `text_string.rs`, (b) still
overlays the password on the padding string rather than appending, (c) still refuses only
the characters the encoding gives no code.

- **(a)** true, and it *is* step (a)'s PDFDocEncoding conversion — the whole of Annex D
  Table D.3 — not a debt. Test `a_password_is_converted_by_the_table_rather_than_by_the_ranges_that_agree`.
- **(b) false of the current code.** `pad_password` is `bytes.take(32).chain(PAD)`, so an
  n-byte password's tail is `PAD[0..32 − n]` — the first 32 − n bytes — an append. Unit
  test `a_short_password_is_extended_from_the_start_of_the_padding_string` asserts the
  tail against `PAD`; the corpus proves it where an overlay fails —
  `a_document_with_a_password_opens_with_it_and_not_without` decrypts eight non-empty
  passwords (`issue6010_2.pdf`'s is `æøå`). The claim was written 2026-08-14, two weeks
  after `crypt.rs` was created (2026-07-30) already appending: a blame-list reading
  transcribed without checking the function.
- **(c)** true, and it is the *encoding's* limit — U+00A0 has no code (0xA0 is the euro
  sign), no bytes to hash, and §7.6.4.1's revision 6 preprocessing is the standard's own
  answer for a password outside PDFDocEncoding. A documented decision.

Algorithm 2 (R2/3/4) and 2.A (R5/6 SASLprep) are cleanly separated —
`authenticate_legacy`/`pad_password` vs `authenticate_aes256` — not conflated.

## Ledger and census

§7.6.4.3.2 `partial` → `implemented`, two witness tests added; §7.6.4 aggregate drops it
from what it lists as owed. No code changed — the residue was already discharged.
§7.6.4.1 and §7.6.6 keep their own unrelated residues.

Census over doc/pdf.js: 25 state /Encrypt, 14 open on the default password; by /R among
those, R2:1 R3:1 R4:7 R6:5 (/R-0 bucket 11 = 10 locked + 1 refused).

Out of contract, not fixed: §7.6.4's note still lists §7.6.4.4.2 steps (e)–(h) as owed
though that row is `implemented`.
