//! The external PDF renderers we compare against.
//!
//! Three independent implementations, deliberately chosen because they share no
//! rendering code: `poppler` (descended from xpdf), `mupdf`, and `ghostscript`. Their
//! *agreement* is the evidence the harness relies on — see [`crate::triangulate`].
//!
//! `pdfium` is a worthwhile fourth, being what Chrome ships and therefore the de facto
//! standard, but it is not packaged in the main Arch repositories.
//!
//! # Independence is a property of the renderer, and it is now recorded
//!
//! The word "independent" above was an assumption until it cost something. `mupdf` and
//! `ghostscript` both link `jbig2dec`, so on a page whose image is JBIG2 they are one
//! implementation and their agreement is not evidence — seven corpus pages were reported
//! as contradicting us on that basis, and we were right about all seven. [`Independence`]
//! now says so in the type, so a reference that cannot vote cannot silently be counted.
//!
//! `hayro` is here for the same reason in a stronger form: it is the only other
//! feature-complete pure-Rust PDF renderer, which makes it a genuinely useful fourth
//! *opinion* and the only renderer this project can fairly compare its speed against — and
//! it shares `skrifa`, `flate2`, `zune-jpeg`, `hayro-jbig2` and `hayro-jpeg2000` with us,
//! which is most of what a page is made of. It never votes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use pdf_render::Raster;

use crate::{HarnessError, png_io};

/// How long a reference renderer may take on one page before it is killed.
///
/// Thirty seconds, matching the per-document budget the corpus gate holds *us* to: a
/// reference that needs longer than we are allowed cannot be the oracle for that page
/// anyway, and a corpus holds files written to make a reader loop.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// An external reference renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Reference {
    /// `pdftoppm`, from poppler.
    Poppler,
    /// `mutool draw`, from `MuPDF`.
    MuPdf,
    /// `gs`, from Ghostscript.
    Ghostscript,
    /// `hayro`, a pure-Rust renderer, driven through our own `pdfref-hayro`.
    ///
    /// Reported, never counted: see [`Independence::Shared`].
    Hayro,
}

/// Whether a reference's agreement with us is evidence.
///
/// The triangulation rule in [`crate::triangulate`] rests on two implementations being able
/// to fail *independently*. Where they cannot, agreement means only that the shared code
/// behaved the same way twice, which is true of any code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Independence {
    /// Shares no code with this project. Its verdict counts.
    Independent,
    /// Shares implementation with this project, so its agreement proves nothing.
    ///
    /// Carries the reason, which is printed wherever the reference is, because a reader
    /// looking at four renders and three votes deserves to know which one abstained and why.
    Shared(&'static str),
}

impl Reference {
    /// Every reference the harness knows how to drive.
    pub const ALL: [Self; 4] = [Self::Poppler, Self::MuPdf, Self::Ghostscript, Self::Hayro];

