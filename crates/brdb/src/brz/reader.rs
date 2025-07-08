use std::ops::Deref;

use indexmap::IndexMap;

use crate::{
    BrFsReader, Brz, BrzIndexData,
    errors::BrFsError,
    fs::BrFs,
    tables::{BrBlob, BrFile, BrFolder},
};

pub struct BrzIndex {
    pub archive: Brz,
    /// A map of folder parents and folder names to their indices
    folder_lut: IndexMap<(i32, String), usize>,
    /// A map of file parents and file names to their indices
    files_lut: IndexMap<(i32, String), usize>,
}

impl From<Brz> for BrzIndex {
    fn from(brz: Brz) -> Self {
        Self::new(brz)
    }
}

impl BrzIndex {
    pub fn new(brz: Brz) -> Self {
        let mut folder_lut = IndexMap::new();
        for (i, name) in brz.index_data.folder_names.iter().enumerate() {
            let parent = brz
                .index_data
                .folder_parent_ids
                .get(i)
                .copied()
                .unwrap_or(-1);
            folder_lut.insert((parent, name.clone()), i);
        }

        let mut files_lut = IndexMap::new();
        for (i, name) in brz.index_data.file_names.iter().enumerate() {
            let parent = brz.index_data.file_parent_ids.get(i).copied().unwrap_or(-1);
            files_lut.insert((parent, name.clone()), i);
        }

        Self {
            archive: brz,
            folder_lut,
            files_lut,
        }
    }
}

impl Deref for BrzIndex {
    type Target = Brz;

    fn deref(&self) -> &Self::Target {
        &self.archive
    }
}

fn file_of(index_data: &BrzIndexData, parent: i32, id: i32) -> Result<BrFile, BrFsError> {
    Ok(BrFile {
        name: index_data
            .file_names
            .get(id as usize)
            .ok_or_else(|| BrFsError::NotFound(format!("file name for {id}")))?
            .clone(),
        content_id: index_data
            .file_content_ids
            .get(id as usize)
            .map(|i| (*i > -1).then_some(*i as i64))
            .ok_or_else(|| BrFsError::NotFound(format!("file content id for {id}")))?,
        created_at: 0,
        deleted_at: None,
        file_id: id as i64,
        parent_id: (parent > -1).then_some(parent as i64),
    })
}

fn folder_of(index_data: &BrzIndexData, parent: i32, id: i32) -> Result<BrFolder, BrFsError> {
    Ok(BrFolder {
        name: index_data
            .folder_names
            .get(id as usize)
            .ok_or_else(|| BrFsError::NotFound(format!("folder name for {id}")))?
            .clone(),
        folder_id: id as i64,
        parent_id: (parent > -1).then_some(parent as i64),
        created_at: 0,
        deleted_at: None,
    })
}

fn children_of(
    index_data: &BrzIndexData,
    files_by_parent: &IndexMap<i32, Vec<i32>>,
    folders_by_parent: &IndexMap<i32, Vec<i32>>,
    id: i32,
) -> Result<IndexMap<String, BrFs>, BrFsError> {
    let mut children = IndexMap::default();

    if let Some(folders) = folders_by_parent.get(&id) {
        for &folder_id in folders {
            let folder = folder_of(index_data, id, folder_id)
                .map_err(|e| e.wrap(format!("folder {folder_id}")))?;
            children.insert(
                folder.name.clone(),
                BrFs::Folder(
                    folder,
                    children_of(index_data, files_by_parent, folders_by_parent, folder_id)?,
                ),
            );
        }
    }

    if let Some(files) = files_by_parent.get(&id) {
        for &file_id in files {
            let file =
                file_of(index_data, id, file_id).map_err(|e| e.wrap(format!("file {file_id}")))?;
            children.insert(file.name.clone(), BrFs::File(file));
        }
    }

    Ok(children)
}

impl BrFsReader for BrzIndex {
    fn get_fs(&self) -> Result<BrFs, BrFsError> {
        let mut files_by_parent = IndexMap::<i32, Vec<i32>>::default();
        for i in 0..self.index_data.num_files {
            let parent = self
                .index_data
                .file_parent_ids
                .get(i as usize)
                .copied()
                .unwrap_or(-1);
            files_by_parent.entry(parent).or_default().push(i);
        }

        let mut folders_by_parent = IndexMap::<i32, Vec<i32>>::default();
        for i in 0..self.index_data.num_folders {
            let parent = self
                .index_data
                .folder_parent_ids
                .get(i as usize)
                .copied()
                .unwrap_or(-1);
            folders_by_parent.entry(parent).or_default().push(i);
        }

        Ok(BrFs::Root(children_of(
            &self.index_data,
            &files_by_parent,
            &folders_by_parent,
            -1,
        )?))
    }

    fn find_folder(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError> {
        let parent = parent_id.map(|id| id as i32).unwrap_or(-1);
        if let Some(&index) = self.folder_lut.get(&(parent, name.to_string())) {
            Ok(Some(index as i64))
        } else {
            Ok(None)
        }
    }

    fn find_file(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError> {
        let parent = parent_id.map(|id| id as i32).unwrap_or(-1);
        if let Some(&index) = self.files_lut.get(&(parent, name.to_string())) {
            Ok(Some(index as i64))
        } else {
            Ok(None)
        }
    }

    fn find_blob(&self, blob_id: i64) -> Result<BrBlob, BrFsError> {
        let content_id = blob_id as i32;
        if content_id < 0 || content_id >= self.index_data.num_blobs {
            return Err(BrFsError::NotFound(format!("blob {content_id}")));
        }

        let compression_method = self
            .index_data
            .compression_methods
            .get(content_id as usize)
            .ok_or_else(|| {
                BrFsError::NotFound(format!("compression method for blob {content_id}"))
            })?
            .clone();

        let size_uncompressed = self
            .index_data
            .sizes_uncompressed
            .get(content_id as usize)
            .ok_or_else(|| BrFsError::NotFound(format!("uncompressed size for blob {content_id}")))?
            .clone() as i64;
        let size_compressed = self
            .index_data
            .sizes_compressed
            .get(content_id as usize)
            .ok_or_else(|| BrFsError::NotFound(format!("compressed size for blob {content_id}")))?
            .clone() as i64;
        let hash = self
            .index_data
            .blob_hashes
            .get(content_id as usize)
            .ok_or_else(|| BrFsError::NotFound(format!("hash for blob {content_id}")))?
            .clone();

        let (blob_start_index, blob_end_index) = self
            .index_data
            .blob_ranges
            .get(content_id as usize)
            .ok_or_else(|| BrFsError::NotFound(format!("range for blob {content_id}")))?
            .clone();

        let content = self
            .blob_data
            .get(blob_start_index..blob_end_index)
            .ok_or_else(|| {
                BrFsError::NotFound(format!(
                    "data for blob {content_id} in range [{blob_start_index}, {blob_end_index})"
                ))
            })?
            .to_owned();

        Ok(BrBlob {
            blob_id,
            compression: compression_method as u8 as i64,
            size_uncompressed,
            size_compressed,
            delta_base_id: None,
            hash: hash.to_vec(),
            content,
        })
    }
}
