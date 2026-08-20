//! The command line, and the settings that have to be applied before a document exists.
//!
//! Everything here runs before the window: which driver stack to ask for, whether there is to be
//! a graphics device at all, where the sandbox stands, and what the argument after the flags
//! actually names. The order matters more than the parsing does — a policy applied halfway
//! through is not a policy — which is why the whole of it is one function a reader can follow
//! from the top rather than a table of handlers.

use std::path::PathBuf;

use render_quorra::QuorraWindowRenderer;
use viewer_core::RestrictionLevel;

use crate::trace::{Trace, parse_topics, speak_up, topic_names};

/// A driver stack `--backend` can name: what talks to the GPU, not which GPU.
///
/// **The distinction is the whole reason this exists.** One GPU is enumerated once per backend
/// that can drive it, under the *device's* name each time, so a name filter selects hardware and
/// cannot express "this card, through DX12". The set of backends is an instance-level choice and
/// the instance is made before anything else, which is why this is decided on the command line
/// and carried to `QuorraWindowRenderer::instance_with` (quorra's ADR 0017; ours is 0221).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
    /// Vulkan: Linux's and Android's, and one of Windows' two.
    Vulkan,
    /// Direct3D 12, Windows only.
    Dx12,
    /// Metal, macOS and iOS only.
    Metal,
    /// OpenGL / OpenGL ES, everywhere and last.
    Gl,
}

impl Backend {
    /// Every value `--backend` accepts, in the order the usage message lists them.
    ///
    /// Two of wgpu's backends are deliberately absent. `BROWSER_WEBGPU` needs a `wasm32` target,
    /// which this program has none of; `NOOP` is compiled only under a wgpu feature this build
    /// does not enable and refuses to initialise without a second opt-in, so naming it would
    /// offer a choice that cannot be honoured — which is the trap `Device::adapter_names_on`
    /// exists to close one level down.
    const ALL: [Self; 4] = [Self::Vulkan, Self::Dx12, Self::Metal, Self::Gl];

    /// What a person types after `--backend`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Vulkan => "vulkan",
            Self::Dx12 => "dx12",
            Self::Metal => "metal",
            Self::Gl => "gl",
        }
    }

    /// The value `--backend` names, or `None` for a word that is not one of them.
    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|backend| backend.name() == word)
    }

    /// The one-backend set this restricts an instance to.
    fn backends(self) -> quorra_gpu::wgpu::Backends {
        match self {
            Self::Vulkan => quorra_gpu::wgpu::Backends::VULKAN,
            Self::Dx12 => quorra_gpu::wgpu::Backends::DX12,
            Self::Metal => quorra_gpu::wgpu::Backends::METAL,
            Self::Gl => quorra_gpu::wgpu::Backends::GL,
        }
    }
}

/// The backend this program asks for when nobody said, on Windows: **DX12**.
///
/// A choice this project now makes, where it used to be wgpu's hub order making it — which puts
/// Vulkan ahead of DX12 among adapters of equal rank, and is how the project owner's machine
/// reached an Intel Vulkan driver that crashed it. The argument is in ADR 0221, and the honest
/// part of it is that **no machine in this project runs Windows**, so this default is reasoned
/// rather than measured. `--backend vulkan` overrides it, and a machine that has no DX12 adapter
/// falls back to every backend with a note rather than refusing to start.
#[cfg(windows)]
pub(crate) const DEFAULT_BACKEND: Option<Backend> = Some(Backend::Dx12);

/// No default anywhere else: one driver stack is the norm, and where there are two the platform's
/// own ranking is the one this project has evidence for. See the Windows arm above.
#[cfg(not(windows))]
pub(crate) const DEFAULT_BACKEND: Option<Backend> = None;