    /// Whether this reference's agreement with us is evidence about our reading.
    ///
    /// # Why `mupdf` and `ghostscript` are still independent here
    ///
    /// On the overwhelming majority of pages they are two implementations of everything that
    /// matters. Marking them `Shared` wholesale would throw away the evidence of a thousand
    /// pages to describe a few dozen, so each piece of sharing is recorded where it applies —
    /// in `oracle.rs`'s `CONTRADICTED_SHARED_JBIG2_DECODER` and the groups beside it — rather
    /// than here.
    ///
    /// **This paragraph said "they share `jbig2dec`, and only that" and it was wrong**, which
    /// the five-hundred-and-eighteenth session found by asking the question the sentence
    /// answers. `ldd` prints the transitive closure; `objdump -p | grep NEEDED` prints what a
    /// binary actually asks for, and it is a different list:
    ///
    /// ```text
    ///                     poppler   mupdf   ghostscript
    ///   libfreetype.so.6     yes     yes      no — its own copy, below
    ///   libjpeg.so.8         yes     yes      yes
    ///   libopenjp2.so.7      yes     yes      yes
    ///   libz.so.1            yes     yes      yes
    ///   liblcms2.so.2        yes     no       yes
    ///   libjbig2dec.so.0     no      yes      yes
    /// ```
    ///
    /// So `jbig2dec` is the *narrowest* thing those two share rather than the only one, and
    /// two of the wider ones are decoders: **on a `DCTDecode` page and on a `JPXDecode` page
    /// all three voting references are one decoder.** Where that bites is the same place
    /// `jbig2dec` bites and in both directions — a consensus of three that is really a
    /// consensus of one, or, where the shared decoder fails, three answers that cannot form a
    /// consensus at all. `AMBIGUOUS_IRREVERSIBLE_JPEG_2000` and
    /// `AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG` are where the second has been named so
    /// far; `tests/jpeg2000.rs` is the answer that needs no reference, because it checks every
    /// corpus codestream against ISO/IEC 15444-5's own software.
    ///
    /// `hayro` is the opposite case. What we share with it — the font rasteriser, the
    /// deflate implementation, the JPEG decoder, and both new image codecs — is not one
    /// format's decoder but the substrate of nearly every page, so there is no useful subset
    /// on which it votes.
    ///
    /// # And all three of them are `FreeType`, though not all three link it
    ///
    /// Found with one `ldd` in the fortieth session and corrected with one `objdump` in the
    /// five-hundred-and-eighteenth. `pdftoppm` and `mutool` link `libfreetype.so.6`; **`gs`
    /// does not** — `libgs.so.10` names no `FreeType` in its `NEEDED` list, *defines* 194 `FT_*`
    /// symbols of its own and leaves none undefined, so it carries a statically linked copy
    /// and the `ldd` line was reaching it through `libfontconfig`. It is not the same copy
    /// either: the system library exports `FT_Palette_Select` and Ghostscript's does not, which
    /// is a build configuration rather than a version.
    ///
    /// The substance survives the correction and is worth more for being measured: all three
    /// rasterise glyphs with `FreeType`'s code while this tree uses `skrifa` and `tiny-skia`, so
    /// on a page whose difference is a letter's edges the three are one family and we are the
    /// only second opinion — the same shape as `jbig2dec` above and far more widely reachable.
    ///
    /// **And the ambiguous bucket measures it.** Over the oracle's 786 ambiguous pages, taking
    /// the mean absolute difference of all ten renderer pairs from the artefacts already on
    /// disk, the closest pair of the ten is `ours + hayro` on **651** of them — and on the 670
    /// judged as text, on **612**. The median distances are ours-to-`hayro` **1.94** of 255
    /// against **5.39** for the closest two of the three that vote. `hayro` shares `skrifa`
    /// with this tree and nothing else about a page; it is a separate interpreter written by
    /// other people, and it is the one reference that is not allowed to vote. So an `ambiguous`
    /// text page in that bucket is usually **two camps, and the voting camp is the one that
    /// cannot agree with itself** — which is not evidence that we are right, and is exactly
    /// what the verdict is made of. `doc/todo/00-ambiguous-bucket.md` has the run.
    ///
    /// It is recorded here and **not** acted on, for the same reason `jbig2dec` is not:
    /// marking three references `Shared` for text would leave the gate with nothing to vote
    /// on, when what they share is one component of a page and everything else about it —
    /// colour, geometry, images, transparency, and *which* glyph was selected — is still
    /// three independent readings. Where it bites is the bound rather than the vote, and
    /// `Tolerance::widened_to` carries that half of the argument.
    #[must_use]
    pub fn independence(self) -> Independence {
        match self {
            Self::Poppler | Self::MuPdf | Self::Ghostscript => Independence::Independent,
            Self::Hayro => Independence::Shared(
                "shares skrifa, flate2, zune-jpeg, hayro-jbig2 and hayro-jpeg2000 with us",
            ),
        }
    }

