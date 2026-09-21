# 1161 — Encryption on the way out is asked for, never inherited

Status: accepted. Session 1162.
Context: `crates/pdf-syntax/src/crypt.rs` (`revision6`, `Unpredictable`, `Encryption::for_output`,
`Encryption::encrypt_with_iv`, `perms_entry`, `wrap_key`, `aes_cbc_encrypt_raw`),
`crates/pdf-syntax/src/serialize.rs` (`serialize_encrypted`, `Protection`, `Access`, `Entropy`,
`SystemEntropy`, `Protected`, `Written::cleartext`), `crates/pdf-syntax/src/lib.rs`,
`crates/pdf-syntax/src/document.rs` (`is_signature_dictionary` made crate-visible),
`crates/pdf-syntax/tests/serialize_encrypted.rs`.
Builds: RFC 0002 section 10's serializer, ADRs 0816, 0817, 0818, 0129.
Clauses: ISO 32000-2 §7.6.2, §7.6.3.1, §7.6.3.3, §7.6.4.1, §7.6.4.2 (Tables 20, 21, 22, 25),
§7.6.4.3.3, §7.6.4.3.4, §7.6.4.4.7, §7.6.4.4.8, §7.6.4.4.9, §7.6.6, §7.5.7, §14.4.

## 1. The condition `CLAUDE.md` set has been met

`CLAUDE.md` scopes writer-side obligations in "where the serializer actually emits the construct"
and names "§7.6 encryption on the way out" among them. ADR 0817 and the ledger's §7.6.2 row both
stated the gap as a *condition* — the rows move the day a writer emits an `/Encrypt`. This is that
writer. `serialize_encrypted` writes the standard security handler at `/V` 5 and `/R` 6 with
Table 25's `AESV3`, which is the one configuration the standard leaves undeprecated: Table 20 says
"Values less than 5 for the V entry are deprecated in PDF 2.0" and §7.6.4.1 says "Use of security
handler revisions 1, 2, 3, 4 and 5 is deprecated in PDF 2.0". A reader meets whatever revision a
file states, which is why `crypt.rs` reads five of them; a writer chooses, so it chooses this one.
`/R` 5 is not written and is not read either — the owner wants the read side eventually, on the
Adobe supplement, and that is a separate argument.

## 2. The protection of a derived file is the caller's statement, not the source's

**A derivative cannot inherit its source's encryption, and revision 6 is the reason rather than a
convenience.** §7.6.4.4.7 stores `/U` as an Algorithm 2.B hash of the user password with a salt;
§7.6.4.4.8 stores `/O` the same way. A program that opened a document with the *owner* password
holds the file encryption key and the owner password and nothing else: it cannot produce the user
password, and it cannot produce a `/U` for a password it does not have. Opening with the *user*
password is no better in the other direction. The one revision where an owner password does unwrap
a user password is §7.6.4.4.6's Algorithm 7 — and that belongs to the revisions §7.6.4.1
deprecates, so writing one of those to make inheritance possible would be choosing a deprecated
construct in order to avoid asking a question.

So `Protection` is an argument. Every object the serializer is handed is plaintext anyway, because
a `Document` decrypts on load; what a derived file asserts over its next reader is a new decision,
and `CLAUDE.md` principle 3 says where such a decision belongs — asked once, in a place a host can
supply. `Access::ALL` is the default a caller should want: a restriction is the reader's to set, so
a program encrypting a file on somebody's behalf withholds nothing the person asking did not
withhold.

## 3. The randomness is an input, so the write stays a function of its arguments

§7.6.4.4.7 step (a) wants "16 random bytes of data using a strong random number generator",
§7.6.4.4.8 step (a) sixteen more, §7.6.4.4.9 step (e) four of filler, and §7.6.3.3 a fresh
16-byte initialisation vector in front of *every* string and stream. Where the file encryption key
itself comes from the standard does not say — Algorithms 8 and 9 both take it as already existing
and §7.6.4.3.3 only *retrieves* one — so it is thirty-two bytes from the same source, recorded as
the writer's choice.

All of it enters through one trait, `Entropy`, and `crypt::revision6` is a pure function of
(passwords, flags, `Unpredictable`). Three things follow. A test states the file it expects, which
is how the algorithms are pinned at all. RFC 0002 section 9's byte determinism survives for the
plaintext writer untouched. And the one place where it does not survive is the standard's own
exception rather than this writer's laxity: an encrypted output differs between writes because
§7.6.3.3 requires it to.

## 4. §7.6.2's exceptions, decided where the object's identity is

The four exceptions and Table 20's two more are applied on the way out exactly as on the way in,
in the same order and by the same predicates:

- the `/Encrypt` dictionary is written after every other object by `write_encrypt_dictionary`,
  which is the one path that does not go through the handler;
- a compressed object's strings are *not* encrypted, because §7.6.2 exempts "strings that are
  inside streams such as … compressed object streams, which themselves are encrypted", and
  `flush` encrypts the carrier;
- a signature dictionary's `/Contents` is skipped by `document::is_signature_dictionary`, the same
  predicate `encrypt_for_update` uses;
- the trailer's `/ID` is Table 15's direct, unencrypted pair, written by `identify`;
- a cross-reference stream never reaches the cipher, because `cross_reference_stream` builds it
  after every object;
- a stream naming a `/Crypt` filter is encrypted with the filter that entry names. **This is the
  one decision that had to be taken rather than copied**, and the rule is that the writer chooses
  what the reader will choose: `Protected::method` asks the output's own dictionary the three
  questions `Document::stream_method` asks, resolving an indirect `/Type` or `/Filter` through the
  output's numbering. A `/Crypt` naming `Identity`, or a filter this file does not carry, leaves
  the stream in the clear — which is what a reader will expect of it — and `Written::cleartext`
  counts it so that a caller can say which parts of a protected file are not.

Two further consequences. Errata Collection 3's Issue #439 appends the document catalog of an
encrypted document to §7.5.7's shall-not list, so `/Root` is written at the outermost level when
the output is encrypted and compressed when it is not. And the header is raised to 2.0, for the
reason `Form::Stream` raises it to 1.5.

## 5. What the tests are worth

A round trip through one crate is self-consistency. Three things make these more than that, and
they are the reason the ledger rows move.

**The halves are not one algorithm run backwards.** Algorithms 8, 9 and 10 are read back by
Algorithm 11, Algorithm 12, §7.6.4.3.3 steps (d) to (f) and Algorithm 13 — code that has opened
seven real producers' encrypted documents since long before this crate could write one.

**The clause's own structural statements are asserted directly**: `/U` and `/O` are 48 bytes in
three named sections whose salts are the bytes the source supplied, `/UE` and `/OE` are 32,
`/Perms` is 16.

**The `/Perms` test is a tamper.** §7.6.4.3.3 step (f) makes the decrypted block outrank the
plaintext `/P`, so the file is written, `/P -4` is rewritten in the bytes as `/P -8`, and the
reader still reports the permission the block carries. Asking a correctly written file what its
permissions are would have been answered by `/P` alone — trap 27's rule, applied before the
assertion was written.
