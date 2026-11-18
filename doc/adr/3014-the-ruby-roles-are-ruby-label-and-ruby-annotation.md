# 3014 — The ruby roles are `Ruby`, `Label` and `RubyAnnotation`

Session 2081. Status: **implemented** (a correction of a record). Context: review 2038 (findings D23, T49).
Clauses: ISO 32000-2 §14.8.4.7.3 Table 369.

ADR 2384 says the ruby roles already cross as "`RB`'s `Ruby` and `RT`'s `RubyAnnotation`". The mapping in
`viewer-accessibility`'s `role.rs` is, and was, `Ruby` to `Role::Ruby`, `RT` to `Role::RubyAnnotation`, and `RB`
to `Role::Label` with `RP`, `WT` and `WP`, because AccessKit has a role for the ruby assembly and for its annotation
and none for the base text. This ADR supersedes ADR 2384's sentence on that point; its decision about Table 380's
decoration and position is unchanged. The test `a_ruby_assembly_keeps_its_base_and_annotation_roles_and_order` in
`crates/viewer-accessibility/tests/tree.rs` pins the three roles, the order and the text of a valid Table 369
assembly at the consumer boundary.