    /// The references whose agreement counts as evidence.
    #[must_use]
    pub fn voting() -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|reference| reference.independence() == Independence::Independent)
            .collect()
    }

    /// Short name used in reports and artefact filenames.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Poppler => "poppler",
            Self::MuPdf => "mupdf",
            Self::Ghostscript => "ghostscript",
            Self::Hayro => "hayro",
        }
    }

    /// The executable this reference needs.
    #[must_use]
    pub fn program(self) -> &'static str {
        match self {
            Self::Poppler => "pdftoppm",
            Self::MuPdf => "mutool",
            Self::Ghostscript => "gs",
            Self::Hayro => "pdfref-hayro",
        }
    }

    /// Package providing this renderer, for error messages that are actionable.
    #[must_use]
    pub fn package_hint(self) -> &'static str {
        match self {
            Self::Poppler => "poppler (Arch) / poppler-utils (Debian)",
            Self::MuPdf => "mupdf-tools",
            Self::Ghostscript => "ghostscript",
            Self::Hayro => "cargo build --release -p hayro-compare --bins",
        }
    }

    /// The flag that makes this program print its version and exit.
    ///
    /// Not uniform: `gs` takes `--version`, while `pdftoppm` and `mutool` take `-v` and
    /// treat `--version` as an input filename — which produces a confusing "couldn't
    /// open file '--version'" rather than a version string.
    #[must_use]
    pub fn version_flag(self) -> &'static str {
        match self {
            Self::Poppler | Self::MuPdf => "-v",
            Self::Ghostscript => "--version",
            // It has no version flag; run with no arguments it prints its usage and exits
            // non-zero, which `is_available` treats as present, as it does for `mutool`.
            Self::Hayro => "",
        }
    }

    /// Where the executable is, which for our own renderer is not `PATH`.
    ///
    /// `pdfref-hayro` is built by this workspace, so it sits beside whatever is running:
    /// next to the executable, or one directory up, because Cargo puts test binaries in
    /// `target/<profile>/deps/` and programs in `target/<profile>/`. `PDFREF_HAYRO`
    /// overrides both. This is the same search `pdf-sandbox` does for its worker and it
    /// carries the same caveat — see trap 10 in the handover — that Cargo will not rebuild
    /// another package's binaries for you.
    #[must_use]
    pub fn program_path(self) -> PathBuf {
        if self != Self::Hayro {
            return PathBuf::from(self.program());
        }
        if let Some(named) = std::env::var_os("PDFREF_HAYRO") {
            return PathBuf::from(named);
        }
        let executable = std::env::current_exe().unwrap_or_default();
        let directory = executable.parent().unwrap_or(Path::new("."));
        for candidate in [
            directory.join(self.program()),
            directory.parent().unwrap_or(directory).join(self.program()),
        ] {
            if candidate.is_file() {
                return candidate;
            }
        }
        // Falls back to the bare name so that an installed copy on `PATH` still works, and
        // so that `is_available` reports absence rather than this function failing.
        PathBuf::from(self.program())
    }

    /// Returns `true` if the executable can be run.
    #[must_use]
    pub fn is_available(self) -> bool {
        // Spawning is the test, not the exit status: `Command::status` fails only when
        // the program cannot be run at all. `mutool` with no subcommand exits non-zero
        // while being perfectly present, so judging by status would report it missing.
        Command::new(self.program_path())
            .arg(self.version_flag())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    /// Returns every reference whose executable is present.
    #[must_use]
    pub fn available() -> Vec<Self> {
        Self::ALL.into_iter().filter(|r| r.is_available()).collect()
    }

    /// Renders `page` of `pdf` at `dpi`, writing intermediates under `work_dir`.
    ///
    /// `page` is one-based, as it is in all three renderers' own command lines and in the
    /// specification. It is a parameter rather than a default because a harness whose
    /// choice of page is implicit is a harness that can silently compare two different
    /// pages; all three renderers seek to a page through the cross-reference table, so a
    /// late page costs no more than the first.
    ///
    /// Bounded by [`DEFAULT_TIMEOUT`]. There is no unbounded variant on purpose: these
    /// renderers are pointed at untrusted files, and a corpus contains files built to make
    /// a reader loop.
    ///
    /// # Errors
    ///
    /// [`HarnessError::RendererMissing`] if the executable is absent,
    /// [`HarnessError::RendererFailed`] if it exits non-zero, exceeds the budget, or
    /// produces no output, and [`HarnessError::Png`] if its output cannot be decoded.
    pub fn render(
        self,
        pdf: &Path,
        page: u32,
        dpi: u32,
        work_dir: &Path,
    ) -> Result<Raster, HarnessError> {
        self.render_within(pdf, page, dpi, work_dir, DEFAULT_TIMEOUT)
    }

    /// Renders `page` of `pdf` at `dpi`, giving the renderer at most `budget`.
    ///
    /// # How the budget is enforced
    ///
    /// By polling [`std::process::Child::try_wait`] and killing the process when it
    /// expires, because the standard library has no wait-with-deadline. The renderer's
    /// two output streams go to a log file beside its image rather than to a pipe: a pipe
    /// nobody drains while polling would deadlock a chatty renderer against its own
    /// buffer, and a file keeps the diagnostics as evidence in the same place as
    /// everything else the run produced.
    ///
    /// # Errors
    ///
    /// As [`Self::render`], with [`HarnessError::RendererFailed`] also reporting a
    /// renderer that exceeded `budget` and was killed.
    pub fn render_within(
        self,
        pdf: &Path,
        page: u32,
        dpi: u32,
        work_dir: &Path,
        budget: Duration,
    ) -> Result<Raster, HarnessError> {
        if !self.is_available() {
            return Err(HarnessError::RendererMissing {
                reference: self,
                package: self.package_hint(),
            });
        }

        std::fs::create_dir_all(work_dir).map_err(|e| HarnessError::RendererFailed {
            reference: self,
            detail: format!("could not create {}: {e}", work_dir.display()),
        })?;

        let output_path = work_dir.join(format!("{}.png", self.name()));
        // A renderer that fails after a previous run succeeded would otherwise be judged
        // by the stale image still sitting there.
        let _ = std::fs::remove_file(&output_path);
        let mut command = self.build_command(pdf, page, dpi, work_dir, &output_path);

        // **Both streams, into one log.** `stdout` went to `null` until the
        // seven-hundred-and-seventh session, which threw away the only sentence Ghostscript
        // writes about *why* it stopped: on a file with no §7.5.2 header it prints `Error:
        // /undefined in obj` and its operand stack to **stdout** and only `Unrecoverable error,
        // exit code 1` to stderr, so the gate's line named the consequence and discarded the
        // cause. `Reference::version` has known which stream `gs` speaks on since it was written;
        // nothing had joined that to this. No renderer here writes its image to stdout — all
        // three are given an output path — and a healthy `gs` run writes zero bytes there,
        // measured, so this costs nothing on the pages that work. ADR 0574.
        //
        // One file description shared by `try_clone`, rather than two opens: two handles at
        // offset zero would overwrite each other's lines, and a shared offset is what keeps the
        // renderer's own interleaving.
        let log_path = work_dir.join(format!("{}.log", self.name()));
        if let Ok(log) = std::fs::File::create(&log_path) {
            match log.try_clone() {
                Ok(second) => {
                    command
                        .stdout(std::process::Stdio::from(log))
                        .stderr(std::process::Stdio::from(second));
                }
                Err(_) => {
                    command
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::from(log));
                }
            }
        }

        let status = self.wait_within(&mut command, budget)?;

        // Ghostscript and mutool both report real problems on stderr while still exiting
        // zero, so success is judged by whether an image appeared, not by exit status
        // alone. The status is still reported when it is non-zero, because that is the
        // more informative failure.
        //
        // **An empty file is not an image**, and until the seven-hundred-and-seventh session
        // this condition was `exists()` alone. `mutool draw` creates its `-o` file before it
        // decides it cannot draw the page, so a document whose page tree it cannot recover
        // left a *zero-byte* PNG behind — which passed this test, reached the decoder, and
        // came back as `HarnessError::Png` saying "unexpected end of file". Two things
        // followed, and both are trap 3's shape one step further in: the gate printed the
        // *harness's* diagnosis where the renderer's own was sitting in the log beside it
        // ("argument error: invalid page number: -1", after `format error: malformed page
        // tree`), and `cache::write_entry` declines to remember a `Png` error — correctly,
        // since a PNG this harness cannot read is not a property of the document — so those
        // pages re-ran `mutool` on every run for ever. A renderer that produced no bytes has
        // produced no output, which is what this now says. ADR 0574.
        let empty = std::fs::metadata(&output_path).is_ok_and(|file| file.len() == 0);
        if !output_path.exists() || empty {
            return Err(HarnessError::RendererFailed {
                reference: self,
                detail: format!(
                    "produced no output (status {:?}): {}",
                    status.code(),
                    diagnosis(&log_path)
                ),
            });
        }

        // And where there *are* bytes and they are not a PNG this harness can read, the
        // renderer's own last line goes in beside the decoder's: the two answer different
        // questions — what the file is, and why the renderer says it wrote one — and a
        // reader of the gate's line needs the second to act on the first. The **variant is
        // deliberately unchanged**: `cache::write_entry` remembers a `RendererFailed` and
        // not a `Png`, and a half-written image is the one failure here that can be the
        // machine's rather than the document's, so it must stay unremembered.
        png_io::read(&output_path).map_err(|error| match error {
            HarnessError::Png { path, message } => HarnessError::Png {
                path,
                message: format!(
                    "{message} ({} says: {})",
                    self.program(),
                    diagnosis(&log_path)
                ),
            },
            other => other,
        })
    }

    /// The sentences in which this renderer says it did not produce what the page asked for.
    ///
    /// # Why this is a vocabulary and not a severity
    ///
    /// Each of these programs states its own severity — `poppler` writes `Syntax Error` and
    /// `Syntax Warning`, `mupdf` prefixes every line with `warning:`, `syntax error:`, `format
    /// error:` or `library error:`, `ghostscript` writes `WARNING` and `FATAL ERROR` — and
    /// reading the severity is the obvious rule and the wrong one. Over the oracle's own corpus
    /// **28 901** of `poppler`'s `Syntax Error` lines are `Type mismatch in PostScript function`,
    /// on pages it draws correctly, and `mupdf`'s `format error: incorrect number of xref entries
    /// in trailer, repairing` says in its own text that it recovered. A predicate on severity is
    /// trap 11's shape exactly: a condition that fires wherever the trouble *could* be involved.
    ///
    /// What is read instead is what the program says it **produced**: a sentence in which it says
    /// it could not make a picture, of the page or of an image on it. That is a narrower claim
    /// than "something went wrong", and it is the only one [`crate::consensus_abstentions`] needs,
    /// because that rule asks one question — is this flat sheet a reading of the page, or a
    /// failure at it? A complaint about *reading the file* is deliberately outside it: `mupdf`
    /// repairs a broken cross-reference table and draws the page, and 14 flat sheets of the
    /// oracle's corpus say `warning: repairing PDF document` beside output nobody disputes.
    ///
    /// # The vocabulary, per program, and where each entry was read
    ///
    /// Derived from every log the oracle's corpus produces — its `<name>.log` files, which
    /// [`crate::cache`] has stored beside the rasters since the eight-hundred-and-forty-second
    /// session — rather than from the three programs' source, which is a claim with no gate on it
    /// (trap 9's last bullet).
    ///
    /// | program | sentence | what it is |
    /// |---|---|---|
    /// | `mupdf` | `library error:` | its own severity for a library that failed to produce data: `cannot decode jbig2 image`, `cannot complete jbig2 image`, three `zlib error:` variants — every one of the five in the corpus is a stream that yielded nothing |
    /// | `mupdf` | `cannot draw '` | it abandoned the page it was given |
    /// | `mupdf`, `ghostscript` | `failed to decode` | **`jbig2dec`'s** words, reaching the log through both programs |
    /// | `ghostscript` | `FATAL ERROR` | its severity for a decoder that stopped |
    /// | `ghostscript` | `Page drawing error occurred` | its own statement that drawing the page failed |
    /// | `ghostscript` | `Unrecoverable error` | its own statement that it stopped |
    ///
    /// **`ghostscript` labels one of these a `WARNING` and it is still a refusal**, which is
    /// worth stating because it looks like an inconsistency and is not. `jbig2dec WARNING failed
    /// to decode; treating as end of file` is two statements: the severity is `ghostscript`'s
    /// judgement about how to carry on, and *failed to decode* is `jbig2dec`'s statement about
    /// what it produced. Testimony is the second of those. The same sentence reaches `mupdf` as
    /// `warning: jbig2dec warning: failed to decode…`, under a `library error:` of `mupdf`'s own
    /// — two programs labelling one library's words differently, which is the clearest evidence
    /// there is that the severity is not the thing to read.
    ///
    /// **`poppler` has no entry, and that is a measurement rather than an omission.** Its
    /// refusals are worded as ordinary syntax errors — `Syntax Error (681): Too many symbols in
    /// JBIG2 symbol dictionary` on the page that occasioned this work, `Could not find start of
    /// jpeg data` on four others — and nothing in the wording distinguishes them from the tens of
    /// thousands of `Syntax Error` lines it writes about defects it recovers from. Adding one of
    /// its sentences and not the others would be a list fitted to the pages this round wanted to
    /// move. So `poppler` gives no testimony this rule can read, its flat sheets are judged by
    /// pixels alone exactly as before, and the oracle prints every sentence this condition did
    /// **not** match so that a later round can widen it on evidence rather than on taste.
    ///
    /// **`hayro` has none either, for a different reason**: it never votes
    /// ([`Independence::Shared`]), so no abstention of its can change a verdict.
    #[must_use]
    pub fn refusals(self) -> &'static [&'static str] {
        match self {
            Self::Poppler | Self::Hayro => &[],
            Self::MuPdf => &["library error:", "cannot draw '", "failed to decode"],
            Self::Ghostscript => &[
                "FATAL ERROR",
                "Page drawing error occurred",
                "Unrecoverable error",
                "failed to decode",
            ],
        }
    }

    /// What this renderer said about the page whose evidence is in `work_dir`.
    ///
    /// Read back off disk rather than returned from [`Self::render`], because the same file has
    /// to be there after a cache hit — [`crate::cache`] restores it — and a value returned only
    /// by the uncached path would make a rule that reads it reach two different verdicts on two
    /// runs of the same corpus.
    #[must_use]
    pub fn testimony(self, work_dir: &Path) -> Testimony {
        match std::fs::read_to_string(work_dir.join(format!("{}.log", self.name()))) {
            Ok(log) => Testimony::of(self, log),
            // A log that is not there is a renderer that said nothing, for this rule's purpose:
            // see [`Testimony`] for why the two are deliberately one case.
            Err(_) => Testimony::silent(self),
        }
    }

    /// Runs a command, killing it if it outlives `budget`.
    fn wait_within(
        self,
        command: &mut Command,
        budget: Duration,
    ) -> Result<std::process::ExitStatus, HarnessError> {
        /// How often the child is checked. Short enough that a killed renderer does not
        /// hold up a corpus run, long enough that polling costs nothing measurable.
        const POLL: Duration = Duration::from_millis(20);

        let mut child = command.spawn().map_err(|e| HarnessError::RendererFailed {
            reference: self,
            detail: format!("could not run {}: {e}", self.program()),
        })?;

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {}
                Err(e) => {
                    return Err(HarnessError::RendererFailed {
                        reference: self,
                        detail: format!("could not wait for {}: {e}", self.program()),
                    });
                }
            }
            if started.elapsed() > budget {
                // Both failures are reported, and neither is allowed to mask the timeout
                // itself: a kill that fails means the process is already gone.
                let _ = child.kill();
                let _ = child.wait();
                // Its own variant rather than a `RendererFailed` whose message says so,
                // because this is the one outcome here that is not a function of the
                // document: it depends on what else the machine was doing. `crate::cache`
                // has to be able to tell it apart from a refusal, which is deterministic.
                return Err(HarnessError::RendererTimedOut {
                    reference: self,
                    budget,
                });
            }
            std::thread::sleep(POLL);
        }
    }

    /// The invocation this reference would run, as text, with the paths that vary removed.
    ///
    /// This exists so that [`crate::cache`]'s key can be derived from the command line
    /// itself rather than from a hand-maintained list of the things that affect it. The
    /// difference matters: a list has to be updated when `-cropbox` is added, and the
    /// consequence of forgetting is not a failure but a comparison against a render made
    /// under the old flag. Deriving the key from [`Self::build_command`] makes that
    /// impossible by construction — a flag that is not in the signature is a flag that is
    /// not passed to the renderer either.
    ///
    /// Two substitutions keep the signature stable across runs and machines: the document's
    /// own path becomes `<pdf>`, since the document is in the key by content, and anything
    /// under `work_dir` becomes `<out>`, since where a page's evidence is written says
    /// nothing about what was drawn.
    #[must_use]
    pub fn command_signature(
        self,
        pdf: &Path,
        page: u32,
        dpi: u32,
        work_dir: &Path,
    ) -> Vec<String> {
        let output = work_dir.join(format!("{}.png", self.name()));
        let command = self.build_command(pdf, page, dpi, work_dir, &output);
        let pdf = pdf.to_string_lossy().into_owned();
        let work_dir = work_dir.to_string_lossy().into_owned();

        let mut signature = vec![self.program().to_owned()];
        signature.extend(command.get_args().map(|argument| {
            let argument = argument.to_string_lossy();
            if argument == pdf {
                return "<pdf>".to_owned();
            }
            argument.replace(&work_dir, "<out>")
        }));
        signature
    }

    /// Builds the command line for this renderer.
    ///
    /// Antialiasing is enabled everywhere it can be. The alternative — turning it off to
    /// make comparison exact — sounds attractive but is wrong: it would compare a
    /// configuration nobody actually views documents in, and would hide precisely the
    /// coverage bugs that matter at edges.
    ///
    /// # Every renderer is told to use the crop box
    ///
    /// ISO 32000-2 §7.7.3.3 Table 31 gives a page its `/CropBox`, and §14.11.2.1 says what
    /// the box is for:
    ///
    /// > The crop box defines the region to which the contents of the page shall be clipped
    /// > (cropped) when displayed or printed.
    ///
    /// That is what a viewer shows. (This sentence used to be attributed to §7.7.3.3 here,
    /// which is where the entry is defined and not where it is explained — the kind of drift
    /// the conformance checker in `tools/conformance` now catches.)
    ///
    /// `mutool draw` does this by default; `pdftoppm` and `gs` default to the
    /// media box and have to be told, which is what `-cropbox` and `-dUseCropBox` are for.
    ///
    /// Leaving the default in place was not a neutral choice. Over the pdf.js corpus it
    /// put 54 documents' first pages permanently beyond comparison — the harness could not
    /// reconcile a 329x204 crop against a 612x792 sheet and reported a geometry
    /// disagreement — and on a page whose crop box has the same *size* as its media box but
    /// a different origin, it would have compared a correct render against a displaced one
    /// and called us wrong. The clause decides this; agreement with `mutool` is only
    /// evidence that the clause was read the same way twice.
    fn build_command(
        self,
        pdf: &Path,
        page: u32,
        dpi: u32,
        work_dir: &Path,
        output: &Path,
    ) -> Command {
        match self {
            Self::Hayro => {
                let mut command = Command::new(self.program_path());
                command
                    .arg(pdf)
                    .arg(page.to_string())
                    .arg(dpi.to_string())
                    .arg(output);
                command
            }
            Self::Poppler => {
                let prefix = work_dir.join(self.name());
                let mut command = Command::new(self.program_path());
                command
                    .arg("-r")
                    .arg(dpi.to_string())
                    .arg("-png")
                    // `-f` and `-l` select the page; `-singlefile` then suppresses the
                    // page-number suffix, so one page comes out at a predictable path.
                    .arg("-f")
                    .arg(page.to_string())
                    .arg("-l")
                    .arg(page.to_string())
                    .arg("-singlefile")
                    .arg("-cropbox")
                    .arg("-aa")
                    .arg("yes")
                    .arg("-aaVector")
                    .arg("yes")
                    .arg(pdf)
                    .arg(prefix);
                command
            }
            Self::MuPdf => {
                let mut command = Command::new(self.program_path());
                command
                    .arg("draw")
                    // Its default already, stated so that a change of default cannot
                    // silently move what this compares against.
                    .arg("-b")
                    .arg("CropBox")
                    .arg("-r")
                    .arg(dpi.to_string())
                    .arg("-o")
                    .arg(output)
                    .arg(pdf)
                    .arg(page.to_string());
                command
            }
            Self::Ghostscript => {
                let mut command = Command::new(self.program_path());
                command
                    .arg("-q")
                    .arg("-dNOPAUSE")
                    .arg("-dBATCH")
                    .arg("-dSAFER")
                    .arg("-sDEVICE=png16m")
                    .arg("-dUseCropBox")
                    .arg(format!("-r{dpi}"))
                    // Ghostscript's antialiasing is off by default; 4 bits is what its
                    // own documentation recommends for rendering to screen resolution.
                    .arg("-dGraphicsAlphaBits=4")
                    .arg("-dTextAlphaBits=4")
                    .arg(format!("-dFirstPage={page}"))
                    .arg(format!("-dLastPage={page}"));
                if let Some(profile) = substituted_cmyk_profile() {
                    command.arg(format!("-sDefaultCMYKProfile={}", profile.display()));
                }
                command
                    .arg(format!("-sOutputFile={}", output.display()))
                    .arg(pdf);
                command
            }
        }
    }

    /// Reports the renderer's own version string, for inclusion in a report.
    ///
    /// A comparison result is not interpretable without knowing which versions produced
    /// it: reference renderers change their output between releases, so a diff that
    /// appears one day may be an upstream change rather than our regression.
    #[must_use]
    pub fn version(self) -> Option<String> {
        let output = Command::new(self.program())
            .arg(self.version_flag())
            .output()
            .ok()?;
        // `gs` prints to stdout; `pdftoppm` and `mutool` print to stderr.
        let text = if output.stdout.is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };
        String::from_utf8_lossy(text)
            .lines()
            .next()
            .map(str::trim)
            .map(ToOwned::to_owned)
    }
}

