use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::{self, Write};

/// OSC 52 requests the local terminal's clipboard, including over SSH.
/// No acceptance acknowledgment is available. Never emit source text as escapes.
pub fn request(writer: &mut impl Write, markdown: &str) -> io::Result<()> {
    write!(writer, "\x1b]52;c;{}\x1b\\", STANDARD.encode(markdown))?;
    writer.flush()
}
