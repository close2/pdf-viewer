# 1162 — A derived file is protected where the caller supplies the means, and a redaction is refused where it does not

Status: accepted. Session 1162.
Amends: [ADR 1124](1124-a-redaction-removes-the-bytes-and-refuses-what-it-cannot-clear.md) §3,
whose sentence "[a]n encrypted document is refused outright, because the serializer emits no
`/Encrypt`" rested on a missing verb. Builds on
[ADR 1161](1161-encryption-on-the-way-out-is-asked-for-never-inherited.md) and
[ADR 0817](0817-a-serializer-that-emits-structure-and-never-content.md).
Context: `crates/pdf-transform/src/lib.rs` (`Protect`, `apply_protected`, `run_plan`),
`crates/pdf-transform/src/split.rs`, `merge.rs`, `pages.rs`, `optimize.rs`, `redact.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-transform/tests/redact.rs`.
Clauses: ISO 32000-2 §7.6.4.1, §7.6.4.2 (Table 22), §12.5.6.23.

## 1. The refusal was about a verb, and the verb now exists

The owner's revisit note on ADR 1124 said the premise was stated as a *condition* everywhere else
it appears and asked whether the serializer's gap should close. It has. Every factual claim in the
note held: ADR 0816's table and ADR 0817 both state the condition, `CLAUDE.md` scopes "§7.6
encryption on the way out" in, and `split` and `merge` did warn-and-emit over the same serializer
while `redact` refused. Nothing in it was wrong.

## 2. Where the request lives, and why it is not the plan's and not the policy's

`Protect` is a sixth argument to `apply_protected`, beside the sinks rather than inside the plan
or the policy, and each exclusion is a reason rather than a leftover.

**Not the `Plan`.** A plan is data a caller builds, logs, compares and serialises to JSON; a
password is none of those things. `viewer_core::Secret` is deliberately not `Clone` and has no
`PartialEq`, and a plan that held one would lose both derives for every verb.

**Not the `Policy`.** `Policy` is about what *this document* asserts over its reader — Table 22's
bits and §12.8.2.2's certification, read and obeyed or not. A `Protect` is the opposite direction:
what the *output* will assert over its next reader. Two questions that point opposite ways do not
belong in one struct because both have the word permission in them.

**Beside the `Source`.** §7.6.4.1's other password already lives there. `apply` and
`apply_borrowed` keep their signatures and delegate with `None`, so no caller outside this crate
changes.

## 3. The five verbs, and the one that refuses

`split`, `merge`, `pages` and `optimize` write whole files and keep what they did before when no
`Protect` is supplied: a warning naming §7.6, now saying *why* — no passwords were supplied —
rather than naming a writer that cannot. With one supplied, the output is encrypted and the
warning does not fire, because nothing was lost.

**`redact` refuses instead, and the asymmetry is the clause's.** §12.5.6.23 asks a redaction to
"remove all traces of the specified content"; a person who put a password on a document asked for
the rest of it not to be read either. Writing the survivors in the clear answers the first
requirement by breaking the second, and it does so for *every remaining page*. A split that loses
protection loses it from a file the caller asked to be cut, and the warning is proportionate; a
redaction that loses it has quietly traded one protection for another in the one operation whose
whole subject is what must not be readable. So the refusal stays where no protection is stated, and
its sentence now names what would lift it rather than what the program cannot do.

`archive` reads no `Protect` at all: ISO 19005 part 2 section 6.1.3 forbids an encrypted file, so a
conversion that encrypted its output would not conform.

## 4. The command line gets two flags and no permission words

`--encrypt-owner-fd <n>` and `--encrypt-user-fd <n>` read one line from a descriptor, for the
reason there is no `--password`: argv is public. They are refused on a verb that does not write a
whole file, because a flag silently ignored is worse than one refused.

**Every permission is granted, and that is a decision.** `CLAUDE.md` principle 3 makes a document's
restrictions the reader's to set and ranks them low; a program encrypting a file on somebody's
behalf has no business withholding from its next reader what the person running it did not ask to
withhold. Table 22's seven caller-decided bits are `pdf_transform::Access`, so a library caller
states them today. A flag that spells them on a command line is a separate argument nobody has
made, and this paragraph is why its absence is not a gap.

## 5. What an encrypted output costs the suite, stated rather than discovered

RFC 0002 section 9's first layer is byte determinism with no flag. It survives untouched for every
output this suite writes today, because every corpus gate and every comparison runs with no
`Protect`. An encrypted output is *not* byte-deterministic, and §7.6.3.3 is why: "the
initialization vector is a 16-byte random number". That is the standard's requirement, not this
writer's choice, and `Protect::write` carries the sentence.
