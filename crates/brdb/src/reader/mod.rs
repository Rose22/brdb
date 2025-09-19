mod reader_trait;

use indexmap::IndexSet;
pub use reader_trait::BrFsReader;
use std::{
    collections::HashSet,
    ops::Deref,
    sync::{Arc, RwLock},
};

use crate::{
    AsBrdbValue, BString, BrFsError, BrdbComponent, BrickChunkSoA, ChunkIndex, ComponentChunkSoA,
    Entity, EntityChunkIndexSoA, EntityChunkSoA,
    assets::LiteralComponent,
    errors::{BrError, BrdbSchemaError},
    lookup_entity_struct_name,
    pending::BrPendingFs,
    schema::{BrdbSchema, BrdbSchemaGlobalData, BrdbStruct, BrdbValue, ReadBrdbSchema},
    schemas::{BRICK_COMPONENT_SOA, BRICK_WIRE_SOA, ENTITY_CHUNK_SOA},
    wrapper::schemas::{
        BRICK_CHUNK_INDEX_SOA, BRICK_CHUNK_SOA, ENTITY_CHUNK_INDEX_SOA, GLOBAL_DATA_SOA,
        OWNER_TABLE_SOA,
    },
};

pub struct BrReader<T> {
    reader: T,
    global_data: RwLock<Option<Arc<BrdbSchemaGlobalData>>>,
    owners_schema: RwLock<Option<Arc<BrdbSchema>>>,
    components_schema: RwLock<Option<Arc<BrdbSchema>>>,
    wires_schema: RwLock<Option<Arc<BrdbSchema>>>,
    bricks_schema: RwLock<Option<Arc<BrdbSchema>>>,
    brick_chunk_index_schema: RwLock<Option<Arc<BrdbSchema>>>,
    entity_chunk_index_schema: RwLock<Option<Arc<BrdbSchema>>>,
    entities_schema: RwLock<Option<Arc<BrdbSchema>>>,
}
impl<T> Deref for BrReader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

pub trait IntoReader {
    type Inner;
    fn into_reader(self) -> BrReader<Self::Inner>
    where
        Self: Sized;
}

