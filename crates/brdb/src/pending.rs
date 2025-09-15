use std::{collections::HashMap, fmt::Display};

use crate::{
    BrFsError,
    errors::BrError,
    wrapper::{
        UnsavedFs,
        schemas::{
            self, BRICK_CHUNK_INDEX_SOA, BRICK_CHUNK_SOA, BRICK_WIRE_SOA, ENTITY_CHUNK_INDEX_SOA,
            GLOBAL_DATA_SOA, OWNER_TABLE_SOA,
        },
    },
};

/// Describes an entire filesystem tree that needs to be written
/// Any `None` values indicate unchanged files or folders
/// Any absent entries will be deleted
/// All files will be hashed and checked for existing blobs
/// Any overwritten files will be marked as deleted
///
/// A revision will be created along with all of the pending
#[derive(Debug)]
pub enum BrPendingFs {
    Root(Vec<(String, BrPendingFs)>),
    Folder(Option<Vec<(String, BrPendingFs)>>),
    File(Option<Vec<u8>>),
}

// Helper trait for adding context to errors
trait Wrap<T> {
    fn about(self, name: impl Display) -> Result<T, BrError>;
    fn about_f(self, name: impl FnMut() -> String) -> Result<T, BrError>;
}
impl<T, E> Wrap<T> for Result<T, E>
where
    BrError: From<E>,
{
    fn about(self, name: impl Display) -> Result<T, BrError> {
        self.map_err(|e| BrError::from(e).wrap(name))
    }
    fn about_f(self, mut name: impl FnMut() -> String) -> Result<T, BrError> {
        self.map_err(|e| BrError::from(e).wrap(name()))
    }
}

