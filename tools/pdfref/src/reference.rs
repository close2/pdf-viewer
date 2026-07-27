//! The external PDF renderers we compare against.
//!
//! Three independent implementations, deliberately chosen because they share no
//! rendering code: `poppler` (descended from xpdf), `mupdf`, and `ghostscript`. Their
//! *agreement* is the evidence the harness relies on — see [`crate::triangulate`].
//!
//! `pdfium` is a worthwhile fourth, being what Chrome ships and therefore the de facto
//! standard, but it is not packaged in the main Arch repositories.

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
}

impl Reference {
    /// Every reference the harness knows how to drive.
    pub const ALL: [Self; 3] = [Self::Poppler, Self::MuPdf, Self::Ghostscript];

    /// Short name used in reports and artefact filenames.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Poppler => "poppler",
            Self::MuPdf => "mupdf",
            Self::Ghostscript => "ghostscript",
        }
    }

    /// The executable this reference needs.
    #[must_use]
    pub fn program(self) -> &'static str {
        match self {
            Self::Poppler => "pdftoppm",
            Self::MuPdf => "mutool",
            Self::Ghostscript => "gs",
        }
    }

    /// Package providing this renderer, for error messages that are actionable.
    #[must_use]
    pub fn package_hint(self) -> &'static str {
        match self {
            Self::Poppler => "poppler (Arch) / poppler-utils (Debian)",
            Self::MuPdf => "mupdf-tools",
            Self::Ghostscript => "ghostscript",
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
        }
    }

    /// Returns `true` if the executable is on `PATH`.
    #[must_use]
    pub fn is_available(self) -> bool {
        // Spawning is the test, not the exit status: `Command::status` fails only when
        // the program cannot be run at all. `mutool` with no subcommand exits non-zero
        // while being perfectly present, so judging by status would report it missing.
        Command::new(self.program())
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

        let log_path = work_dir.join(format!("{}.log", self.name()));
        if let Ok(log) = std::fs::File::create(&log_path) {
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::from(log));
        }

        let status = self.wait_within(&mut command, budget)?;

        // Ghostscript and mutool both report real problems on stderr while still exiting
        // zero, so success is judged by whether an image appeared, not by exit status
        // alone. The status is still reported when it is non-zero, because that is the
        // more informative failure.
        if !output_path.exists() {
            return Err(HarnessError::RendererFailed {
                reference: self,
                detail: format!(
                    "produced no output (status {:?}): {}",
                    status.code(),
                    last_line(&log_path)
                ),
            });
        }

        png_io::read(&output_path)
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
                return Err(HarnessError::RendererFailed {
                    reference: self,
                    detail: format!("exceeded {budget:?} and was killed"),
                });
            }
            std::thread::sleep(POLL);
        }
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
    /// ISO 32000-2 §7.7.3.3 defines `/CropBox` as "the region to which the contents of the
    /// page shall be clipped (cropped) when displayed or printed", and that is what a
    /// viewer shows. `mutool draw` does this by default; `pdftoppm` and `gs` default to the
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
            Self::Poppler => {
                let prefix = work_dir.join(self.name());
                let mut command = Command::new(self.program());
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
                let mut command = Command::new(self.program());
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
                let mut command = Command::new(self.program());
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
                    .arg(format!("-dLastPage={page}"))
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

/// The last non-empty line of a renderer's log, for an error message that says why.
///
/// The last rather than the first: these renderers narrate their progress and warn about
/// recoverable damage, so what finally stopped them is at the end.
fn last_line(log: &Path) -> String {
    std::fs::read_to_string(log).map_or_else(
        |_| "no diagnostics".to_owned(),
        |text| {
            text.lines()
                .rev()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("no diagnostics")
                .to_owned()
        },
    )
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
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