/// What the command line asked for.
pub(crate) struct Arguments {
    /// The document to open.
    pub(crate) path: PathBuf,
    /// What to say about what is happening, from `--trace` and `--trace=<topics>`.
    pub(crate) trace: Trace,
    /// Whether to draw with `render-cpu` rather than the graphics device, from `--cpu`.
    ///
    /// **And therefore whether a graphics device is created at all**, since the
    /// three-hundred-and-eighty-fourth session: this flag now decides the presenter as well as
    /// the rasteriser, so a run that asks for the processor opens no driver. See ADR 0221.
    pub(crate) processor: bool,
    /// The driver stack `--backend` named, or [`DEFAULT_BACKEND`] where it did not.
    pub(crate) backend: Option<Backend>,
    /// Whether [`Arguments::backend`] is a person's answer or this program's default, which is
    /// what decides between a refusal and a fallback when no adapter matches it.
    pub(crate) backend_asked_for: bool,
    /// The page `--page` named, counting from one.
    pub(crate) opens_at: Option<usize>,
    /// Annex O's fragment identifier, where the argument carried one after a `#`.
    pub(crate) fragment: Option<String>,
    /// What this reader does with the restrictions a document asserts, from
    /// `--ignore-restrictions`.
    ///
    /// **Not a user interface for them**, which `CLAUDE.md` says is not to be built yet: it is
    /// the one policy value `viewer-core` asks for, supplied the way this host supplies every
    /// other one it has — the sandbox, the backend, the page to open at. The four levels the
    /// project owner named, and the menu that will offer them, are later.
    pub(crate) restrictions: RestrictionLevel,
    /// How many whole pages the window retains a low-resolution picture of, from
    /// `--proxy-pages`, defaulting to [`crate::stale::PROXY_PAGES`].
    ///
    /// **The project owner asked for the extent to be configurable and rule 2 decided where**
    /// (`doc/todo/37`, ADR 0443). The pictures are stand-ins — deliberately wrong pictures — so
    /// everything that makes one lives in a private module of this binary, and the count may not
    /// become a `Command`, a field of a boundary type or anything a gate could link to. So it is
    /// the host's, exactly as `--cpu`, `--backend` and `--no-sandbox` are.
    pub(crate) proxy_pages: usize,
}

/// Reads the command line, applies the two settings that must be applied before anything opens a
/// document, and exits where it cannot.
///
/// Separate from `main` because the sandbox decision is one of them: it decides *where* this
/// document's images are decoded, and a policy applied halfway through is not a policy.
pub(crate) fn arguments(began: std::time::Instant) -> Arguments {
    let mut path = None;
    let mut sandbox = true;
    let mut trace = Trace::off(began);
    let mut processor = false;
    let mut backend = DEFAULT_BACKEND;
    let mut backend_asked_for = false;
    let mut opens_at = None;
    let mut restrictions = RestrictionLevel::On;
    let mut proxy_pages = crate::stale::PROXY_PAGES;
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--licences" || argument == "--licenses" {
            print!("{}", crate::NOTICE);
            std::process::exit(0);
        } else if argument == "--no-sandbox" {
            sandbox = false;
        } else if argument == "--trace" || argument.to_string_lossy().starts_with("--trace=") {
            // `--trace=<topics>` rather than `--trace <topics>`, because the flag has taken no
            // value for a hundred sessions and a document is what follows it: `--trace doc.pdf`
            // must keep meaning what it always meant, and only the equals form can promise that.
            let list = argument.to_string_lossy();
            let list = list.split_once('=').map_or("", |(_, rest)| rest);
            match parse_topics(list) {
                Ok(topics) => trace.topics = topics,
                Err(word) => {
                    eprintln!(
                        "--trace={word}: not a topic. One of: {}, or all — each optionally \
                         prefixed with - to leave it out.",
                        topic_names()
                    );
                    std::process::exit(2);
                }
            }
            // The graphics stack's own voice, which is silent until something receives it.
            if trace.any() {
                speak_up();
            }
        } else if argument == "--cpu" {
            processor = true;
        } else if argument == "--backend" {
            // Refused here rather than carried as a string and refused later: a word that names
            // no backend is a typing mistake, and the list of what would have worked is a better
            // answer than a launch that quietly ignored the flag.
            let Some(word) = arguments.next() else {
                eprintln!("--backend wants one of: {}", backend_names());
                std::process::exit(2);
            };
            let Some(named) = Backend::parse(&word.to_string_lossy()) else {
                eprintln!(
                    "--backend {}: not a backend. One of: {}",
                    word.to_string_lossy(),
                    backend_names()
                );
                std::process::exit(2);
            };
            backend = Some(named);
            backend_asked_for = true;
        } else if argument == "--proxy-pages" {
            proxy_pages = retained_pages(arguments.next());
        } else if argument == "--ignore-restrictions" {
            restrictions = RestrictionLevel::Off;
        } else if argument == "--page" {
            // A page number as the title bar shows it, which is one-based. §12.3.2.1's
            // `/OpenAction` is the document's own answer to the same question and wins where
            // this is absent; where both are stated, the person asking now wins.
            opens_at = arguments
                .next()
                .and_then(|value| value.to_string_lossy().parse::<usize>().ok())
                .filter(|page| *page > 0);
            if opens_at.is_none() {
                eprintln!("--page wants a page number, counting from 1");
                std::process::exit(2);
            }
        } else if path.is_none() {
            path = Some(argument);
        } else {
            eprintln!("unexpected argument: {}", argument.to_string_lossy());
            std::process::exit(2);
        }
    }
    let Some(argument) = path else {
        usage();
        std::process::exit(2);
    };
    let (path, fragment) = split_fragment(&argument);

    say_what_this_build_cannot_do();

    if !sandbox {
        pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
        // Said out loud, once, on the way past. Turning the sandbox off is a reasonable choice
        // for documents you produced yourself and a bad one for documents that arrived by
        // email, and the difference is not visible from inside the program.
        println!(
            "note: --no-sandbox — JBIG2 and JPEG 2000 will be decoded in this process, with no \
             memory ceiling, and a decoder failure will take the viewer down with it"
        );
    }

    Arguments {
        path,
        trace,
        processor,
        backend,
        backend_asked_for,
        opens_at,
        fragment,
        restrictions,
        proxy_pages,
    }
}

