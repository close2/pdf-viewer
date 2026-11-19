# Q953 — What does a revision 6 file whose `/Perms` does not verify open as?

Source: round 2131, the specification audit of §7.6 (`doc/reviews/2131-specification-audit-7-6.md`,
finding 2131-T01).

## The clause

ISO 32000-2 §7.6.4.3.3, Algorithm 2.A, the last step of *retrieving the file encryption key*:

> f) Decrypt the 16-byte Perms string using AES-256 in ECB mode with an initialization vector of
> zero and the file encryption key as the key. Verify that bytes 9-11 of the result are the
> characters "a", "d", "b". Bytes 0-3 of the decrypted Perms entry, treated as a little-endian
> integer, are the user permissions. They shall match the value in the P key.

Table 21 makes `/Perms` "Required if R is 6". §7.6.4.4.12's Algorithm 13 repeats the check with
"should match". Neither says what a reader does when the verification fails.

## What this tree does

`crypt::perms_block` returns nothing when the block is absent, shorter than 16 bytes, or decrypts
without the `adb` marker, and `Encryption::new` then opens the document under the plaintext `/P`
and `/EncryptMetadata` with nothing reported. Where the marker is there and `/P` disagrees, the
block wins (ADR 2962, a recovery policy).

At revision 6 the file encryption key unwrapped from `/UE` or `/OE` is checked by nothing else:
`/U` and `/O` authenticate the *password*, and `/Perms` is the only value encrypted under the key
itself. So a block that fails the check means either the key is wrong (a damaged `/UE` or `/OE`)
or the block is (a damaged or removed `/Perms`), and the plaintext `/P` it falls back to is bound
to no key.

## The options

1. **Refuse the file by name** — "§7.6.4.3.3 step (f): `/Perms` does not verify under the
   retrieved key". Strictest; loses a readable document whose only damage is `/Perms`.
2. **Open it, read `/P` as written, and say so** — a document note ("the permissions are not
   bound to the key") beside today's behaviour. Nothing is lost; the reader sees the gap.
3. **Open it with no grants from `/P`** — the restrictions a file asserts are the reader's to
   switch off anyway (CLAUDE.md principle 3), so this costs a reader at `on` some operations.
4. **Keep today's behaviour** and record it as policy in an ADR, the way ADR 2962 records the
   disagreeing-block case.

The audit's suggestion is 2: the step says "verify", and a verification whose failure changes
nothing a reader can see is not one; a refusal (1) would trade a readable document for a check
on permissions, which are low priority by the owner's own ranking.
