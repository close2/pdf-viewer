# 1202 — A refusal's ground may not be a fact the refusal manufactures

Session 1182. Status: **accepted** (a correction to one sentence's reasoning).
Amends the *ground* of [ADR 1134](1134-the-public-key-reader-key-is-a-host-input-deferred-behind-the-crate-graph.md),
not its verdict: §7.6.5's public-key security handlers stay refused. ADR 1134 stays exactly as
written.
Clauses: ISO 32000-2 §7.6.5, §7.6.5.1.

ADR 1134's closing sentence gives the refusal two grounds and only one of them is a ground. It
reads that the row "stays `reported` — nothing of the decryption is built, and the refusal is the
whole answer, correct today because no reachable recipient exists"; but reachability is what the
same ADR's Decision 2 decides, by making the reader's private key a host input on ADR 1076's
`Supply` shape "defaulting to *none*". No recipient can be reachable until the feature the ADR
defers is built, so the second half of that sentence is the design's own consequence offered as the
reason the design is right. The ground that stands alone is Decision 1's, and it is a fact about
this tree that nothing in the decision produces: the three parts §7.6.5 needs over `crypt.rs`'s
AES-CBC and SHA-256 all live in `pdf-signature`, which depends on `pdf-syntax`, so `crypt.rs`
cannot call them without a cycle, and the structural remedy — a shared crypto crate below both, or
the stack duplicated upward — is `doc/stack.md`'s rule and the owner's call in `doc/questions/Q66`,
answered in `A66` on 2026-09-16: a shared crate below both, built when a real trigger arrives.
**The refusal is therefore correct because the call cannot be made from where the clause is read
and the build waits on the trigger the owner named, and for no other reason.**

Two things this does *not* correct, both checked rather than assumed. The **census is not circular**,
although the revisit note that found the closing sentence says it is: the ledger's §7.6.5 row grounds
"no match to find" in whose certificates the five documents carry — four are one Apache bug report's
own attachments and the fifth is encrypted to a device, its recipient named `zune-tuner://` — which
is a fact about the documents and holds whatever key a host supplies. And the **row's note never
carried the circular sentence**, so there is nothing in `doc/conformance/ledger.toml` to fix; it
already ends on the dependency-graph blocker and on A66's trigger. The correction is confined to the ADR
record, which is why it is a record of its own.

The general rule, which is the reason this is an ADR and not a note: **a deferral may not be
justified by a state its own design guarantees.** Such a ground cannot fail, so nothing re-reads it
when the world changes, and `doc/habits.md`'s decay is silent by construction. Where a refusal has
a real blocker and a manufactured one beside it, the manufactured one is the sentence to delete.
