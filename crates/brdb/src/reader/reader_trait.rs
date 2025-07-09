use std::fmt::Display;

use crate::{errors::BrFsError, fs::BrFs, tables::BrBlob};

pub trait BrFsReader {
    /// Find and read a file from the brdb filesystem, returning its decompressed content as a byte vector.
    fn read_file(&self, path: impl Display) -> Result<Vec<u8>, BrFsError> {
        let path = path.to_string();

        if path.starts_with("/") {
            return Err(BrFsError::AbsolutePathNotAllowed);
        }

        let mut components = path.split("/").peekable();
        let mut entire_path = String::from("");
        let mut parent_id = None;
        let mut content_id = 0;

        while let Some(name) = components.next() {
            entire_path.push('/');
            entire_path.push_str(name);

            // If there is more in the path, the current component must be a folder
            if components.peek().is_some() {
                let Some(next) = self
                    .find_folder(parent_id, name)
                    .map_err(|e| e.wrap(format!("find folder {entire_path}")))?
                else {
                    return Err(BrFsError::NotFound(format!("folder {entire_path}")));
                };
                parent_id = Some(next);
                continue;
            }

            // Find the file in the current folder
            content_id = self
                .find_file(parent_id, name)
                .map_err(|e| e.wrap(format!("find file {entire_path}")))?
                .ok_or_else(|| BrFsError::NotFound(format!("file {entire_path}")))?;
            break;
        }

        // Read the blob
        Ok(self
            .find_blob(content_id)
            .map_err(|e| e.wrap(format!("find blob {content_id}")))?
            .read()
            .map_err(|e| e.wrap(format!("read blob {content_id}")))?)
    }

    /// Find a file by its name and parent folder id in the brdb filesystem, returning its folder_id
    fn find_folder(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError>;

    /// Find a file by its name and parent in the brdb filesystem, returns the blob_id if found.
    fn find_file(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError>;

    /// Read the metadata for a file in the brdb filesystem.
    fn find_blob(&self, content_id: i64) -> Result<BrBlob, BrFsError>;

    /// Get the filesystem representation of the BRDB database.
    fn get_fs(&self) -> Result<BrFs, BrFsError>;
}

impl<T: BrFsReader> BrFsReader for &T {
    fn find_folder(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError> {
        (*self).find_folder(parent_id, name)
    }

    fn find_file(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError> {
        (*self).find_file(parent_id, name)
    }

    fn find_blob(&self, content_id: i64) -> Result<BrBlob, BrFsError> {
        (*self).find_blob(content_id)
    }

    fn get_fs(&self) -> Result<BrFs, BrFsError> {
        (*self).get_fs()
    }
}

impl BrFsReader for () {
    fn find_folder(&self, _parent_id: Option<i64>, _name: &str) -> Result<Option<i64>, BrFsError> {
        unimplemented!()
    }

    fn find_file(&self, _parent_id: Option<i64>, _name: &str) -> Result<Option<i64>, BrFsError> {
        unimplemented!()
    }

    fn find_blob(&self, _content_id: i64) -> Result<BrBlob, BrFsError> {
        unimplemented!()
    }

    fn get_fs(&self) -> Result<BrFs, BrFsError> {
        unimplemented!()
    }
}
