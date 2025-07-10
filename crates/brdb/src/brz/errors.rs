use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrzError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("invalid magic bytes {0:?}, expected `BRZ`")]
    InvalidMagic([u8; 3]),
    #[error("invalid format version {0}")]
    InvalidFormat(u8),
    #[error("invalid folder count {0}, expected a positive integer")]
    InvalidNumFolders(i32),
    #[error("invalid file count {0}, expected a positive integer")]
    InvalidNumFiles(i32),
    #[error("invalid blob count {0}, expected a positive integer")]
    InvalidNumBlobs(i32),
    #[error("invalid index decompressed length {0}, expected a positive integer")]
    InvalidIndexDecompressedLength(i32),
    #[error("invalid index compressed length {0}, expected a positive integer")]
    InvalidIndexCompressedLength(i32),
    #[error("invalid index data hash {0:?}, expected {1:?}")]
    InvalidIndexHash([u8; 32], [u8; 32]),
    #[error("invalid compression method {0}, expected a valid CompressionMethod")]
    InvalidCompressionMethod(u8),
    #[error("invalid blob decompressed length {0}, expected a positive integer")]
    InvalidBlobDecompressedLength(i32),
    #[error("invalid blob compressed length {0}, expected a positive integer")]
    InvalidBlobCompressedLength(i32),
    #[error(transparent)]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("decompression error: {0}")]
    Decompress(std::io::Error),
}
