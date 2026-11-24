//! Framed JSON-RPC transport for the client and rust-analyzer subprocess.

use serde_json::Value;
use std::io::{self, BufRead, Write};

/// Reads one Content-Length framed JSON-RPC message.
///
/// # Arguments
///
/// * `reader` — Buffered language-server stream.
///
/// # Returns
///
/// An optional message, with [`None`] indicating a clean end of stream.
///
/// # Errors
///
/// Returns [`io::Error`] for malformed framing, JSON, or interrupted streams.
pub(crate) fn read(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return if length.is_none() {
                Ok(None)
            } else {
                Err(io::ErrorKind::UnexpectedEof.into())
            };
        }
        if line.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':')
            && key.eq_ignore_ascii_case("Content-Length")
        {
            length = Some(value.trim().parse::<usize>().map_err(io::Error::other)?);
        }
    }
    let length = length.ok_or_else(|| io::Error::other("missing Content-Length"))?;
    if length > 64 * 1024 * 1024 {
        return Err(io::Error::other("language-server message exceeds 64 MiB"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}

/// Writes and flushes one framed JSON-RPC message.
///
/// # Arguments
///
/// * `writer` — Destination language-server stream.
/// * `message` — JSON-RPC message.
///
/// # Returns
///
/// An empty result after the message is flushed.
///
/// # Errors
///
/// Returns [`io::Error`] if serialization or transport fails.
pub(crate) fn write(writer: &mut impl Write, message: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    //! Transport regression checks independent of rust-analyzer availability.

    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    /// Verifies consecutive UTF-8 messages retain byte-accurate framing.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Two JSON-RPC messages containing café and an emoji.
    /// ```
    ///
    /// # Assertions
    ///
    /// - Both messages round-trip independently and a clean EOF returns None.
    #[test]
    fn framed_messages_round_trip() {
        let expected = json!({"jsonrpc":"2.0", "id":1, "result":"café 🙂"});
        let mut bytes = Vec::new();
        write(&mut bytes, &expected).unwrap();
        write(&mut bytes, &expected).unwrap();
        let mut reader = Cursor::new(bytes);
        assert_eq!(read(&mut reader).unwrap(), Some(expected.clone()));
        assert_eq!(read(&mut reader).unwrap(), Some(expected));
        assert_eq!(read(&mut reader).unwrap(), None);
    }

    /// Verifies malformed or truncated input cannot be treated as a clean shutdown.
    ///
    /// # Example Under Test
    ///
    /// ```text
    /// Missing length, invalid length, and a body shorter than its declared length.
    /// ```
    ///
    /// # Assertions
    ///
    /// - Every malformed frame returns a transport error.
    #[test]
    fn malformed_frames_fail() {
        for bytes in [
            "\r\n{}",
            "Content-Length: x\r\n\r\n{}",
            "Content-Length: 9\r\n\r\n{}",
        ] {
            assert!(read(&mut Cursor::new(bytes)).is_err());
        }
    }
}
