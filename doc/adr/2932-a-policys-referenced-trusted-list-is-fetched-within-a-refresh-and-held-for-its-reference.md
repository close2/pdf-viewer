# 2932 — A policy's referenced trusted list is fetched within a refresh, and held for its reference

Session 2059, a signatures round (`doc/questions/A800` item 3, `doc/todo/77` section 2 item 4).
Status: implemented in `pdf-signature`; no host passes references yet. Rows: §12.8.3.4.4.
Context: ADRs 1085, 1728, 2842, 2874, 2882, 2886; `doc/questions/A98`; ETSI TS 119 172-2 V1.1.1
clause 4.22.2.1, TS 119 615 V1.3.1 GPR-4.0-01 and -03, clauses 4.1 and 4.2, cited by clause and
paraphrased (ADR 1085).

## Context

ADR 2874 determines a policy's `TAsInTrustedList` anchors from the files a host holds, and counts a
reference no file answers as unsearched. ADR 2882's refresh fetches the LOTL, its pivots and the lists
its pointers locate, and nothing else, so a reference to any other location stays unsearched for good.

## Is the fetch owed?

TS 119 172-2 clause 4.22.2.1 says that where a reference is found the set of anchors *shall* be
determined, and its step 1's initial set *shall* include every certificate in the referenced list. The
reference is `TLReference`, an `anyURI`, and the policy gives no other way to obtain the list, so a
verifier that never obtains it cannot determine the set and leaves the condition indeterminate. No
clause says "fetch", but every TS 119 615 procedure names its inputs by the location they are
retrieved from (PRO-4.1.4-01, PRO-4.2.4-04), and that is how the two texts read together. The
determination is owed, and so is obtaining the list from its location.

## Does it need the owner's word?

No. ADR 1728 already fetches a location a document names, the policy's published copy, at A98's
`Submissions` level, `ask` by default. A policy's list reference is the same kind of act: a GET that
tells a server signatures are checked here. A document never turns the network on. A reference only
adds locations to a refresh the person asked for at their own level.

## Decision

1. **`trusted_list::refresh::refresh_with`** takes the references. After clause 4.1 has passed and the
   Member States' lists have been fetched, each reference the clauses did not already obtain is
   fetched once, last, inside `MAX_FETCHES`. A reference that is not `http` or `https` is not asked
   for (`Fate::NotHttp`). `refresh` is `refresh_with` with none.
2. **A referenced list is kept only once it is believed** (ADR 2842). A Member State's list is kept
   where clause 4.2's checks of its signer pass under the authenticated LOTL's pointer for its
   territory. A list of trusted lists is never kept (`Fate::NoProcedure`), because clause 4.1
   authenticates one only as obtained from `OJEU-LOTL-Loc`. A list whose territory has no pointer is
   refused with clause 4.2's indication.
3. **A list kept for a reference is not clause 4.2's output.** PRO-4.2.4-03 and -04 obtain a Member
   State's list from its pointer's location alone, so `Lists::read` holds a stored file from any
   other location in `Reading::referenced`. It is still authenticated, it answers only the reference
   that names it (`anchors::Supplied::resolve`), and clauses 4.3 to 4.7 never search it. A stale
   copy at an old location can therefore never stand in for the current list.
4. **A file kept for a reference is spared ADR 2882's removal** while a refresh is given that
   reference, and removed as any other stored file once none is.
5. **The host's half is the question.** A host hands references to `refresh_with` only after asking
   with the locations named, as ADR 1728's question does, because the locations come from a document.

## GPR-4.0-03, unchanged

ADR 2886's reading stands. The *should* is not taken, TLS is not weakened, and a referenced location
whose connection fails is reported as not fetched, with the transport's reason, like every other.

## What is not done

No host passes references yet. That needs the core's event naming the references of a shown
signature's policy, each window's question naming the locations, and the C ABI's constant: the same
five hosts and ABI that ADR 2886 reached, which this round does not own.
