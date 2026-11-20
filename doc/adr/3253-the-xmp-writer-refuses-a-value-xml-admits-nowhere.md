# 3253 — The XMP writer refuses a value XML 1.0 admits nowhere, by name

Session 2138, an archive round in the `lane3` worktree. Status: **accepted** and built. Context:
ADR 3238 (the caller-side guard on a Table 349 value), ADR 2681 (a carried value the packet cannot
hold); `crates/pdf-model/src/xmp.rs` (`supplement`, `restate`, `record`, `xml_admits`,
`WriteError::NotXmlText`). Clauses: ISO 32000-2 §14.3.2, §14.3.3; ISO 19005-4 section 6.7.2.1;
XML 1.0 section 2.2.

## Context

XML 1.0's `Char` production admits no C0 control but tab, line feed and carriage return, and
neither U+FFFE nor U+FFFF, in any spelling: a character reference to one is not well-formed either.
ADR 3238 found the fuzzed `/Title` of `poppler-742-0-fuzzed` written into `dc:title` raw, a packet
no later edit could parse, and answered it in the caller. Every writer of `pdf_model::xmp` that
takes a caller's text could be handed the same value by the next caller.

## Decision

1. `supplement`, `restate` and `record` refuse a value holding such a character before writing
   anything, with `WriteError::NotXmlText { property, character }` naming the property as prefix
   and local name and the first such character. `supplement` refuses even where the packet already
   states the property: the value is the caller's to answer for, and whether it would have been
   written is not a reason to hand it over.
2. The predicate is public, `xml_admits`, and the archive's information rows use it rather than a
   copy of their own, so the caller's question and the writer's refusal cannot drift apart.
3. `packet`, which writes only the identification schema's fixed values and returns no `Result`,
   is not changed.

## Consequences

A public enum gains a variant (`#[non_exhaustive]` already); the fuzz workspace is checked. The
caller-side routes of ADRs 2681 and 3238 still decide what becomes of such a value; this refusal is
the floor under them.
