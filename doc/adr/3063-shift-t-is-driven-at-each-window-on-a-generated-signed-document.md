# 3063 — Shift + T is driven at each window on one generated signed document

Session 2090. Status: **implemented** for the C, Qt and `viewer-ui` windows; GTK left. Context: ADR 2936, ADR 2937,
record 2061 ("No window-level test drives a signed document through Shift + T"). Clauses: ISO 32000-2 §12.8.3.4.4;
ETSI TS 119 172-2 clause 4.22.2.1 b), TS 119 172-3's `TAsInTrustedList`.

## Decision

1. **One generator, included by `#[path]`.** `tools/conformance/test-support/signed_under_a_policy.rs` builds a
   one-page document whose signature names an explicit policy by digest and URL (EN 319 122-1 clause 5.2.9), the
   TS 119 172-3 policy whose one trust point is a `TAsInTrustedList`, and `bind`, which hands the copy back as a
   window's fetch does (`Command::PolicyCopy`). The reference names a host under `.invalid`; no real list, no corpus
   bytes. Included as `gpu_lock_hook.rs` is, so the windows drive the same document.
2. **Each test asks the window, at `ask`, before and after the copy is bound.** Before: the question names no
   reference (the calibration: only a bound policy is anybody's agreement). After: the window's held question
   carries the reference (`Asked::TrustedLists`, `Pending::TrustedLists`) and names it, and the yes starts the
   refresh; the C window also says the refresh fetches "the 1 list(s) a signature policy references". That held
   value is what the yes hands `Refresher::start`; what the refresh then does with a reference is
   `pdf-signature`'s, tested there.
3. **GTK is left.** Its Shift + T is the shared `requests::trusted_lists`, the code the C and Qt tests drive, but
   its host has no constructor a test can call without a running `gtk4::Application`; building one is its own slot.
