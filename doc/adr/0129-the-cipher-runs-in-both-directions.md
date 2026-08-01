# ADR 0129 — The cipher runs in both directions

*Session 144. Supersedes the refusal recorded in ADR 0121.*

## Context

Since the hundred-and-thirty-sixth session this tree writes ISO 32000-2 §7.5.6's incremental
update: a person fills in a form field, and the file they opened comes back with the new objects
and a new cross-reference section appended, the producer's bytes untouched underneath. ADR 0121
wrote down two costs of that writer, and this is the first of them:

> An *encrypted* document is refused, because §7.6 has to run on the way out and does not yet.

§7.6.2 is unambiguous about why the refusal was the honest answer rather than a gap:

> Encryption applies to all strings and streams in the document's PDF file

A writer that appended plaintext strings to an encrypted file would produce objects that the
file's own reader — including this one — decrypts into noise. Twenty-six of the corpus's 974
documents carry an `/Encrypt`, and every one of them was a document this program could open,
display, select text in, edit a field in, and then not save.

## The question

Writing encrypted objects needs three things: a cipher running the other way, a decision about
which strings the clause exempts, and an initialisation vector. Only the third was interesting.

## Decision

**One: the cipher is the same code with the direction reversed, and §7.6.3.1 says so.**

> RC4 is a symmetric stream cipher: the same algorithm shall be used for both encryption and
> decryption

So `Encryption::encrypt` is `Encryption::decrypt` with two branches changed. AES differs in
exactly the two ways the clause states — RFC 8018's pad is added rather than removed, and the
initialisation vector is generated rather than read off the front — and `Direction`, which
already existed in this module for Algorithm 2.B's one encrypting step, was the precedent that
the shape was right.

**Two: §7.6.2's exceptions are stated once, not twice.**

`Document::encrypt_for_update` is `Document::decrypt_object`'s mirror and shares its three
decisions rather than restating them: the object number the encryption dictionary occupies, a
signature dictionary's `/Contents`, and the method a stream's own dictionary selects through
`/Crypt`, `/Type /XRef` or `/Type /Metadata`. Restating them beside the writer would be two
statements of one clause, and the one place they could disagree is the one place a file becomes
unreadable by its own producer.

Two further exceptions are not shared and are not remembered either, because the writer builds
both objects itself and never hands them to the cipher: §7.5.8.2's "the cross-reference stream
shall not be encrypted and strings appearing in the cross-reference stream dictionary shall not
be encrypted", and Table 15's `/ID`, which "shall be direct objects and shall be unencrypted"
because NOTE 4 explains the circularity if they were not — the identifier is an input to the key
that would decrypt it.

**Three: the initialisation vector comes from the platform, and that is a new dependency.**

§7.6.3.2 step (d) has a sentence that binds a writer and nobody else, which is why it had no site
in this tree until now:

> the initialization vector is a 16-byte random number that is stored as the first 16 bytes of
> the encrypted stream or string

`getrandom` is the thinnest thing that provides it: no PRNG state, no `rand` ecosystem, one call
to `getrandom(2)`. The alternative considered and rejected was to derive the vector from the
plaintext and the key — the SIV construction — which needs no dependency and keeps a save
byte-for-byte reproducible. It is not what the clause says, and it would leak that two objects in
one update encrypt the same bytes. Exact conformance is the cheaper side of that trade.

**And one thing the writer gets wrong if it forgets:** §7.3.8.2's `/Length` is the length of the
stream *as it sits in the file*, and AES makes that longer than the plaintext by an initialisation
vector and a pad. `encrypt_value` rewrites it. A `/Length` still describing the plaintext ends the
stream inside its own ciphertext.

## The cost, written down

**An encrypted document's update is no longer byte-identical from one save to the next.** That is
the clause's requirement rather than a choice, and it costs one testing habit: `identify`'s
determinism — the second file identifier is a digest of the bytes so far, so that saving the same
edit twice produces the same file — survives only for an unencrypted document. The tests for an
encrypted save read the file back instead of comparing it, which is the stronger assertion anyway.

## Consequences, and how it is checked

`UpdateError::Encrypted` is gone and `UpdateError::Encryption` stands where it did, meaning
something narrower: the document's own key will not encrypt what is being written. Exactly one
shape of document reaches it, and the corpus has two of them — §7.6.4.1's "Documents in which only
file attachments are encrypted", where `/StmF` and `/StrF` are both `Identity` so the file opens
with no password at all, while an attachment's stream names a crypt filter through §7.6.6 and
cannot be read. Writing *that* object back would need a key this reader never had, so the update
refuses rather than replacing an unreadable attachment with an empty one.
`auth-event-ef-open.pdf` is the fixture; `encrypted-attachment.pdf` is the same shape and is not,
because its cross-reference table has to be rebuilt by scanning and it fails one refusal earlier.

The gate is `a_string_written_into_an_encrypted_document_comes_back_out_of_it`, over the six
documents `encryption.rs` uses to cover the handler's whole matrix — revisions 2, 3, 4 and 6
across RC4, AES-128 and AES-256. Each gets a string written into its catalog and read back
through the reader 974 corpus documents go through. **The second assertion is the discriminating
one**: the value is a sentinel no PDF contains, and it must not appear anywhere in the appended
bytes. Without it the test passes with the encryption removed altogether — a reader handed a
plaintext string cannot tell it from one it decrypted correctly, and only the file can.

Both assertions were confirmed to fail with `encrypt_for_update` stubbed to return its argument.

Outside the tree: all six updated files were opened by `mutool` and by `pdftotext`, both of which
extract text byte-identical to the original and read the written string out of the catalog.
`issue17069.pdf` draws one `aes padding out of range` warning from `mutool`, and it draws the same
warning on the original file.
