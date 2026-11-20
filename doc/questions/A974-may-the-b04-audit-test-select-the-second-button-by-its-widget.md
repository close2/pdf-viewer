Status: complete
Given: 2026-11-20, in conversation — transcribed by the round
Owes: update audit 2148's B04 test (selecting_the_second_of_two_same_named_buttons_turns_it_on_spec_audit) to send Entered::Selected(second.annotation) instead of Entered::Text(state) while keeping all its assertions and leaving B03's control unchanged, and remove B04 from expected-failures-current.json (a forms / audit-fix round)

> yes

Reading: adopts the recommendation — update audit 2148's B04 test to call `set_field` with `Entered::Selected(second.annotation)` (ADR 3311's widget-selecting edit) while keeping its assertions intact and leaving B03's first-kid control (`Entered::Text("Yes")`) unchanged.
