# 3309 — An unverified revision 6 `/Perms` block opens under `/P` as written, and is said

Session 2152. Status: accepted. Context: `doc/questions/A953` (option 2), audit 2131-T01
(`doc/reviews/2131-specification-audit-7-6.md`), ADR 2962, ADR 1044.

## Decision

§7.6.4.3.3 step (f) decrypts the 16-byte `/Perms` string under the retrieved file encryption key and
says to "Verify that bytes 9-11 of the result are the characters "a", "d", "b"". At revision 6 that
step is the only check of a key unwrapped from `/UE` or `/OE`. Where the block is absent, shorter than
16 bytes, or decrypts without the marker, the document still opens, and its flags are `/P` and
`/EncryptMetadata` as written (the owner's answer to Q953). The failure is never silent:

1. `pdf_syntax::crypt::Encryption` keeps `UnboundPermissions` (`Absent`, `Short`, `Unverified`) for a
   revision 6 handler that holds a key, and `Document::unbound_permissions` answers it. A document
   opened without a key (`/AuthEvent /EFOpen`) has no block to verify and reports nothing. Revision 5,
   deprecated, keeps reading its block where it verifies and says nothing where it does not.
2. `viewer_core::notes::about` turns it into a document note in the report a host asks for
   (`Command::Report`), off the launch path as every note there is: "this file's encryption states …
   (§7.6.4.3.3 step (f)), so its permissions are not bound to its encryption key: they are read from
   the /P entry as written, and nothing checked that the key was the one its producer used".

The sentence says what was not checked rather than that the file is damaged: the page decrypts either
way, and a wrong key would have shown as noise before this note mattered.

## Tests

`pdf-syntax`'s `serialize_encrypted::a_revision_6_perms_block_that_does_not_verify_opens_under_p_as_written`
writes a file with this tree's revision 6 writer and opens it with the block absent, cut short and
zeroed, each beside a `/P` that clears bit 3, and reads the plaintext flags, the reason and the page.
`viewer-core`'s `unbound_permissions_note` reads the note from the report and its absence for a
block that verifies.