/// How many pages `--proxy-pages` asked the window to retain a low-resolution picture of.
///
/// **Refused rather than defaulted, for the reason `--backend` is**: a number this program could
/// not read is a typing mistake, and a launch that quietly ignored it would leave a person
/// measuring the default while believing they had changed it — which for a knob whose whole purpose
/// is to be measured against its own absence is the worst failure it could have.
fn retained_pages(word: Option<std::ffi::OsString>) -> usize {
    let read = word.and_then(|value| value.to_string_lossy().parse::<usize>().ok());
    let Some(pages) = read else {
        eprintln!(
            "--proxy-pages wants a count of pages, 0 or more (default {})",
            crate::stale::PROXY_PAGES
        );
        std::process::exit(2);
    };
    pages
}

/// What this *build* cannot do, said before anything is opened.
///
/// Both are facts about the executable rather than about a document, and a person choosing a viewer
/// for untrusted files deserves them in the first line rather than in a release note. Linux has
/// seccomp-BPF and Landlock; the other two platforms get the worker process and no kernel
/// confinement, which is a decision with an argument (ADR 0194) rather than an omission. AT-SPI is
/// Linux's too, and AccessKit's other two adapters are not wired in here — a build that quietly
/// exposed nothing would look exactly like one whose bridge is broken (ADR 0214).
///
/// A function of its own rather than two blocks inside the command line, because neither of them
/// reads an argument: they are what this executable says about itself, whatever it was asked for.
fn say_what_this_build_cannot_do() {
    if !pdf_sandbox::lockdown::ENFORCED_BY_THIS_BUILD {
        println!(
            "note: this build has no kernel confinement for the image decoder — seccomp-BPF \
             and Landlock are Linux interfaces. JBIG2 and JPEG 2000 are still decoded in a \
             separate process, so a decoder failure costs one image rather than the viewer, \
             and there is no address-space ceiling on that process."
        );
    }
    if let Some(missing) = viewer_accessibility::Bridge::shortfall() {
        println!("note: {missing}");
    }
}

/// The thread that creates the graphics instance, or `None` where this run will not have one.
///
/// **A function rather than three lines in `main`, so that the promise `--cpu` makes has a
/// test.** Creating a `wgpu::Instance` *is* loading the driver — it is where quorra measured
/// roughly 80% of what bring-up blocks for, and it is where the project owner's machine crashed —
/// so a flag that means "no graphics device" has to mean this thread is not spawned. The
/// difference between a flag that chooses a rasteriser and a flag that avoids a driver is exactly
/// the `None` below, and `cpu_creates_no_graphics_instance` is what keeps it there.
pub(crate) fn spawn_instancing(
    processor: bool,
    backend: Option<Backend>,
) -> Option<std::thread::JoinHandle<quorra_gpu::wgpu::Instance>> {
    (!processor).then(|| {
        std::thread::spawn(move || match backend {
            Some(named) => QuorraWindowRenderer::instance_with(named.backends()),
            None => QuorraWindowRenderer::instance(),
        })
    })
}

