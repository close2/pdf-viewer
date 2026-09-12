Status: complete
Given: 2026-09-09, by the owner's hand
Owes: the integer-key rename scheme (retrofit: unverified)

Implement it — my intermediate-names idea, in the shape the response proposes.

Integer keys with a stride of 100, counting from 00100; inserting before a page takes the gap between its neighbours. Renormalise only on an explicit request. The page labels a reader actually sees stay available through meta/. A rename that crosses directories, is not a well-formed key, or lands on an occupied key is refused by name; a rename is an append write like every other.