/// What a reference renderer said while it was asked to draw one page: both of its output
/// streams, verbatim and in the renderer's own interleaving.
///
/// [`Reference::render_within`] writes them to `<name>.log` beside the image, and
/// [`crate::cache`] stores and restores that file with the picture, so this is available on a
/// cache hit and a miss alike — which it was not until the eight-hundred-and-forty-second
/// session, and which is why the rule below can be part of a verdict at all (ADR 0769).
///
/// **Silence is a reading, not an absence.** A renderer that printed nothing said nothing, and
/// [`Self::is_silent`] is true for that as it is for a log that could not be read. The two are
/// deliberately one case here: nothing this harness does with a log distinguishes "the program
/// was quiet" from "the file is missing", and the safe direction for both is that no testimony
/// was given, so nothing is concluded from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Testimony {
    reference: Reference,
    log: String,
}

impl Testimony {
    /// What `reference` wrote while it was asked to draw a page.
    #[must_use]
    pub fn of(reference: Reference, log: impl Into<String>) -> Self {
        Self {
            reference,
            log: log.into(),
        }
    }

    /// A renderer that said nothing, or whose words were not kept.
    #[must_use]
    pub fn silent(reference: Reference) -> Self {
        Self::of(reference, String::new())
    }

    /// Which renderer gave it.
    #[must_use]
    pub fn reference(&self) -> Reference {
        self.reference
    }