/// The backends `--backend` accepts on this build, for a message that has to list them.
pub(crate) fn backend_names() -> String {
    Backend::ALL
        .iter()
        .map(|backend| backend.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Splits `document.pdf#page=5` into the file and ISO 32000-2 Annex O's fragment identifier.
///
/// **The filesystem decides, not the punctuation**, and that is this host's choice rather than
/// anything the annex says. A `#` is an ordinary character in a file name on every system this
/// program runs on, so an argument that names an existing file is taken whole; only when it does
/// not is it read as a URI-shaped reference and split at its first `#`, which is where RFC 3986
/// puts the boundary. The cost is one `stat` on the launch path and a file called `a#b.pdf` that
/// still opens; the alternative — splitting first — makes that file unopenable and says nothing.
///
/// `viewer-core` never sees this decision: what crosses is the fragment alone, undecoded, because
/// splitting a URI is the host's job and percent-decoding belongs to whoever knows which component
/// it is decoding.
fn split_fragment(argument: &std::ffi::OsStr) -> (PathBuf, Option<String>) {
    let whole = PathBuf::from(argument);
    if whole.exists() {
        return (whole, None);
    }
    let text = argument.to_string_lossy();
    match text.split_once('#') {
        Some((path, fragment)) if !path.is_empty() => {
            (PathBuf::from(path), Some(fragment.to_owned()))
        }
        // No `#`, or nothing before it. Hand the whole thing on and let the read fail by name:
        // a path that does not exist is a better message than a fragment nobody asked for.
        _ => (whole, None),
    }
}

/// What the program does when it is given nothing to open.
fn usage() {
    eprintln!("usage: pdf-viewer [--no-sandbox] <document.pdf>");
    eprintln!("       pdf-viewer --licences");
    eprintln!();
    eprintln!("Arrows, Page Up/Down or Space turn pages; Home and End jump; + and - zoom;");
    eprintln!("o shows the sidebar — the outline, the layers and the embedded files;");
    eprintln!("? shows the third-party notices; drag to select text, a selects the page,");
    eprintln!("s saves, Escape quits.");
    eprintln!();
    eprintln!("  --no-sandbox  decode JBIG2 and JPEG 2000 images in this process rather than");
    eprintln!("                in a confined worker. Faster by a process spawn and a pipe");
    eprintln!("                round trip; appropriate only for documents you trust.");
    eprintln!("  --page N      open at page N, counting from 1 as the title bar does.");
    eprintln!("  doc.pdf#...   ISO 32000-2 Annex O's fragment identifier, which says where to");
    eprintln!("                open: page=5, nameddest=Chapter3, zoom=150,0,792, view=FitH,700,");
    eprintln!("                viewrect=..., comment=..., structelem=.... Parameters are");
    eprintln!("                separated by & and carried out left to right; whatever this");
    eprintln!("                program cannot do is named rather than ignored.");
    eprintln!("  --cpu         draw with the processor rather than the graphics device, and open");
    eprintln!("                no graphics driver at all: no instance, no adapter, no device. The");
    eprintln!("                page reaches the window through a software surface instead.");
    eprintln!("                Slower, and the same rasteriser the reference comparison is built");
    eprintln!("                on: a page that appears with this and not without it is the");
    eprintln!("                device's, and so is a launch that only works with it.");
    eprintln!("  --proxy-pages N");
    eprintln!("                how many whole pages the window keeps a low-resolution picture of,");
    eprintln!("                so that a view change reaching area the last frame has no pixels");
    eprintln!("                for — a zoom out, a scroll, a page turn — shows something rather");
    eprintln!("                than the window's background. 0 turns it off; each page costs one");
    eprintln!("                render on the idle render thread and under a megabyte.");
    eprintln!("  --backend B   which driver stack talks to the GPU, not which GPU: vulkan, dx12,");
    eprintln!("                metal or gl. What to reach for when one stack on this machine is");
    eprintln!("                broken and another is not. Refused, rather than quietly ignored,");
    eprintln!("                where this machine has no adapter behind the one named.");
    eprintln!("  --trace       print every command, every event and every frame, each line");
    eprintln!("                stamped with the seconds since this process started, and");
    eprintln!("                whatever the graphics stack has to say. What to run when a page");
    eprintln!("                will not appear: the last line printed is the step that did not");
    eprintln!("                finish. A frame's line names its stages — host, scene, device,");
    eprintln!("                settle — and the percentiles over every frame are printed on the");
    eprintln!("                way out. PDFVIEWER_LOG=error|warn|info|debug sets how much of");
    eprintln!(
        "                the graphics stack's own logging comes with it, defaulting to warn."
    );
    eprintln!("  --trace=T,U   only the topics named, of: launch, frames, events, window,");
    eprintln!("                pointer, access, selection — or all. A topic prefixed with -");
    eprintln!("                is left out, and a list that starts with one means everything");
    eprintln!("                else: --trace=frames to chase a slow page, --trace=-pointer for");
    eprintln!("                everything but the flood a moving mouse makes.");
    eprintln!("  --ignore-restrictions");
    eprintln!("                perform an operation a document says its reader may not — filling");
    eprintln!("                in a field under §7.6.4.2's permission flags or an author's");
    eprintln!("                §12.8.2.2 certification. The default is to obey and say so.");
    eprintln!("  --licences    print the third-party notices this binary carries, and exit.");
}

#[cfg(test)]
mod tests {
    use render_quorra::QuorraWindowRenderer;

    use super::{Backend, DEFAULT_BACKEND, backend_names, spawn_instancing};

    /// **The whole of what `--cpu` promises.** A run on the processor creates no
    /// `wgpu::Instance`, which is what loads the driver: before the
    /// three-hundred-and-eighty-fourth session the flag chose which rasteriser drew the page
    /// and the driver was loaded regardless, so a driver that faulted while loading took the
    /// run down whether or not the flag was given. That is the defect the project owner hit on
    /// Windows, and this test is what stops it coming back.
    ///
    /// A backend named alongside `--cpu` changes nothing: there is nothing to name a backend
    /// *for*.
    #[test]
    fn cpu_creates_no_graphics_instance() {
        assert!(spawn_instancing(true, None).is_none());
        assert!(spawn_instancing(true, Some(Backend::Vulkan)).is_none());
        assert!(spawn_instancing(true, Some(Backend::Dx12)).is_none());
    }

    /// And the control, without which the test above passes on a function that never spawns:
    /// a run that did *not* ask for the processor makes one, and it is a real instance.
    #[test]
    fn without_cpu_an_instance_is_created() {
        let thread = spawn_instancing(false, None).expect("a run without --cpu creates one");
        let instance = thread.join().expect("the thread creating the instance");
        // Enumerating proves the instance is usable rather than merely constructed. The list
        // may be empty on a machine with no adapter at all, which is not what is being asked.
        let _ = QuorraWindowRenderer::adapters_on(&instance);
    }

    /// Every value `--backend` accepts is parsed by the name it prints, and nothing else is.
    #[test]
    fn a_backend_is_named_by_the_word_it_prints() {
        for backend in Backend::ALL {
            assert_eq!(Backend::parse(backend.name()), Some(backend));
        }
        assert_eq!(Backend::parse("Vulkan"), None, "the match is exact");
        assert_eq!(Backend::parse("webgpu"), None, "not a value on this build");
        assert_eq!(Backend::parse(""), None);
        assert_eq!(backend_names(), "vulkan, dx12, metal, gl");
    }

    /// The platform default, stated as a test because it is a decision rather than a detail:
    /// on Windows this project asks for DX12 first, and everywhere else it asks for nothing and
    /// leaves the choice where it was. ADR 0221 argues it; no machine in this project can run
    /// the Windows arm, which is why the arm is written down here as well as there.
    #[test]
    fn the_default_backend_is_dx12_on_windows_and_nothing_elsewhere() {
        #[cfg(windows)]
        assert_eq!(DEFAULT_BACKEND, Some(Backend::Dx12));
        #[cfg(not(windows))]
        assert_eq!(DEFAULT_BACKEND, None);
    }
}
