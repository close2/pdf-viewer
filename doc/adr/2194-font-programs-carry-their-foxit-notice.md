# 2194 — Font programs carry their Foxit notice

Session 1680, archive. Status: accepted; implementation under validation.
Owner answer A550 accepts Q550's notice recommendation.

ISO 32000-2 §14.3.2 Tables 347–348 permits an object-level metadata stream on
an actual font file. PDF/A-2 §6.6.2.1 and PDF/A-4 §6.7.2.1 require that metadata
to be well-formed XMP. The predefined Rights Management schema defines
`xmpRights:UsageTerms` as a language alternative. We attach an indirect
`/Type /Metadata /Subtype /XML` stream with one `rdf:Alt` default-language item.
Its decoded character content is the complete shipped `LICENSE_FOXIT`, including
the trailing newline; XML escaping changes serialization, not the notice text.

Provenance is exact bundled program bytes before any metric modification, not a
PDF font name or a search for a string. The ten shipped Foxit faces are covered in
the descriptorless shared stream, described substitution, existing-program
restatement, and composite substitution emitters. An unrelated supplied program
receives no invented notice. This covers newly embedded, substituted, and rewritten
bundled programs. Untouched pre-existing programs are not retroactively annotated:
an already conforming input still crosses unchanged. Identical newly allocated packets may share an
object. Existing unrelated font metadata is supplemented rather than discarded.

An existing rights property is accepted only when it is the exact canonical
language alternative we write. Namespace-aware removal on a scratch copy must
produce exactly the result of removing that one canonical element. Nothing is
removed from output. This proves its complete text without using the viewer
reader's trimmed display value, and accepts our supplemented packet on reuse.
Different terms, unreadable metadata, duplicate properties, or a license copied
into a comment cannot silently authorize replacement. They receive a named
refusal. Other valid equivalent XML serializations remain conservatively refused.

The existing PDF/A validator checks the resulting object metadata. The focused
witnesses cover all ten faces at 2b and 4, both metric-restatement routes, exact
text, preserved CreatorTool, repeat attachment, and conflicting existing terms.
The reader's XMP display semantics and licensing policy are unchanged.