    /// Whether the renderer said nothing at all.
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.log.trim().is_empty()
    }

    /// What the renderer wrote, verbatim.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.log
    }

    /// The sentence in which the renderer says it did not draw what the page asked for.
    ///
    /// `None` where it said nothing of the kind — which includes saying nothing at all, and
    /// includes the far commoner case of a renderer narrating a defect it recovered from. See
    /// [`Reference::refusals`] for what separates the two and on what evidence.
    #[must_use]
    pub fn refusal(&self) -> Option<&str> {
        let refusals = self.reference.refusals();
        self.log
            .lines()
            .map(str::trim)
            .find(|line| refusals.iter().any(|refusal| line.contains(refusal)))
    }
}

/// What a renderer's log says about why it stopped: its first and last non-empty lines.
///
/// **This was the last line alone**, on the reasoning that "these renderers narrate their
/// progress and warn about recoverable damage, so what finally stopped them is at the end" —
/// which is true of the *stopping* and false of the *reason*, and all three of them prove it on
/// the corpus. `mutool` ends with `cannot draw '<path>'` under a first line of
/// `format error: non-page object in page tree`; `gs` ends with `Unrecoverable error, exit code
/// 1` under `Error: /undefined in obj`. Each pair is one sentence naming the clause the file
/// broke and one naming nothing, and the gate was printing the second.
///
/// So both, joined, and only where they differ — a one-line log is one line. It stays two rather
/// than the whole file because a log is unbounded and this string goes on a gate's own line and
/// into `cache`'s remembered failure; the two ends are where a refusal's cause and its
/// consequence sit, and the file is on disk beside the render for anyone who wants the middle.
/// ADR 0574.
fn diagnosis(log: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(log) else {
        return "no diagnostics".to_owned();
    };
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return "no diagnostics".to_owned();
    };
    match lines.next_back() {
        Some(last) if last != first => format!("{first} … {last}"),
        _ => first.to_owned(),
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The CMYK profile `ghostscript` is to be pointed at instead of its own, when a caller is
/// running trap 9's shared-data removal.
///
/// # What this is for, and why it is a knob rather than a hand-run command
///
/// Trap 9's second bullet: `mupdf` and `ghostscript` agree about `DeviceCMYK` because they
/// read **one file** — `/usr/share/ghostscript/iccprofiles/default_cmyk.icc`, which `libgs`
/// reads off the disk and `libmupdf` carries compiled in at the same 187 484 bytes and the
/// same digest. ADR 0048 established that by *evaluating* the profile; what it never did was
/// **take it away**, and a mechanism named is not a mechanism priced (ADR 0497). `gs`'s
/// `-sDefaultCMYKProfile` is the removal: point one member of the pair at a different press
/// and whatever agreement the shared file was manufacturing has to go with it.
///
/// The knob is here rather than in a round's shell script because [`crate::Cache`] keys a
/// remembered render on [`Reference::command_signature`], so an invocation this function
/// changes is a **different key** and cannot be answered out of the baseline's cache — which
/// is the one way an experiment like this silently measures nothing. A hand-run `gs` outside
/// the harness has neither that protection nor the harness's page box, antialiasing and
/// timeout (trap 3).
///
/// Unset — which is every gate, every ratchet and every verdict in this tree — it adds no
/// argument at all, so `gs` uses the profile it would have used and nothing about the
/// baseline depends on this function having been written. ADR 0773.
fn substituted_cmyk_profile() -> Option<PathBuf> {
    let named = std::env::var_os("PDFREF_GS_CMYK_PROFILE")?;
    (!named.is_empty()).then(|| PathBuf::from(named))
}

/// Where a renderer's intermediate files are kept by default.
///
/// Deliberately not a temporary directory that vanishes: on a disagreement these images
/// are the evidence, and deleting them leaves a report nobody can act on.
///
/// `CARGO_TARGET_DIR` is honoured when set, but note that Cargo does not export it to
/// child processes just because `build.target-dir` is configured — so a non-default
/// build directory needs `--work-dir` rather than being detected. Callers that know
/// better (tests, which have `CARGO_TARGET_TMPDIR`) should pass their own path.
#[must_use]
pub fn default_work_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| "target".into()))
        .join("pdfref")
}
