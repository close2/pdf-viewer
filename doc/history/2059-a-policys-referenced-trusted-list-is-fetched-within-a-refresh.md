# 2059 — A policy's referenced trusted list is fetched within a refresh, and held for its reference

Session 2059, a signatures round in the gwg worktree (`doc/questions/A800` item 3, `doc/todo/77` section 2
item 4; §12.8.3.4.4). ADR 2932; no question.

**The slice.** Item 4's two halves: (a) a list a policy's `TAsInTrustedList` references outside the LOTL's
tree, and (b) GPR-4.0-03's *should*, re-confirmed only.

**The premise.** (a) held: `anchors::Supplied::resolve` searched only the host's files, and `refresh`
asked only for the LOTL, its pivots and its pointers. The brief's ADR numbers slipped: the anchors are ADR
2874, not 2878 or 2879 (time stamps, a C host); the fetch is ADR 2882 and the window ADR 2886; belief
after authentication is ADR 2842, not 2852. No owner's word was needed: ADR 1728 already fetches a
location a document names, at A98's `Submissions` level, so Q887 was not written. TS 119 172-2 clause
4.22.2.1 b) says the anchor set *shall* be determined from the referenced list, and the reference is its
only way to obtain it.

**Built.** `refresh::refresh_with`: references the clauses did not obtain are fetched last, once, after
clause 4.1 passed; only `http` and `https` are asked for (`Fate::NotHttp`); a Member State's list is kept
where clause 4.2's signer checks pass; a list of trusted lists never is (`Fate::NoProcedure`, since clause
4.1 authenticates only from `OJEU-LOTL-Loc`); a kept file is spared ADR 2882's removal while it is
referenced. `Lists::read` holds a stored file from a location the LOTL does not point at as
`Reading::referenced`: it answers only its reference and is never searched by clauses 4.3 to 4.7
(PRO-4.2.4-03 and -04 obtain from the pointer alone). `anchors` resolves through it.

**Tests.** Three in `trusted_list/refresh/tests.rs`: the reference fetched last, kept, held apart and
determining anchors; a wrongly signed mirror, a LOTL copy and a `file:` reference not kept or not asked;
the kept file spared and then removed. **Plant**: the `EU` guard dropped (a LOTL copy is then kept) and
`Reading` not holding it apart make two tests fail; with both reversed, all pass.

**(b).** A failed TLS connection is still `submit::fetch`'s error, reported as "not fetched: …" with the
held copy kept, exactly as ADR 2886 says. TLS is untouched.

**Gates.** Over `pdf-signature`: fmt exit 0 (after one `cargo fmt`); clippy `-D warnings --all-targets`
exit 0 (after one fix, `!(..)` in an `if`); nextest 640 of 640, 5 skipped; rustdoc `-D warnings
--document-private-items` exit 0; `cd fuzz && cargo check --bins` exit 0; conformance after `git add -N`
exit 0.

**Left.** No host passes references to `refresh_with` yet. That needs the core's event naming a shown
policy's references, each window's question naming the locations, and the C ABI. §12.8.3.4.4 stays
`partial`.
