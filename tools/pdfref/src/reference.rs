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

use pdf_render::Raster;

use crate::{HarnessError, png_io};

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

    /// Renders page one of `pdf` at `dpi`, writing intermediates under `work_dir`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::RendererMissing`] if the executable is absent,
    /// [`HarnessError::RendererFailed`] if it exits non-zero or produces no output, and
    /// [`HarnessError::Png`] if its output cannot be decoded.
    pub fn render(self, pdf: &Path, dpi: u32, work_dir: &Path) -> Result<Raster, HarnessError> {
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
        let mut command = self.build_command(pdf, dpi, work_dir, &output_path);

        let output = command.output().map_err(|e| HarnessError::RendererFailed {
            reference: self,
            detail: format!("could not run {}: {e}", self.program()),
        })?;

        // Ghostscript and mutool both report real problems on stderr while still exiting
        // zero, so success is judged by whether an image appeared, not by exit status
        // alone. The status is still reported when it is non-zero, because that is the
        // more informative failure.
        if !output_path.exists() {
            return Err(HarnessError::RendererFailed {
                reference: self,
                detail: format!(
                    "produced no output (status {:?})\nstderr: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            });
        }

        png_io::read(&output_path)
    }

    /// Builds the command line for this renderer.
    ///
    /// Antialiasing is enabled everywhere it can be. The alternative — turning it off to
    /// make comparison exact — sounds attractive but is wrong: it would compare a
    /// configuration nobody actually views documents in, and would hide precisely the
    /// coverage bugs that matter at edges.
    fn build_command(self, pdf: &Path, dpi: u32, work_dir: &Path, output: &Path) -> Command {
        match self {
            Self::Poppler => {
                // `-singlefile` suppresses the page-number suffix, giving a predictable
                // output path instead of `prefix-1.png`.
                let prefix = work_dir.join(self.name());
                let mut command = Command::new(self.program());
                command
                    .arg("-r")
                    .arg(dpi.to_string())
                    .arg("-png")
                    .arg("-singlefile")
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
                    .arg("-r")
                    .arg(dpi.to_string())
                    .arg("-o")
                    .arg(output)
                    .arg(pdf)
                    .arg("1");
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
                    .arg(format!("-r{dpi}"))
                    // Ghostscript's antialiasing is off by default; 4 bits is what its
                    // own documentation recommends for rendering to screen resolution.
                    .arg("-dGraphicsAlphaBits=4")
                    .arg("-dTextAlphaBits=4")
                    .arg("-dFirstPage=1")
                    .arg("-dLastPage=1")
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
