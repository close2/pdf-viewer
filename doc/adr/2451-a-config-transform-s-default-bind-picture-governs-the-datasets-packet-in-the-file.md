# 2451 — A `config` transform's default bind picture governs the `datasets` packet in the file

Session 1808, a clause round in batch 121. Status: implemented.

## Context

ADR 2434 left one question from chapter 15: does the default bind picture that a `config` packet's
`transform` supplies (XFA 3.3 page 527) govern a `datasets` packet already in the file, or only
data loaded from outside? XFA 3.3 is cited by chapter and printed page and paraphrased, never quoted.

## Decision

1. **It governs, because XFA 3.3 knows only one way data enters the Data DOM.** Pages 123–124
   define the data loader as the component that maps an XML data document into the XFA Data DOM,
   and the data unloader as its reverse. The `datasets` packet is "XML data content" an XFA form
   produces or consumes (page 1038), and the loader itself creates the `xfa`, `datasets` and `data`
   nodes that the packet carries (page 124). Chapter 15's rules are invoked through the
   configuration (pages 508–509), and the `transform` element takes effect during the data-load
   phase (page 898's element reference). No page exempts the packaged data. So a field whose
   template states no bind picture takes the transform's picture for the data element that the
   `ref` names. Rule 2 then parses with it and page 176 writes with it, as for a template's own
   `bind` picture (ADR 2434 §5).
2. **The data element's name comes from the binding.** That is the last step of a direct match or
   an explicit reference, or the name of a global binding. Page 527 names the element after any
   renaming by other rules, and those rules are refused here (3), so no renaming stands between the
   name and the picture. Two transforms that give one name different pictures cause the field to
   be named, not resolved by choosing one: the element reference calls overlapping transforms
   order-sensitive and not recommended, and states nothing that orders two pictures.
3. **The other mapping rules this program does not carry out leave every changed field named.**
   Sequences 1–13 of page 509's table — `startNode`, `excludeNS`, `attributes`, `record`, `range`,
   and a `transform`'s `presence`, `rename`, `nameAttr`, `whitespace`, `ifEmpty` and
   `groupParent` — change which data nodes exist, what they are called or what they hold. Page
   545's XSLT stages count too, but only with a `uri`, since page 898 makes an empty one mean the
   stage is not done. Binding past any of them could write the wrong node, so each changed field
   goes into `Written::xfa_unkept` with the rule's name (trap 5). `adjustData`, `incrementalLoad`,
   `window` and `uri` do not decide which node a field binds, and are not refused.
4. **Every `common` element's `data` is read.** XFA 3.3 places `common` under each application's
   element and does not say which one's `data` applies, so a rule stated anywhere is honoured or
   named. This is a choice. The locale reader's first `common` is its precedent (ADR 2414).

## Consequences

- The four hybrids carry `acrobat/common/data` with `adjustData` and empty XSLT URIs, and no
  `transform`, so they save as before. Three things fail when planted out (trap 13): the default
  picture, tested by `a_config_transform_gives_a_field_its_default_bind_picture`, which runs
  EXAMPLE 15.24's shape and a control naming another element; the refusal, tested by
  `a_mapping_rule_this_does_not_carry_out_leaves_the_field_named`; and the unit tests beside
  `pdf_model::xfa::config`.
- Not built: carrying out the other rules. Each needs its own reading, and step 4 does not need
  them.