impl BrPendingFs {
    pub fn from_unsaved(fs: UnsavedFs) -> Result<Self, BrError> {
        use BrPendingFs::*;
        let mut worlds = vec![];

        let global_data_schema = schemas::global_data_schema();
        let owners_schema = schemas::owners_schema();
        let brick_chunk_index_schema = schemas::bricks_chunk_index_schema();
        let brick_chunk_schema = schemas::bricks_chunks_schema();
        let wires_schema = schemas::bricks_wires_schema();
        let entity_chunk_index_schema = schemas::entities_chunk_index_schema();

        for (world_id, world) in fs.worlds {
            // This index needs to exist because the type ids of brick assets are
            // stored in the GlobalData, and the type ids of procedural
            // bricks are assigned starting from the end of the basic brick
            // asset names.
            //
            // When new brick assets are added, the length of the basic
            // brick asset names will increase, and the type ids of procedural
            // bricks in older chunks will not match the new
            // basic brick asset names.
            //
            // This offset allows older chunks to properly load, assuming the global
            // data does not change the order of brick asset names.
            let proc_brick_starting_index = world.global_data.basic_brick_asset_names.len() as u32;

            let mut world_dir = vec![
                // Write GlobalData
                (
                    "GlobalData.schema".to_owned(),
                    File(Some(
                        global_data_schema.to_bytes().about("GlobalData.schema")?,
                    )),
                ),
                (
                    "GlobalData.mps".to_owned(),
                    File(Some(
                        global_data_schema
                            .write_brdb(GLOBAL_DATA_SOA, &world.global_data)
                            .about("GlobalData.mps")?,
                    )),
                ),
                // Write Owners
                (
                    "Owners.schema".to_owned(),
                    File(Some(owners_schema.to_bytes().about("Owners.schema")?)),
                ),
                (
                    "Owners.mps".to_owned(),
                    File(Some(
                        owners_schema
                            .write_brdb(OWNER_TABLE_SOA, &world.owners)
                            .about("Owners.mps")?,
                    )),
                ),
            ];

            if let Some(_env) = world.environment.as_ref() {
                // TODO: Write Environment.bp
            }
            if let Some(_minigame) = world.minigame.as_ref() {
                // TODO: Write Minigame.bp
            }

            let mut bricks_dir = vec![
                // Shared schemas
                (
                    "ChunkIndexShared.schema".to_owned(),
                    File(Some(
                        brick_chunk_index_schema
                            .to_bytes()
                            .about("ChunkIndexShared.schema")?,
                    )),
                ),
                (
                    "ChunksShared.schema".to_owned(),
                    File(Some(
                        brick_chunk_schema.to_bytes().about("ChunksShared.schema")?,
                    )),
                ),
                (
                    "WiresShared.schema".to_owned(),
                    File(Some(wires_schema.to_bytes().about("WiresShared.schema")?)),
                ),
                // Component schema
                (
                    "ComponentsShared.schema".to_owned(),
                    File(Some(
                        world
                            .component_schema
                            .to_bytes()
                            .about("ComponentsShared.schema")?,
                    )),
                ),
            ];
            let mut grids_dir = vec![];

            // Bricks/Grids/N/Chunks
            // Bricks/Grids/N/Components
            // Bricks/Grids/N/Wires
            // Bricks/Grids/N/ChunkIndex.mps
            for (grid_id, grid) in world.grids {
                let mut grid_dir = vec![(
                    "ChunkIndex.mps".to_owned(),
                    File(Some(
                        brick_chunk_index_schema
                            .write_brdb(BRICK_CHUNK_INDEX_SOA, &grid.chunk_index)
                            .about_f(|| format!("Grids/{grid_id}/ChunkIndex.mps"))?,
                    )),
                )];

                let brick_chunks_dir = grid
                    .bricks
                    .into_iter()
                    .map(|(chunk, mut bricks)| {
                        bricks.procedural_brick_starting_index = proc_brick_starting_index;
                        Ok((
                            format!("{chunk}.mps"),
                            File(Some(
                                brick_chunk_schema
                                    .write_brdb(BRICK_CHUNK_SOA, &bricks)
                                    .about_f(|| format!("Grids/{grid_id}/Chunks/{chunk}.mps"))?,
                            )),
                        ))
                    })
                    .collect::<Result<Vec<_>, BrError>>()?;
                let component_chunks_dir = grid
                    .components
                    .into_iter()
                    .map(|(chunk, components)| {
                        let buf = components
                            .to_bytes(&world.component_schema)
                            .about_f(|| format!("Grids/{grid_id}/Components/{chunk}.mps"))?;

                        Ok((format!("{chunk}.mps"), File(Some(buf))))
                    })
                    .collect::<Result<Vec<_>, BrError>>()?;
                let wire_chunks_dir = grid
                    .wires
                    .iter()
                    .map(|(chunk, wires)| {
                        Ok((
                            format!("{chunk}.mps"),
                            File(Some(
                                wires_schema
                                    .write_brdb(BRICK_WIRE_SOA, wires)
                                    .about_f(|| format!("Grids/{grid_id}/Wires/{chunk}.mps"))?,
                            )),
                        ))
                    })
                    .collect::<Result<Vec<_>, BrError>>()?;

                // Append non-empty chunk directories to the grid directory
                if !brick_chunks_dir.is_empty() {
                    grid_dir.push(("Chunks".to_owned(), Folder(Some(brick_chunks_dir))));
                }
                if !component_chunks_dir.is_empty() {
                    grid_dir.push(("Components".to_owned(), Folder(Some(component_chunks_dir))));
                }
                if !wire_chunks_dir.is_empty() {
                    grid_dir.push(("Wires".to_owned(), Folder(Some(wire_chunks_dir))));
                }
                grids_dir.push((grid_id.to_string(), Folder(Some(grid_dir))));
            }

            let mut entities_dir = vec![
                (
                    "ChunkIndex.schema".to_owned(),
                    File(Some(
                        entity_chunk_index_schema
                            .to_bytes()
                            .about("ChunkIndex.schema")?,
                    )),
                ),
                (
                    "ChunkIndex.mps".to_owned(),
                    File(Some(
                        entity_chunk_index_schema
                            .write_brdb(ENTITY_CHUNK_INDEX_SOA, &world.entity_chunk_index)
                            .about("ChunkIndex.mps")?,
                    )),
                ),
                (
                    "ChunksShared.schema".to_owned(),
                    File(Some(
                        world
                            .entity_schema
                            .to_bytes()
                            .about("ChunksShared.schema")?,
                    )),
                ),
            ];

            // Entities/Chunks/*
            let entities_chunks_dir = world
                .entity_chunks
                .into_iter()
                .map(|(chunk, entities)| {
                    let buf = entities
                        .to_bytes(&world.entity_schema)
                        .about_f(|| format!("Entities/Chunks/{chunk}.mps"))?;

                    Ok((format!("{chunk}.mps"), File(Some(buf))))
                })
                .collect::<Result<Vec<_>, BrError>>()?;

            // Only add the Chunks directory if there are any chunks
            if !entities_chunks_dir.is_empty() {
                entities_dir.push(("Chunks".to_owned(), Folder(Some(entities_chunks_dir))));
            }
            bricks_dir.push(("Grids".to_owned(), Folder(Some(grids_dir))));
            world_dir.push(("Bricks".to_owned(), Folder(Some(bricks_dir))));
            world_dir.push(("Entities".to_owned(), Folder(Some(entities_dir))));
            worlds.push((world_id.to_string(), Folder(Some(world_dir))));
        }

        let meta_dir = (
            "Meta".to_owned(),
            Folder(Some(vec![
                (
                    "Bundle.json".to_owned(),
                    File(Some(
                        serde_json::to_vec(&fs.meta.bundle).about("Bundle.json")?,
                    )),
                ),
                (
                    "Screenshot.jpg".to_owned(),
                    File(fs.meta.screenshot.clone()),
                ),
                (
                    "World.json".to_owned(),
                    File(Some(
                        serde_json::to_vec(&fs.meta.world).about("World.json")?,
                    )),
                ),
            ])),
        );

        let world_dir = ("World".to_owned(), Folder(Some(worlds)));

        Ok(Root(vec![meta_dir, world_dir]))
    }

