# 3497 — A chain of page actions ends where its last one goes

Session 2199. Status: accepted. Context: review 2197 (findings 2197-B03, 2197-D02); ISO 32000-2
§12.6.2, Table 196, §12.6.4.12, §12.6.4.15.

Table 196's `/Next` is "an array of action dictionaries that shall be performed in order", and
NOTE 1 has them "executed in order, each followed in turn by any actions specified in its Next
entry". `interact::resolve` therefore folds a chain's page-naming requests in that order: each
go-to, Named action (§12.6.4.12), document-part action and thread action replaces the page the
chain has reached, with the view stated by the same request, and a `NextPage` or `PrevPage`
counts from the page the previous request reached rather than from the page the chain started
on. Two `NextPage` actions advance two pages; a go-to followed by `NextPage` ends on the page
after the go-to's. A request naming no page of this document moves nothing. A movie action's
`/T` is looked for on the page the chain had reached when the movie action came.

The earlier rule — the first page named wins — rested on a reading that §12.6.2 states no rule
for a pair of jumps. It states one: the order.

§12.6.4.15's transition action has the processor "render the state of the page viewing area as
it exists after completion of the previous action and display it using a transition", and drawing
is suspended for the rest of the sequence. Where no later request moves the page, the state the
transition renders is the state the chain ends on, and `Event::Transition` followed by the jump is
exactly that. Where a later page request moves on, the clause asks for an intermediate frame —
the page reached so far, drawn with the transition — and then the chain's end. A host draws one
face per command (`Event::Transition` names an effect, and the next page drawn is its face), so
the transition is played into the end, and the difference is said out loud as a note naming the
page the transition belonged to (trap 5). Drawing the intermediate face needs an event that
carries a page to draw before the jump; this is the cost, written down.
