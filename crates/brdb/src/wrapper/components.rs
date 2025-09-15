use crate::{
    BrdbSchemaError,
    schema::{
        BrdbInterned, BrdbSchema, BrdbSchemaGlobalData, BrdbSchemaMeta, BrdbStruct, BrdbValue,
        as_brdb::{AsBrdbIter, AsBrdbValue},
        write::write_brdb,
    },
    schemas::BRICK_COMPONENT_SOA,
    wrapper::{BString, Quat4f, Vector3f},
};

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct ComponentTypeCounter {
    pub type_index: u32,
    pub num_instances: u32,
}

impl AsBrdbValue for ComponentTypeCounter {
    fn as_brdb_struct_prop_value(
        &self,
        schema: &BrdbSchema,
        _struct_name: BrdbInterned,
        prop_name: BrdbInterned,
    ) -> Result<&dyn AsBrdbValue, crate::errors::BrdbSchemaError> {
        match prop_name.get(schema).unwrap() {
            "TypeIndex" => Ok(&self.type_index),
            "NumInstances" => Ok(&self.num_instances),
            _ => unreachable!(),
        }
    }
}

impl TryFrom<&BrdbValue> for ComponentTypeCounter {
    type Error = BrdbSchemaError;

    fn try_from(value: &BrdbValue) -> Result<Self, Self::Error> {
        Ok(Self {
            type_index: value.prop("TypeIndex")?.as_brdb_u32()?,
            num_instances: value.prop("NumInstances")?.as_brdb_u32()?,
        })
    }
}

#[derive(Default)]
pub struct ComponentChunkSoA {
    pub component_type_counters: Vec<ComponentTypeCounter>,
    pub component_brick_indices: Vec<u32>,
    pub joint_brick_indices: Vec<u32>,
    pub joint_entity_references: Vec<u32>,
    pub joint_initial_relative_offsets: Vec<Vector3f>,
    pub joint_initial_relative_rotations: Vec<Quat4f>,

    // A copy of all components that need to be written.
    // The `BrdbComponent` trait is writable
    pub unwritten_struct_data: Vec<Box<dyn BrdbComponent>>,
}

impl ComponentChunkSoA {
    pub fn add_component(
        &mut self,
        global_data: &BrdbSchemaGlobalData,
        brick_index: u32,
        component: &dyn BrdbComponent,
    ) {
        let Some((component_ty_name, struct_ty)) = component.get_schema_struct() else {
            // Cannot add component without a type
            return;
        };
        // Unwrap safety: The component type was already added to the global data before
        // this function was called.
        let type_index = global_data
            .component_type_names
            .get_index_of(component_ty_name.as_ref())
            .unwrap() as u32;

        // Check if the last counter matches the type index
        if let Some(counter) = self.component_type_counters.last_mut()
            && counter.type_index == type_index
        {
            counter.num_instances += 1;
        } else {
            // No counters yet, add the first one
            self.component_type_counters.push(ComponentTypeCounter {
                type_index,
                num_instances: 1,
            });
        }
        // Track the brick index for this component
        self.component_brick_indices.push(brick_index);

        // Clone the component data into unwritten_struct_data to be written later
        // Only if the component has a struct type
        if struct_ty.is_some() {
            self.unwritten_struct_data.push(component.boxed_component());
        }
    }

    pub fn to_bytes(self, schema: &BrdbSchema) -> Result<Vec<u8>, BrdbSchemaError> {
        let mut buf = schema.write_brdb(BRICK_COMPONENT_SOA, &self)?;

        for (i, component_data) in self.unwritten_struct_data.into_iter().enumerate() {
            // Unwrap safety: The component can only be added to unwritten_struct_data if
            // get_schema_struct() returns Some(_, Some(_))
            let Some((_, Some(struct_ty))) = component_data.get_schema_struct() else {
                // Cannot write entity data without a type
                continue;
            };

            // Append to the buffer and serialize the component's data
            write_brdb(
                &schema,
                &mut buf,
                struct_ty.as_ref(),
                component_data.as_ref(),
            )
            .map_err(|e| e.wrap(format!("component data {i}: {struct_ty}")))?;
        }
        Ok(buf)
    }
}

