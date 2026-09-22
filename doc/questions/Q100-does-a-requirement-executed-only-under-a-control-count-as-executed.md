# Q100 — Does a requirement executed only under a host control count as executed?

Status: open. Raised in session 1196, alongside ADR 1229.

## The situation

ISO 32000-2 §8.6.6.5 states a `shall` with no condition on it:

> For NChannel colour spaces, the components shall be evaluated individually; that is, only the
> ones not present on the output device shall use the alternate colour space of that component.

Until this session that sentence was not executed at all, and the ledger row said `departed` on
ADR 1193's argument: carrying it out means combining *n* colours where a fill needs one, and the
only combination the standard writes down is §10.8.3's separation simulation, which is conditioned
on a reader's request this program had no control for.

The control now exists (`--separations=on|off`, a `ViewState` input, ADR 1228) and the algorithm is
built (ADR 1229). So the sentence **is** executed — when a reader asks. The control's default is
`off`, and the default is argued rather than convenient: §10.8.2 says what the alternate route is
for on a display ("Alternate colour spaces are supplied for DeviceN and Separation colour spaces
so that files prepared for generation of separations can be displayed on other devices"), and
Table 70 says which of the two functions describes what an opaque fill needs — a `Separation`'s
describes "the appearance of that colourant alone", a `DeviceN`'s "the appearance of its colourants
in combination".

## The question, which is about the ledger's vocabulary and not about this clause

`implemented` means "every normative requirement in the clause is executed". Is a requirement
executed *on request, through a control a host supplies* executed?

Two readings, both defensible:

- **Yes.** `CLAUDE.md` principle 3 already says a policy is "asked once, in a place a host can
  supply", rather than hard-coded at the point of the operation. A capability that exists and is
  selected by the host is built; which way it defaults is a product decision with its own argument,
  and the note records it. On this reading §8.6.6.5 is `implemented`.
- **No.** The clause's `shall` carries no condition, and with the default every real document is
  drawn the other way. On this reading the word is `departed`, and what is departed from is the
  *default* rather than the capability.

It is left at `departed` pending an answer, which is the conservative of the two.

It will not be the last time this is asked. §10.8.3's own control, §12.8's four levels for a
document's restrictions (`doc/todo/38`), and any future "ask before the operation" policy all have
the same shape: a requirement whose execution a person switches.

## Recommendation

**`implemented`**, with the note stating the default and its argument — which is what the row's
note already carries. The alternative makes `departed` the permanent word for every requirement a
reader can switch off, which would eventually describe most of clause 12 as well, and would leave
the ledger unable to tell "we did not build it" from "we built it and you can turn it on".

If the answer is yes, one row moves: §8.6.6.5, `departed` → `implemented`. Nothing else in this
session depends on it.