    #[cfg(feature = "brz")]
    /// Convert this pending FS into a BRZ archive
    pub fn to_brz_data(self, zstd_level: Option<i32>) -> Result<crate::brz::Brz, BrError> {
        use std::collections::{HashMap, VecDeque};

        use crate::{
            brz::{Brz, BrzIndexData, CompressionMethod},
            compression::compress,
            errors::BrFsError,
        };

        let mut queue = VecDeque::new();
        queue.push_front((None, "Root".to_owned(), self));

        let mut index = BrzIndexData::default();
        let mut blob_data = Vec::new();
        let mut hash_to_blob_index: HashMap<[u8; 32], i32> = HashMap::new();

        while let Some((parent_id, name, fs)) = queue.pop_front() {
            match fs {
                BrPendingFs::Root(items) => {
                    for (name, item) in items {
                        queue.push_back((None, name, item));
                    }
                }

                // Insert the folder, then all of its children
                BrPendingFs::Folder(Some(items)) => {
                    let folder_id = index.num_folders;
                    // Add this folder
                    index.num_folders += 1;
                    index.folder_parent_ids.push(parent_id.unwrap_or(-1));
                    index.folder_names.push(name.clone());

                    // Queue the folder's children
                    for (item_name, item_fs) in items {
                        queue.push_back((Some(folder_id), item_name, item_fs));
                    }
                }

                // Insert the file, and its content if it was not already inserted
                BrPendingFs::File(Some(content)) => {
                    use crate::tables::BrBlob;

                    index.num_files += 1;
                    index.file_parent_ids.push(parent_id.unwrap_or(-1));
                    index.file_names.push(name.clone());
                    let hash = BrBlob::hash(&content);

                    let content_id = if content.is_empty() {
                        -1
                    } else if let Some(i) = hash_to_blob_index.get(&hash) {
                        *i
                    } else {
                        let blob_id = index.num_blobs;
                        index.num_blobs += 1;

                        hash_to_blob_index.insert(hash.clone(), blob_id);
                        index.blob_hashes.push(hash);
                        index.sizes_uncompressed.push(content.len() as i32);

                        let start = blob_data.len();

                        // Compress the content if a zstd level is specified
                        if let Some(zstd_level) = zstd_level {
                            let compressed =
                                compress(&content, zstd_level).map_err(BrFsError::Compress)?;

                            if compressed.len() < content.len() {
                                index.sizes_compressed.push(compressed.len() as i32);
                                index
                                    .compression_methods
                                    .push(CompressionMethod::GenericZstd);
                                // Update the blob ranges with compressed size
                                blob_data.extend_from_slice(&compressed);
                            } else {
                                // If the compressed size is larger than the uncompressed size,
                                // store it as uncompressed
                                index.sizes_compressed.push(content.len() as i32);
                                index.compression_methods.push(CompressionMethod::None);
                                // Update blob ranges with uncompressed size
                                blob_data.extend_from_slice(&content);
                            }
                        } else {
                            index.sizes_compressed.push(content.len() as i32);
                            index
                                .compression_methods
                                .push(crate::brz::CompressionMethod::None);
                            blob_data.extend_from_slice(&content);
                        }

                        index.blob_ranges.push((start, blob_data.len()));
                        blob_id
                    };

                    index.file_content_ids.push(content_id)
                }
                BrPendingFs::File(None) | BrPendingFs::Folder(None) => {
                    // Noop - these files are ignored.
                }
            }
        }
        index.blob_total_size = blob_data.len();

        Ok(Brz {
            index_data: index,
            blob_data,
        })
    }