impl AsBrdbValue for ComponentChunkSoA {
    fn as_brdb_struct_prop_array(
        &self,
        schema: &BrdbSchema,
        _struct_name: BrdbInterned,
        prop_name: BrdbInterned,
    ) -> Result<crate::schema::as_brdb::BrdbArrayIter, crate::errors::BrdbSchemaError> {
        Ok(match prop_name.get(schema).unwrap() {
            "ComponentTypeCounters" => self.component_type_counters.as_brdb_iter(),
            "ComponentBrickIndices" => self.component_brick_indices.as_brdb_iter(),
            "JointBrickIndices" => self.joint_brick_indices.as_brdb_iter(),
            "JointEntityReferences" => self.joint_entity_references.as_brdb_iter(),
            "JointInitialRelativeOffsets" => self.joint_initial_relative_offsets.as_brdb_iter(),
            "JointInitialRelativeRotations" => self.joint_initial_relative_rotations.as_brdb_iter(),
            _ => unreachable!(),
        })
    }
}

impl TryFrom<&BrdbValue> for ComponentChunkSoA {
    type Error = BrdbSchemaError;

    fn try_from(value: &BrdbValue) -> Result<Self, Self::Error> {
        Ok(Self {
            component_type_counters: value.prop("ComponentTypeCounters")?.try_into()?,
            component_brick_indices: value.prop("ComponentBrickIndices")?.try_into()?,
            joint_brick_indices: value.prop("JointBrickIndices")?.try_into()?,
            joint_entity_references: value.prop("JointEntityReferences")?.try_into()?,
            joint_initial_relative_offsets: value
                .prop("JointInitialRelativeOffsets")?
                .try_into()?,
            joint_initial_relative_rotations: value
                .prop("JointInitialRelativeRotations")?
                .try_into()?,
            unwritten_struct_data: Vec::new(),
        })
    }
}

/// This trait allows BrdbComponents to be cloned
/// despite being a dyn trait
pub trait BoxedComponent {
    fn boxed_component(&self) -> Box<dyn BrdbComponent>;
}

pub trait BrdbComponent: AsBrdbValue + BoxedComponent {
    /// Emit the structs needed to use this component in a world
    fn get_schema(&self) -> Option<BrdbSchemaMeta> {
        None
    }

    /// Emit asset references this component uses
    fn get_external_asset_references(&self) -> Vec<(BString, BString)> {
        Default::default()
    }

    /// Emit the "ComponentTypeName" and "ComponentDataStructName" pair for this
    /// component
    fn get_schema_struct(&self) -> Option<(BString, Option<BString>)> {
        None
    }

    /// Emit a list of wire ports this component uses
    fn get_wire_ports(&self) -> Vec<BString> {
        Default::default()
    }
}

/// Blanket implement boxed for all BrdbComponents with Clone
/// ... enabling them to be cloned
impl<T: Clone + BrdbComponent + 'static> BoxedComponent for T {
    fn boxed_component(&self) -> Box<dyn BrdbComponent> {
        Box::new(self.clone())
    }
}

// Empty component... may have its usecases
impl BrdbComponent for () {}

// This may be a footgun when crafting brdbs from nothing but it's helpful for editing brdb files in place
impl BrdbComponent for BrdbStruct {
    fn get_schema(&self) -> Option<BrdbSchemaMeta> {
        None
    }

    fn get_external_asset_references(&self) -> Vec<(BString, BString)> {
        Vec::new()
    }

    fn get_schema_struct(&self) -> Option<(BString, Option<BString>)> {
        Some((BString::Static(""), Some(self.get_name().to_owned().into())))
    }

    fn get_wire_ports(&self) -> Vec<BString> {
        Vec::new()
    }
}
