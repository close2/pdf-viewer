# 2764 — A submission that ends without form data runs the `postSubmit` events

Session 1976, a ledger round (K.2). Status: implemented. Completes decision 2 of ADR 2702 (the
`postSubmit` of a submit the host transmitted); extends ADR 2076 (an answer crossing to the confined
worker) and ADR 2624 (the command that follows a returned request).

## Context

XFA 3.3 page 447, "Standard post-submit processing on the client": "The client XFA processor
performs the following steps after any attempted submit operation: 1. Whether or not the submit
succeeded, examine the form for any postSubmit events [...] execute them. [...] 2. If the submit
succeeded and data was returned, merge the data from the Connection DOM into the Data DOM." Page 389
adds "[a]fter a cancelled or failed submit operation a postSubmit event also fires to allow clean-up
code to run". `viewer-core` fired `postSubmit` for a submission it did not send (cancelled, or a
composition that failed) and, through `Command::Respond`, for one whose answer was form data. A
submission the host transmitted and that ended any other way — no answer, a refusal, an answer that
is a PDF, a page or nothing — fired none, because only the host sees how a transmission ends and no
command told the core.

## Decision

1. **`Command::Unanswered { document }`** is that end: the named document's `postSubmit` events run
   with no primary event, as `Respond`'s do, and a document no longer open takes nothing. It
   crosses the confined wire (command kind 49) and the C ABI (`quorra_unanswered`).
2. **`viewer_host::submit::Returned::follow_up`** replaces `policy_copy`: the one command that
   follows a returned request's sentence — a policy's copy as before, and `Unanswered` for a
   submission whose answer `reply` does not import. Both read one function, `form_data`, so the
   window's import and the end cannot disagree. `quorra`, `quorra-confined` and every
   `NativeWindow` (`viewer-gtk`, `viewer-qt`) dispatch it where they dispatched the copy.
3. **A submission the host does not send ends the same way.** The reader's level refusing it, a
   `no` to its question, a question declined because another was waiting, and a thread that could
   not be started each dispatch `Unanswered` beside the sentence they already said: page 389's
   "cancelled or failed submit operation" is one the client attempted and did not complete, and the
   core ran its `preSubmit` before handing it over, so page 400's cleanup owes the `postSubmit`.

## Consequences

- A C caller handed a `QUORRA_EVENT_SUBMIT` calls `quorra_unanswered` wherever it does not call
  `quorra_respond`. The WinUI host transmits no submission and gains only the declaration.
