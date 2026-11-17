# 2923 — A SAML attribute is a policy's signer attribute by its URI name, and by nothing else

Session 2053, a signatures round. Status: implemented. Rows: §12.8.3.4.4. Context: ADRs 2787,
2895, 2922; ETSI TS 119 172-2 V1.1.1 clause 4.26 item 4; TS 119 172-3 V1.1.1 clause 4.26; OASIS SAML
2.0 core sections 2.7.3.1 and 8.2; OASIS SAML 2.0 profiles (`saml-profiles-2.0-os`) section 8.2.2;
RFC 3061; RFC 2141 section 2.

ADR 2895 left one question for the round that built a SAML reader: when does an attribute a SAML
assertion carries satisfy a policy's `AttributeIdMustBePresent`, or fall inside a set that names
types? X.509 has no rule for a SAML attribute, and SAML core section 8.2 leaves the interpretation of
an attribute's name to the application under every name format it defines. One convention is
written down by SAML itself:

- **Profiles section 8.2.2** names an X.500 directory attribute type in SAML by the RFC 3061 URN of
  its object identifier (`urn:oid:2.5.4.72`), under core section 8.2.2's URI name format.
- **A TS 119 172 identifier** is an object identifier (TS 119 172-3) or a URI (TS 119 172-2), which
  this tree already reads as the object identifier where it is an `urn:oid:` URN.

So, under the URI name format only: an object identifier is named by the attribute whose name is its
RFC 3061 URN (the `urn` and the namespace identifier compared without case, RFC 2141 section 2), and
a URI identifier by the attribute whose name is that URI. Under the URI format any other name is a
different attribute. Under any other format — `basic`, `unspecified`, or none stated — the name
means what an application decides, so the comparison is **not decided**: a mandated attribute such
an assertion might carry is not evaluated, never met and never broken (`signer_attributes::names`).

The `FriendlyName` is never compared: core section 2.7.3.1 makes it a display aid and nothing that
identifies the attribute. Values are not compared either: TS 119 172's value constraints have no
form (ADR 2895).
