//! §9.6.5's Type 3 glyph descriptions, run as content streams.

use std::sync::Arc;

use pdf_render::Transform;

use crate::content::report::Unsupported;
use crate::content::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Runs one Type 3 glyph description, ISO 32000-2 §9.6.4.
    ///
    /// `text_rendering` is §9.4.4's text rendering matrix — everything the glyph is placed by
    /// except the font's own matrix, which is applied here because it is the font's business
    /// rather than the text object's.
    ///
    /// The steps §9.6.4 lays out for each character code are all here or in
    /// [`crate::type3::Type3Font`]: the encoding and `/CharProcs` lookups are the font's, and
    /// this does the rest — save the state, set the CTM, run the description, restore.
    pub(super) fn draw_type3_glyph(
        &mut self,
        font: &crate::type3::Type3Font,
        code: u32,
        state: &GraphicsState,
        text_rendering: Transform,
    ) {
        // §9.6.4 b): "If the name is not present as a key in CharProcs, no glyph shall be
        // painted." Neither that nor a code the encoding does not name is a failure — both
        // are defined outcomes — so neither is reported, and both still advance the text
        // position, which the caller does whatever happens here.
        let Some(glyph) = font.glyph(self.document, code) else {
            return;
        };

        // A glyph description may show text in a Type 3 font — another one, or its own — so a
        // file can build a recursion out of glyphs, and §9.6.4 says what one is worth since
        // Errata Collection 3 (Issue #111) inserted a paragraph below NOTE 1: "Implementations
        // also need to avoid potential infinite recursion if a Type 3 glyph description refers to
        // itself directly or indirectly. The result in all such cases is implementation-dependent."
        // So a description already running further out is refused where it is re-entered, below,
        // and reported by name; every other nesting — a glyph showing a different glyph of its
        // own font, a form, a tiling cell — is bounded by depth alone, at `MAX_FORM_DEPTH` in
        // `Interpreter::run`. The permission is the glyph's, and ADR 0793's argument against
        // refusing by identity is about the rest (ADR 1792).
        //
        // **At the re-entry rather than at the depth bound, because of what the bound costs a
        // glyph.** A description showing codes that reach itself runs a number of times
        // exponential in the bound before the bound is asked, so what ends it otherwise is the
        // memory its marks take (`MAX_LIST_BYTES`) or the person's stop, never a count (ADR 1809)
        // — 2.2 s of one page in a release build when ADR 1792 profiled it, and the same page
        // interprets in 71 ms refused here. The question
        // costs one comparison per description running, at most `MAX_FORM_DEPTH` of them, and is
        // asked before the stream is decoded.

        // Table 110's `/CharProcs`: each value "shall be a content stream that constructs and
        // paints the glyph for that character. The stream shall include as its first operator
        // either d0 or d1 , followed by operators describing one or more graphics objects." So
        // §7.8.2's prefix rule reaches a glyph description, and this clause makes the prefix
        // *faithful* in a way the general argument does not: `d0`/`d1` is required to be first,
        // so any prefix carrying a mark carries the glyph's own declaration ahead of it, and
        // Table 110's `/Widths` — not the description — supplies the advance, so what the
        // damage costs is marks inside this glyph and never the position of the next one.
        // Named by the glyph rather than by the code, because §9.6.4 step b) keys `/CharProcs`
        // that way and two codes may reach one description. `glyph` above returned `Some`, so
        // the encoding does name this code; the fallback is unreachable and is written rather
        // than unwrapped because nothing in the type system says so.
        let name = font.glyph_name(code).map_or_else(
            || "?".to_owned(),
            |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
        );
        let detail = format!("a Type 3 glyph description /{name} (§9.6.4)");
        // By the stream object, because that is what "itself" is: two codes, or two fonts, that
        // reach one description reach the same glyph procedure. A `/CharProcs` value that is not
        // a reference is not the stream Table 110 requires and is not asked about.
        let reference = font.glyph_reference(self.document, code);
        if reference.is_some_and(|reference| self.descriptions_running.contains(&reference)) {
            self.note(Unsupported::NestingCycle { stream: detail });
            return;
        }
        let Some(data) = self.content_stream(&glyph, &detail) else {
            self.note(Unsupported::Font {
                detail: format!("Type 3 glyph for code {code} could not be decoded"),
            });
            return;
        };

        // §9.6.4: "When the glyph description begins execution, the current transformation
        // matrix (CTM) shall be the concatenation of the font matrix (FontMatrix in the
        // current font dictionary) and the text space that was in effect at the time the
        // text-showing operator was invoked". Everything else is inherited: "Aside from the
        // CTM, the graphics state shall be inherited from the graphics state at the point of
        // invocation of the text-showing operator" — which is what cloning it does, and the
        // clone is also step c)'s save and restore, since nothing the description changes can
        // reach the caller's copy.
        let mut inner = state.clone();
        inner.transform = font.font_matrix().then(text_rendering);

        // §7.8.3's first step for a glyph description, which Errata Collection 3 put in front
        // of §9.6.4's own rule (Issue #128): "the stream dictionary of that glyph description
        // content stream". Resolved here rather than in `Type3Font` because the font holds the
        // `/CharProcs` dictionary and not the decoded streams — a glyph is read when it is
        // drawn — and cloned only where the stream states one, which is the rare case.
        let stated = self
            .document
            .get_key(&glyph.dict, "Resources")
            .as_dict()
            .cloned();

        let saved_uncoloured = self.uncoloured;
        self.glyph_depth = self.glyph_depth.saturating_add(1);
        self.enter_ledger_frame(crate::content::ledger::Route::Type3Glyph, reference);
        if let Some(reference) = reference {
            self.descriptions_running.push(reference);
        }
        // §7.8.3's search for a glyph description's resources ends at the page: the glyph
        // stream's own dictionary, then the Type 3 font dictionary that held `/CharProcs`, then
        // the page and what §7.7.3.4 gave it — Errata Collection 3's Issue #128, stated in full
        // on [`crate::type3::Type3Font::resources`], whose last parameter is the page's for that
        // reason. **Not the stream whose text-showing operator reached this glyph**, which is a
        // different dictionary whenever that stream is a form with `/Resources` of its own, and
        // which is what this call site passed until ADR 1059.
        // The page's dictionary is reached through an `Arc` rather than borrowed, because
        // `run` takes `&mut self`: one pointer clone per glyph description, against a `Dictionary`
        // that has to outlive the call. [`Interpreter::page_resources`](crate::content::Interpreter::page_resources) says why it is owned.
        let page_resources = Arc::clone(&self.page_resources);
        self.run(
            &data,
            font.resources(stated.as_ref(), &page_resources),
            &inner,
        );
        self.leave_ledger_frame();
        if reference.is_some() {
            self.descriptions_running.pop();
        }
        self.glyph_depth = self.glyph_depth.saturating_sub(1);
        // `d1` inside the description raised this; the description is over. Restoring rather
        // than clearing is what lets an uncoloured glyph invoke another one without the
        // inner one's end re-enabling colour for the rest of the outer.
        self.uncoloured = saved_uncoloured;
    }
}
