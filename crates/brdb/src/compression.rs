use std::io::{Read, Write};

/// Compress the given content using zstd with the specified level.
pub fn compress(content: &[u8], zstd_level: i32) -> Result<Vec<u8>, std::io::Error> {
    let mut compressed = vec![];
    let mut enc = zstd::Encoder::new(&mut compressed, zstd_level)?;
    enc.write_all(content)?;
    enc.do_finish()?;
    Ok(compressed)
}

pub fn decompress(content: &[u8], size_uncompressed: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut output = vec![0u8; size_uncompressed];
    zstd::Decoder::new(content)?.read_exact(&mut output)?;
    Ok(output)
}
