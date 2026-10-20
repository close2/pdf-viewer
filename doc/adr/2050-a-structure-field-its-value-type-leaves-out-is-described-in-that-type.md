# 2050 — A structure field its value type leaves out is described in that type

Session 1607, the archive slot of batch ninety-six. Status: **accepted**, **built** in
`pdf-transform`, `pdf-archive` and `pdf-model`. The census's last `not-built-yet` site with a document
at 2b. Code: `crates/pdf-model/src/xmp.rs` (`describe_fields`), `crates/pdf-archive/src/table/
metadata.rs` (`undescribed_fields`), `crates/pdf-transform/src/archive/prepare.rs`
(`prepare_field_descriptions`), `Rewrite::ExtensionFieldDescribed`.

## 1. What the site asks

`metadata/extension-schema-structure-fields-are-described` is ISO 19005-2 section 6.6.2.3.1's half
that reaches inside a structure: a property has to use the extension schema describing it, and a
value type's fields are described by its `pdfaType:field` (Table 5), a sequence of Table 6
structures, and by nothing else. The witness, veraPDF's `6-6-2-3-3-t03-fail-b`, describes
`CustomValueType` in `http://www.custom.com/1/` with no `pdfaType:field` (TechNote 0010's A029: an
empty array) and then states `cvt:field1` inside `fs:property1`. veraPDF reports it under 6.6.2.3.1
as well. Read against the 2005 XMP edition (ADR 1991), a structure's fields are its value type's to
define, so the packet contradicts its own description.

## 2. Described, as ADR 1245 describes a schema — a statement, not a loss

The two repairs are describing the field in its type or cutting the field (the limits document's
section 3.9, a loss). ADR 1245 settled the same question one level up: a schema the packet uses and
describes nowhere is given a container whose namespace, prefix, names and serialised value types are
the packet's own and whose three unstated fields are one fixed sentence each, a `Stated` decision.
Here the field's namespace and name are the packet's, its value type is the form its serialisation
shows (`serialised_value_type`: `Text`, `Seq Text`, `Bag Text`, `Lang Alt`), and Table 6's
`pdfaField:description` — the one field no file states — is `FIELD_DESCRIPTION`, saying the
converter wrote it. The report's `described_extension_fields` names every field. The brief's
hypothesis that the remedy is a `supply` held only as an option: nothing about a field's meaning is
needed to make the packet agree with itself, so the default is the statement and an operator's own
description is a configuration this version does not read yet.

## 3. Where it is written, and what is refused

`describe_fields` inserts into the producer's own description of the type — at the end of its
`pdfaType:field` sequence, or a new `pdfaType:field` before the description's close tag — each
inserted element declaring its own bindings, every other byte unchanged. It runs last of the six
packet editors (`Edited::fields`) and the result is read back with the row's own reading
(`undescribed_fields`) before it is kept; the catalog's own packet is read without inheriting
itself. Refused by name, each because describing would decide something the file does not say: a
field in another namespace than its type's (Table 5 gives a type one field namespace), a field whose
value is itself a structure (the undeterminable case one level down), and a type the packet does not
describe exactly once in a form a field can be added to (twice, only in the catalog's packet, or
self-closing).

## 4. Evidence

The witness converts to 2b: veraPDF 1.30.3 compliant (before: 6.6.2.3.1 test 2 failed), this
validator conforms, its page byte-identical at 72 and 720 dpi to the input's. `pdf-model` tests the
writer on both shapes and the refusal; `pdf-transform` tests the remedy and the namespace refusal.