impl<T> IntoReader for T
where
    T: BrFsReader,
{
    type Inner = Self;
    fn into_reader(self) -> BrReader<Self> {
        BrReader::new(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkMeta {
    pub index: ChunkIndex,
    pub num_bricks: u32,
    pub num_wires: u32,
    pub num_components: u32,
}
impl Deref for ChunkMeta {
    type Target = ChunkIndex;

    fn deref(&self) -> &Self::Target {
        &self.index
    }
}
impl AsRef<ChunkIndex> for ChunkMeta {
    fn as_ref(&self) -> &ChunkIndex {
        &self.index
    }
}
impl From<ChunkMeta> for ChunkIndex {
    fn from(value: ChunkMeta) -> Self {
        value.index
    }
}

impl<T> BrReader<T> {
    pub fn new(brdb: T) -> Self
    where
        T: BrFsReader,
    {
        Self {
            reader: brdb,
            global_data: Default::default(),
            owners_schema: Default::default(),
            components_schema: Default::default(),
            wires_schema: Default::default(),
            bricks_schema: Default::default(),
            brick_chunk_index_schema: Default::default(),
            entity_chunk_index_schema: Default::default(),
            entities_schema: Default::default(),
        }
    }

    /// Convert this filesystem to a pending filesystem with all files present
    pub fn to_pending(&self) -> Result<BrPendingFs, BrFsError>
    where
        T: BrFsReader,
    {
        self.reader.get_fs()?.to_pending(&self.reader)
    }

    /// Convert this filesystem to a pending filesystem all files in Patch mode (None for unchanged)
    pub fn to_pending_patch(&self) -> Result<BrPendingFs, BrFsError>
    where
        T: BrFsReader,
    {
        self.reader.get_fs()?.to_pending_patch()
    }

    /// Read the GlobalData
    pub fn read_global_data(&self) -> Result<Arc<BrdbSchemaGlobalData>, BrError>
    where
        T: BrFsReader,
    {
        // Parse the GlobalData schema
        let schema = self
            .read_file("World/0/GlobalData.schema")?
            .as_slice()
            .read_brdb_schema()
            .map_err(|e| e.wrap("Read GlobalData Schema"))?;

        // Parse the GlobalData struct of arrays
        let mps = self
            .read_file("World/0/GlobalData.mps")?
            .as_slice()
            .read_brdb(&schema, GLOBAL_DATA_SOA)
            .map_err(|e| e.wrap("Read BRSavedGlobalDataSoA"))?;

        let mps_struct = mps.as_struct()?;

        let str_set = |prop| {
            mps_struct
                .prop(prop)?
                .as_array()?
                .into_iter()
                .map(|s| Ok(s.as_str()?.to_owned()))
                .collect::<Result<IndexSet<String>, BrdbSchemaError>>()
        };
        let str_vec = |prop| {
            mps_struct
                .prop(prop)?
                .as_array()?
                .into_iter()
                .map(|s| Ok(s.as_str()?.to_owned()))
                .collect::<Result<Vec<String>, BrdbSchemaError>>()
        };

        // Extract the asset names and types from the data
        let mut external_asset_types = HashSet::new();
        let external_asset_references = mps_struct
            .prop("ExternalAssetReferences")?
            .as_array()?
            .into_iter()
            .map(|s| {
                let s = s.as_struct()?;
                let asset_type = s.prop("PrimaryAssetType")?.as_str()?;
                let asset_name = s.prop("PrimaryAssetName")?.as_str()?;
                external_asset_types.insert(asset_type.to_owned());
                Ok((asset_type.to_owned(), asset_name.to_owned()))
            })
            .collect::<Result<IndexSet<_>, BrdbSchemaError>>()?;

        Ok(Arc::new(BrdbSchemaGlobalData {
            external_asset_types,
            external_asset_references,
            entity_type_names: str_set("EntityTypeNames")?,
            basic_brick_asset_names: str_set("BasicBrickAssetNames")?,
            procedural_brick_asset_names: str_set("ProceduralBrickAssetNames")?,
            material_asset_names: str_set("MaterialAssetNames")?,
            component_type_names: str_set("ComponentTypeNames")?,
            component_data_struct_names: str_vec("ComponentDataStructNames")?,
            component_wire_port_names: str_set("ComponentWirePortNames")?,
        }))
    }

    /// Read and cache the GlobalData
    pub fn global_data(&self) -> Result<Arc<BrdbSchemaGlobalData>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(data) = self.global_data.read().unwrap().as_ref() {
            return Ok(data.clone());
        }
        let data = self.read_global_data()?;
        self.global_data.write().unwrap().replace(data.clone());
        Ok(data)
    }
    /// Read the Owners table
    pub fn owners_soa(&self) -> Result<BrdbStruct, BrError>
    where
        T: BrFsReader,
    {
        let owners_schema = self.owners_schema()?;
        let owners_data = self
            .read_file("World/0/Owners.mps")?
            .as_slice()
            .read_brdb(&owners_schema, OWNER_TABLE_SOA)?;
        match owners_data {
            BrdbValue::Struct(s) => Ok(*s),
            ty => Err(BrError::Schema(BrdbSchemaError::ExpectedType(
                "Struct".to_string(),
                ty.get_type().to_owned(),
            ))),
        }
    }
    /// Read the Owners schema
    pub fn owners_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.owners_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Owners.schema")?
            .as_slice()
            .read_brdb_schema()?;
        self.owners_schema.write().unwrap().replace(schema.clone());
        Ok(schema)
    }
    /// Read the shared components chunk schema
    pub fn components_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.components_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Bricks/ComponentsShared.schema")?
            .as_slice()
            .read_brdb_schema_with_data(self.global_data()?)?;
        self.components_schema
            .write()
            .unwrap()
            .replace(schema.clone());
        Ok(schema)
    }

    /// Read the shared component chunk indices
    pub fn component_chunk_soa(
        &self,
        grid_id: usize,
        chunk: ChunkIndex,
    ) -> Result<(ComponentChunkSoA, Vec<BrdbStruct>), BrError>
    where
        T: BrFsReader,
    {
        let global_data = self.global_data()?;
        let schema = self.components_schema()?;

        let path = format!("World/0/Bricks/Grids/{grid_id}/Components/{chunk}.mps");
        let buf = self.read_file(path)?;
        let buf = &mut buf.as_slice();

        let mps = buf.read_brdb(&schema, BRICK_COMPONENT_SOA)?;
        let soa = ComponentChunkSoA::try_from(&mps)
            .map_err(|e| e.wrap(format!("Read component chunk {chunk}")))?;

        let mut component_data = Vec::new();
        for counter in &soa.component_type_counters {
            let type_name = global_data
                .component_type_names
                .get_index(counter.type_index as usize)
                .cloned()
                .unwrap_or("illegal".to_string());
            let struct_name = global_data
                .component_data_struct_names
                .get(counter.type_index as usize)
                .cloned()
                .unwrap_or("illegal".to_string());

            if struct_name == "None" {
                continue;
            }

            for _ in 0..counter.num_instances {
                let BrdbValue::Struct(s) = buf
                    .read_brdb(&schema, &struct_name)
                    .map_err(|e| e.wrap(format!("Read component {type_name}/{struct_name}")))?
                else {
                    continue;
                };
                component_data.push(*s);
            }
        }
        Ok((soa, component_data))
    }

    /// Read the shared component chunk indices
    pub fn component_chunk(
        &self,
        grid_id: usize,
        chunk: ChunkIndex,
    ) -> Result<(ComponentChunkSoA, Vec<BrdbStruct>), BrError>
    where
        T: BrFsReader,
    {
        let global_data = self.global_data()?;
        let schema = self.components_schema()?;

        let path = format!("World/0/Bricks/Grids/{grid_id}/Components/{chunk}.mps");
        let buf = self.read_file(path)?;
        let buf = &mut buf.as_slice();

        let mps = buf.read_brdb(&schema, BRICK_COMPONENT_SOA)?;
        let soa = ComponentChunkSoA::try_from(&mps)
            .map_err(|e| e.wrap(format!("Read component chunk {chunk}")))?;

        let mut component_data = Vec::new();
        for counter in &soa.component_type_counters {
            let type_idx = counter.type_index as usize;
            let type_name = global_data
                .component_type_names
                .get_index(type_idx)
                .cloned()
                .unwrap_or_else(|| "illegal".to_string());
            let struct_name = global_data
                .component_data_struct_names
                .get(type_idx)
                .cloned()
                .unwrap_or_else(|| "illegal".to_string());

            if struct_name == "None" {
                continue;
            }

            for i in 0..counter.num_instances {
                let BrdbValue::Struct(s) = buf
                    .read_brdb(&schema, &struct_name)
                    .map_err(|e| e.wrap(format!("Read component {i} {type_name}/{struct_name}")))?
                else {
                    continue;
                };
                component_data.push(*s);
            }
        }
        Ok((soa, component_data))
    }

    /// Read the shared wires chunk schema
    pub fn wires_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.wires_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Bricks/WiresShared.schema")?
            .as_slice()
            .read_brdb_schema()?;
        self.wires_schema.write().unwrap().replace(schema.clone());
        Ok(schema)
    }
    pub fn wire_chunk_soa(&self, grid_id: usize, chunk: ChunkIndex) -> Result<BrdbStruct, BrError>
    where
        T: BrFsReader,
    {
        let path = format!("World/0/Bricks/Grids/{grid_id}/Wires/{chunk}.mps");
        let mps = self
            .read_file(path)?
            .as_slice()
            .read_brdb(&self.wires_schema()?, BRICK_WIRE_SOA)?;
        match mps {
            BrdbValue::Struct(s) => Ok(*s),
            ty => Err(BrError::Schema(BrdbSchemaError::ExpectedType(
                "Struct".to_string(),
                ty.get_type().to_owned(),
            ))),
        }
    }
    /// Read the shared brick-chunk-index schema
    pub fn brick_chunk_index_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.brick_chunk_index_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Bricks/ChunkIndexShared.schema")?
            .as_slice()
            .read_brdb_schema()?;
        self.brick_chunk_index_schema
            .write()
            .unwrap()
            .replace(schema.clone());
        Ok(schema)
    }
    /// Read the shared bricks chunk schema
    pub fn bricks_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.bricks_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Bricks/ChunksShared.schema")?
            .as_slice()
            .read_brdb_schema()?;
        self.bricks_schema.write().unwrap().replace(schema.clone());
        Ok(schema)
    }
    /// Read the brick chunk indices for a specific grid
    pub fn brick_chunk_index(&self, grid_id: usize) -> Result<Vec<ChunkMeta>, BrError>
    where
        T: BrFsReader,
    {
        let brick_index = self
            .read_file(format!("World/0/Bricks/Grids/{grid_id}/ChunkIndex.mps"))?
            .as_slice()
            .read_brdb(&self.brick_chunk_index_schema()?, BRICK_CHUNK_INDEX_SOA)?;
        let num_bricks = brick_index.prop("NumBricks")?;
        let num_wires = brick_index.prop("NumWires")?;
        let num_components = brick_index.prop("NumComponents")?;
        let chunk_indices = brick_index
            .prop("Chunk3DIndices")?
            .as_array()?
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                Ok(ChunkMeta {
                    index: s.try_into()?,
                    num_bricks: num_bricks.index_unwrap(i)?.as_brdb_u32()?,
                    num_wires: num_wires.index_unwrap(i)?.as_brdb_u32()?,
                    num_components: num_components.index_unwrap(i)?.as_brdb_u32()?,
                })
            })
            .collect::<Result<Vec<_>, BrdbSchemaError>>()?;
        Ok(chunk_indices)
    }
    pub fn brick_chunk_soa(
        &self,
        grid_id: usize,
        chunk: ChunkIndex,
    ) -> Result<BrickChunkSoA, BrError>
    where
        T: BrFsReader,
    {
        let path = format!("World/0/Bricks/Grids/{grid_id}/Chunks/{chunk}.mps");
        let mps = self
            .read_file(path)?
            .as_slice()
            .read_brdb(&self.bricks_schema()?, BRICK_CHUNK_SOA)?;
        Ok((&mps).try_into()?)
    }
    /// Read the shared entity chunk schema
    pub fn entities_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.entities_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Entities/ChunksShared.schema")?
            .as_slice()
            .read_brdb_schema_with_data(self.global_data()?)?;
        self.entities_schema
            .write()
            .unwrap()
            .replace(schema.clone());
        Ok(schema)
    }
    pub fn entities_chunk_index_schema(&self) -> Result<Arc<BrdbSchema>, BrError>
    where
        T: BrFsReader,
    {
        if let Some(schema) = self.entity_chunk_index_schema.read().unwrap().as_ref() {
            return Ok(schema.clone());
        }
        let schema = self
            .read_file("World/0/Entities/ChunkIndex.schema")?
            .as_slice()
            .read_brdb_schema()?;
        self.entity_chunk_index_schema
            .write()
            .unwrap()
            .replace(schema.clone());
        Ok(schema)
    }

    /// Read the entity chunk indices
    pub fn entity_chunk_index(&self) -> Result<Vec<ChunkIndex>, BrError>
    where
        T: BrFsReader,
    {
        let entities_index = self
            .read_file("World/0/Entities/ChunkIndex.mps")?
            .as_slice()
            .read_brdb(&self.entities_chunk_index_schema()?, ENTITY_CHUNK_INDEX_SOA)?;
        Ok(entities_index.prop("Chunk3DIndices")?.try_into()?)
    }

    /// Read the entity chunk indices
    pub fn entity_chunk_index_soa(&self) -> Result<EntityChunkIndexSoA, BrError>
    where
        T: BrFsReader,
    {
        let entities_index = self
            .read_file("World/0/Entities/ChunkIndex.mps")?
            .as_slice()
            .read_brdb(&self.entities_chunk_index_schema()?, ENTITY_CHUNK_INDEX_SOA)?;
        Ok((&entities_index).try_into()?)
    }

    pub fn entity_chunk(&self, chunk: ChunkIndex) -> Result<Vec<Entity>, BrError>
    where
        T: BrFsReader,
    {
        let global_data = self.global_data()?;
        let schema = self.entities_schema()?;
        let path = format!("World/0/Entities/Chunks/{chunk}.mps");
        let buf = self.read_file(path)?;
        let buf = &mut buf.as_slice();
        let illegal = "illegal".to_string();

        let mps = buf
            .read_brdb(&schema, ENTITY_CHUNK_SOA)
            .map_err(|e| e.wrap(format!("Read entity chunk {chunk}")))?;
        let soa = EntityChunkSoA::try_from(&mps)
            .map_err(|e| e.wrap(format!("Read entity chunk {chunk}")))?;

        let mut entity_data = Vec::new();
        let mut index = 0;

        for counter in soa.type_counters {
            let type_name = global_data
                .entity_type_names
                .get_index(counter.type_index as usize)
                .unwrap_or(&illegal);

            let struct_name = lookup_entity_struct_name(type_name);
            for i in 0..counter.num_entities {
                let data: Arc<Box<dyn BrdbComponent>> = if let Some(struct_name) = struct_name {
                    let value = buf.read_brdb(&schema, struct_name).map_err(|e| {
                        e.wrap(format!("Read entity {i} {type_name}/{struct_name}"))
                    })?;
                    let component = LiteralComponent::new_from_data(
                        type_name,
                        struct_name,
                        None,
                        Arc::new(value.as_struct()?.as_hashmap()?),
                        [],
                    );
                    Arc::new(Box::new(component))
                } else {
                    Arc::new(Box::new(()))
                };

                entity_data.push(Entity {
                    asset: BString::from(type_name),
                    id: Some(soa.persistent_indices[index] as usize),
                    owner_index: Some(soa.owner_indices[index]),
                    location: soa.locations[index],
                    rotation: soa.rotations[index],
                    velocity: soa.linear_velocities[index],
                    angular_velocity: soa.angular_velocities[index],
                    color_and_alpha: soa.colors_and_alphas[index].clone(),
                    frozen: soa.physics_locked_flags.get(index),
                    sleeping: soa.physics_sleeping_flags.get(index),
                    data,
                });
                index += 1;
            }
        }
        Ok(entity_data)
    }

    pub fn entity_chunk_soa(
        &self,
        chunk: ChunkIndex,
    ) -> Result<(EntityChunkSoA, Vec<Option<BrdbStruct>>), BrError>
    where
        T: BrFsReader,
    {
        let schema = self.entities_schema()?;
        let path = format!("World/0/Entities/Chunks/{chunk}.mps");
        let buf = self.read_file(path)?;
        let buf = &mut buf.as_slice();
        let mps = buf.read_brdb(&schema, ENTITY_CHUNK_SOA)?;
        let soa = EntityChunkSoA::try_from(&mps)
            .map_err(|e| e.wrap(format!("Read entity chunk {chunk}")))?;
        let global_data = self.global_data()?;
        let illegal = "illegal".to_string();

        let mut entity_data = Vec::new();

        for counter in &soa.type_counters {
            let type_name = global_data
                .entity_type_names
                .get_index(counter.type_index as usize)
                .unwrap_or(&illegal);

            let Some(struct_name) = lookup_entity_struct_name(type_name) else {
                entity_data.push(None);
                continue;
            };
            if struct_name == "None" {
                entity_data.push(None);
                continue;
            }

            for i in 0..counter.num_entities {
                let BrdbValue::Struct(s) = buf
                    .read_brdb(&schema, struct_name)
                    .map_err(|e| e.wrap(format!("Read entity {i} {type_name}/{struct_name}")))?
                else {
                    continue;
                };

                entity_data.push(Some(*s));
            }
        }

        Ok((soa, entity_data))
    }
}
