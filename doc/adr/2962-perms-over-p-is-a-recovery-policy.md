# 2962 — Reading `/Perms` over a disagreeing `/P` is a recovery policy, not the clause's rule

Session 2072. Status: **accepted.** Context: review 2038 finding D16, ISO 32000-2 §7.6.4.3.3 step (f),
§7.6.4.4.12 (Algorithm 13), ADRs 0031 and 0820.

`crypt::perms_block` called the decrypted `/Perms` block "authoritative", resting on step (f)'s "Bytes 0-3 of the
decrypted Perms entry … are the user permissions". The same step goes on "They shall match the value in the P key",
and Algorithm 13 says they "should match". A file whose two copies disagree is outside what the standard describes,
and neither clause says which copy then wins. The reader keeps reading the block (the copy under the key), and this
ADR records that as this reader's policy for a non-conforming file, superseding the reading of it as the clause's
rule. The doc comment says so, and `a_tampered_p_loses_to_the_perms_block_this_writer_wrote` already pins it as policy.
