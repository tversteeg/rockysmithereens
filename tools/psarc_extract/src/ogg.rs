use anyhow::Result;
use rodio_wem::WemParser;

const SEGMENT_SIZE: usize = 255;
const MAX_SEGMENTS: usize = 255;

/// Convert the wem input file to ogg/vorbis.
pub fn convert_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();

    // Parse the wem file.
    let parser = WemParser::new(bytes)?;

    // Write the headers
    flush(&parser.ident_header, &mut out, false)?;
    flush(&parser.comment_header, &mut out, false)?;
    flush(&parser.setup_header, &mut out, false)?;

    // Write the packets
    for (index, packet) in parser.packets.iter().enumerate() {
        let is_last = parser.packets.len() == index + 1;
        flush(&packet.data, &mut out, is_last)?;
    }

    Ok(out)
}

/// Flush bytes into the ogg stream.
fn flush(bytes: &[u8], ogg: &mut Vec<u8>, is_last: bool) -> Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }

    let first = if ogg.is_empty() { 2u8 } else { 0u8 };
    // If there's nothing in the out buffer this is the first
    let last = if is_last { 4u8 } else { 0u8 };

    // Calculate required segments
    let mut segments = (bytes.len() + SEGMENT_SIZE) / SEGMENT_SIZE;
    if segments == MAX_SEGMENTS + 1 {
        segments = MAX_SEGMENTS;
    }

    // Write header
    ogg.extend("OggS\x00".as_bytes());
    ogg.push(first | last);

    todo!();
}
