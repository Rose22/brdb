use crate::{compression::decompress, errors::BrFsError};

#[derive(Clone, Debug)]
pub struct BrBlob {
    pub blob_id: i64,
    pub compression: i64,
    pub size_uncompressed: i64,
    pub size_compressed: i64,
    pub delta_base_id: Option<i64>, // always null
    pub hash: Vec<u8>,
    pub content: Vec<u8>,
}

impl BrBlob {
    /// Get the BLAKE3 hash of the given content.
    pub fn hash(content: &[u8]) -> [u8; 32] {
        *blake3::hash(content).as_bytes()
    }

    /// Read (and decompress) the content of a blob in the brdb filesystem.
    pub fn read(self) -> Result<Vec<u8>, BrFsError> {
        let content = if self.compression == 0 {
            self.content
        } else {
            // Ensure blob compressed content length is correct
            if self.content.len() != self.size_compressed as usize {
                return Err(BrFsError::InvalidSize {
                    name: "compressed content".to_string(),
                    found: self.content.len(),
                    expected: self.size_compressed as usize,
                });
            }

            // Decompress the content
            decompress(&self.content, self.size_uncompressed as usize)
                .map_err(BrFsError::Decompress)?
        };

        // Verify the size of the decompressed content
        if content.len() != self.size_uncompressed as usize {
            return Err(BrFsError::InvalidSize {
                name: "uncompressed content".to_string(),
                found: content.len(),
                expected: self.size_uncompressed as usize,
            });
        }

        let hash = Self::hash(&content);

        // Verify the hash of the decompressed content
        if hash != self.hash.as_slice() {
            return Err(BrFsError::InvalidHash {
                found: hash.to_vec(),
                expected: self.hash,
            });
        }

        Ok(content)
    }
}

#[derive(Default, Clone, Debug)]
pub struct BrRevision {
    pub revision_id: i64,
    pub description: String,
    pub created_at: i64,
}

#[derive(Default, Clone, Debug)]
pub struct BrFolder {
    pub folder_id: i64,
    pub parent_id: Option<i64>, // references folder_id
    pub name: String,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Default, Clone, Debug)]

pub struct BrFile {
    pub file_id: i64,
    pub parent_id: Option<i64>, // references folders(folder_id),
    pub name: String,
    pub content_id: Option<i64>,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
}
