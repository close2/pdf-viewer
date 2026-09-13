# 1040 — The event a failed authorization fails, and the tree nobody walked

Session 1023. Status: **accepted**. Two decisions that share a session and nothing else: what ISO
32000-2 §7.6.6 Table 25's `/AuthEvent` decides, and that §14.12.2's document part hierarchy is
walked from the catalog so §14.13.8's associated files can be reached.

Context: `crates/pdf-syntax/src/crypt.rs`, `crates/pdf-model/src/document_part.rs`,
`crates/pdf-model/src/attachment.rs`. Ledger rows §7.6.6, §14.12.4, §14.12.4.1, §14.13, §14.13.8.
ADR 0031 is the decision the first half amends; ADR 0295 the shape the second half repeats.

## 1. `/AuthEvent` decides which event fails, and it had never been read

ADR 0031 settled what a document whose attachment alone is encrypted does when no password
authenticates: it opens, and the attachment refuses. The argument was two sentences of §7.6.6 —
authorization is needed "before the stream can be accessed", and an access without it "as an
error" — and it concluded that "`/AuthEvent /DocOpen` says *when* authorization is attempted, not
what fails if it does not succeed."

**It read one of the clause's two populations.** Table 25's own entry states the other half in the
sentence after the one ADR 0031 quotes:

> The event that shall be used to trigger the authorization that is required to access file
> encryption keys used by this filter. If authorization fails, the event shall fail.

The event is the one the entry names. For `DocOpen` — which is also the entry's default — that
event is the document open, so a key wanted there and not obtained is a document that does not
open. For `EFOpen` it is the embedded file's access, which is where ADR 0031's two sentences then
apply exactly as it said.

The decisive argument is not the wording but the consequence: **an entry whose two values produce
identical behaviour is an entry nobody has implemented.** Before this session nothing in the tree
read `/AuthEvent` at all, and every document whose `/StmF` and `/StrF` were `Identity` was
tolerated alike.

### What it changes, and the pair that shows it

`auth-event-ef-open.pdf` and `encrypted-attachment.pdf` are the same 2 719 bytes but for one line —
`/AuthEvent /EFOpen` in the crypt filter dictionary — with the same ciphertext in object 8. They are
pdf.js's own pair for this distinction. This reader now answers them differently: the first opens
with its attachment refused, the second asks for the password `DocOpen` requires.

The references do not distinguish them. `pdftoppm` opens both, `mutool` and `gs` refuse both. That
is evidence about a reading in principle 5's one permitted direction and nothing more; the answer
here comes off the clause, and it agrees with no reference on both files because none of the three
reads the entry either.

Held to `DocOpen` whatever it states: a filter named by `/StmF` or `/StrF`, which Table 25 requires
in as many words. A declared `/CF` entry is taken to be a used one — deciding otherwise would mean
scanning every stream in the file for a `/Crypt` specifier before answering whether the document
opens, which is the whole file on the launch path to settle a question the declaration settles.

An `/AuthEvent` naming neither value takes `DocOpen` with the absent entry. It is the stated
default and it is the conservative direction: the worst it costs is a password prompt that turned
out to be unnecessary, where the other guess would open a document whose key the clause says shall
have been obtained first. Table 25 states no "report it as unsupported" for this entry the way it
does for `/CFM`, so no file is refused over it.

### What `doc/md/` cannot say

The conversion lost Table 25's `EFOpen` row down to its last word and the final four words of the
`/StmF`-and-`/StrF` sentence. Both are quoted here in prose rather than as blockquotes for that
reason, and `mutool draw -F text -o - doc/ISO_32000-2_sponsored_EC3.pdf 106` prints the cell whole.
§7.6.6's body paragraph states `EFOpen` in text the conversion did keep, which is what the code
quotes.

## 2. A `DPart` nothing enumerates carries a file nothing can reach

§14.13.8 lets any `DPart` dictionary carry an `/AF` array. `attachment::associated` has always read
such an array from any dictionary — and nothing in the tree ever *had* a `DPart` to hand it, except
the one §12.6.4.5's `GoToDp` names. So a document whose only route to an embedded payload was a
part's `/AF` carried a file no panel could list and no host could extract.

That is ADR 0295's shape one clause family over, and §14.13.3's row records the identical fix for
the catalog's own array. `document_part::hierarchy` walks Table 29's `/DPartRoot` to Table 408's
dictionary, its `/DPartRootNode` to the root `DPart`, and the tree depth first;
`attachment::attachments` appends every part's associated files to §7.7.4's list, deduplicated by
embedded stream.

Two hops rather than one, because Errata Collection 3 Issue #609 replaces §14.12.2's
"DPartRootNode dictionary" with the DPartRoot dictionary and Table 408's cross-reference, so the
clause's two sentences now agree.

What bounds the walk is the clause: §14.12.2's "[a] child DPart dictionary shall not be referenced
by more than one parent DPart dictionary" makes a second reference a malformed file, so a reference
already walked is not descended into — which is also what makes a cycle finite, without a second
guard.

**No document in any corpus on this disk states a `/DPart` at all.** `cargo run --release -p
pdf-model --example witness_census -- DPartRoot DPart` reads 1 479 files, opens 1 452 of them, and
answers `0 raw, 0 as a name, 0 in stream data` for both terms — the `objects` layer being the one
to believe, because a raw byte scan cannot see a name inside a §7.5.7 object stream (ADR 0403).
Every test here is therefore hand-built and says so (trap 4), and each is calibrated against the
defect it should find rather than believed (trap 13).