    // Get the children of the root if this is a root
    pub fn to_root(self) -> Option<Vec<(String, BrPendingFs)>> {
        match self {
            BrPendingFs::Root(items) => Some(items),
            _ => None,
        }
    }

    // Get the children of this folder if this is a folder
    pub fn to_folder(self) -> Option<Vec<(String, BrPendingFs)>> {
        match self {
            BrPendingFs::Folder(items) => items,
            _ => None,
        }
    }

    // Get the file content of this pending FS this is a file
    pub fn to_file(self) -> Option<Vec<u8>> {
        match self {
            BrPendingFs::File(items) => items,
            _ => None,
        }
    }

    /// Apply a PendingFs as a patch to this FS
    pub fn with_patch(mut self, patch: BrPendingFs) -> Result<Self, BrFsError> {
        self.patch(patch)?;
        Ok(self)
    }

    /// Apply a PendingFs as a patch to this FS
    pub fn patch(&mut self, patch: BrPendingFs) -> Result<(), BrFsError> {
        match (self, patch) {
            // Root and folders apply patches to existing children and insert new files
            (BrPendingFs::Folder(Some(children)), BrPendingFs::Folder(Some(patch)))
            | (BrPendingFs::Root(children), BrPendingFs::Root(patch)) => {
                let mut patch_map = patch.into_iter().collect::<HashMap<_, _>>();
                // Patch anything that already exists in the folder
                for (name, fs) in children.iter_mut() {
                    let Some(patch) = patch_map.remove(name) else {
                        continue;
                    };
                    fs.patch(patch)?;
                }
                // Append the other patches
                children.extend(patch_map);
            }
            // If the folder is empty, insert the patched folder
            (BrPendingFs::Folder(source @ None), BrPendingFs::Folder(Some(patch))) => {
                *source = Some(patch);
            }
            (BrPendingFs::File(data), BrPendingFs::File(Some(patch))) => {
                *data = Some(patch);
            }
            (BrPendingFs::Folder(_), BrPendingFs::Folder(None))
            | (BrPendingFs::File(_), BrPendingFs::File(None)) => {
                // None means no changes for this patch
            }
            (left, right) => {
                return Err(BrFsError::InvalidStructure(
                    left.to_string(),
                    right.to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Display for BrPendingFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrPendingFs::Root(items) => write!(
                f,
                "[{}]",
                items
                    .iter()
                    .map(|(n, i)| format!("{n} {i}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            BrPendingFs::Folder(items) => write!(
                f,
                "[{}]",
                items
                    .as_ref()
                    .map(|v| v
                        .iter()
                        .map(|(n, i)| format!("{n} {i}"))
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_else(|| "empty".to_string())
            ),
            BrPendingFs::File(content) => write!(
                f,
                "({})",
                content
                    .as_ref()
                    .map(|v| v.len().to_string())
                    .unwrap_or_default()
            ),
        }
    }
}
