# 2233 — Stability follows source and context lifetimes

Status: accepted. Session 1699, 2026-10-22.

## Assessment

Colour selection and paint-time conversion have different lifetimes. Eager conversion
loses the selected source when intent or BPC changes. Resolving a name again at painting
can instead select a shadowing Form resource. Keep an immutable resolved profile and
components, then derive output under the current complete context. Declared PDF colour
space remains separate because operators and overprint depend on it.

Reuse the existing direct-ICC source for the immediate Indexed base. Another generic
source enum or persistent cache would add invalidation obligations without evidence
that it improves this bounded fix. Independent baseline failures and omission controls
provide stronger regression protection than two implementation paths agreeing.

The same boundary discipline applies elsewhere in this batch: global DefaultRGB writes
need all-context eligibility; Qt frame borrows end in one synchronous refresh; GTK
observations report a generation without becoming another rendering authority.
This identifies concrete coupling risks, not a universal diagnosis of every regression
or evidence that a wholesale rewrite would help.

## Test infrastructure

A mutation export reused the shared Cargo target and replaced a nextest executable
while its suite was running. Cargo's build lock does not protect later test execution.
Keep mutable exports in private target directories and verify tested artifact identity.
The failed suite and serialized passing retry are both retained in ADR 2235.
