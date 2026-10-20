//! This machine, as the program's world: the files, the terminal, the streams and the outputs.
//!
//! What the in-process route hands the program, and what the broker answers a confined worker's
//! requests from — so a request the fences admit is carried out by exactly the code the program
//! would have run itself, and its bytes are the same (ADR 2140).

use std::io::{BufRead as _, IsTerminal as _};
use std::path::Path;

use pdf_transform::Sinks;
use pdf_transform::confined::{Reading, World};
use pdf_transform::tool::{ToolRequest, ToolResult};

use super::{FileSinks, StdoutSinks};

/// This machine.
#[derive(Debug, Default)]
pub(super) struct Here;

#[expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the program's own standard output and error, whose entire output is a report"
)]
impl World for Here {
    fn read(&mut self, path: &Path, how: Reading) -> std::io::Result<Vec<u8>> {
        match how {
            Reading::Bytes => std::fs::read(path),
            Reading::Whole => pdf_syntax::read_file(path),
        }
    }

    fn document(&mut self, path: &Path) -> std::io::Result<pdf_syntax::FileBytes> {
        // On disk rather than read whole: ADR 0809's window, so that a merge of six-gigabyte
        // inputs costs each one's trailer, table and selected pages rather than its bytes.
        pdf_syntax::FileBytes::on_disk(path)
    }

    fn password(&mut self, descriptor: u32) -> std::io::Result<String> {
        let file = std::fs::File::open(format!("/dev/fd/{descriptor}"))?;
        let mut line = String::new();
        std::io::BufReader::new(file).read_line(&mut line)?;
        Ok(line)
    }

    fn is_terminal(&mut self) -> bool {
        std::io::stdin().is_terminal()
    }

    fn answer(&mut self) -> std::io::Result<String> {
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        Ok(answer)
    }

    fn print(&mut self, text: &str) {
        print!("{text}");
    }

    fn say(&mut self, text: &str) {
        eprint!("{text}");
    }

    fn sinks(&self, to_stdout: bool) -> Box<dyn Sinks> {
        if to_stdout {
            Box::new(StdoutSinks::default())
        } else {
            Box::new(FileSinks)
        }
    }

    fn execute(&mut self, request: &ToolRequest) -> Result<ToolResult, String> {
        pdf_transform::executor::execute(request).map_err(|error| error.to_string())
    }

    fn read_beside(&mut self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}
