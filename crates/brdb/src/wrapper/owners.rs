use uuid::Uuid;

use crate::schema::as_brdb::{AsBrdbIter, AsBrdbValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Guid {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

impl From<Guid> for Uuid {
    fn from(value: Guid) -> Self {
        value.uuid()
    }
}
impl From<Uuid> for Guid {
    fn from(value: Uuid) -> Self {
        Guid::from_uuid(value)
    }
}

impl Guid {
    pub fn uuid(self) -> Uuid {
        Uuid::from_u128(
            (self.a as u128) << 96
                | (self.b as u128) << 64
                | (self.c as u128) << 32
                | (self.d as u128),
        )
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        let v = uuid.as_u128();
        Self {
            a: (v >> 96) as u32,
            b: (v >> 64) as u32,
            c: (v >> 32) as u32,
            d: v as u32,
        }
    }
}

impl Default for Guid {
    fn default() -> Self {
        Self {
            a: u32::MAX,
            b: u32::MAX,
            c: u32::MAX,
            d: u32::MAX,
        }
    }
}

impl AsBrdbValue for Guid {
    fn as_brdb_struct_prop_value(
        &self,
        schema: &crate::schema::BrdbSchema,
        _struct_name: crate::schema::BrdbInterned,
        prop_name: crate::schema::BrdbInterned,
    ) -> Result<&dyn AsBrdbValue, crate::errors::BrdbSchemaError> {
        match prop_name.get(schema).unwrap() {
            "A" => Ok(&self.a),
            "B" => Ok(&self.b),
            "C" => Ok(&self.c),
            "D" => Ok(&self.d),
            _ => unreachable!(),
        }
    }
}

pub struct Owner {
    pub user_id: Guid,
    pub user_name: String,
    pub display_name: String,
}

impl Default for Owner {
    fn default() -> Self {
        Self {
            user_id: Guid::default(),
            user_name: "PUBLIC".to_string(),
            display_name: "PUBLIC".to_string(),
        }
    }
}

pub struct OwnerTableSoA {
    pub user_ids: Vec<Guid>,
    pub user_names: Vec<String>,
    pub display_names: Vec<String>,
    pub entity_counts: Vec<u32>,
    pub brick_counts: Vec<u32>,
    pub component_counts: Vec<u32>,
    pub wire_counts: Vec<u32>,
}

impl Default for OwnerTableSoA {
    fn default() -> Self {
        let mut soa = Self {
            user_ids: Vec::new(),
            user_names: Vec::new(),
            display_names: Vec::new(),
            entity_counts: Vec::new(),
            brick_counts: Vec::new(),
            component_counts: Vec::new(),
            wire_counts: Vec::new(),
        };
        soa.add(&Owner::default());
        soa
    }
}

impl OwnerTableSoA {
    pub fn new() -> Self {
        Self {
            user_ids: Vec::new(),
            user_names: Vec::new(),
            display_names: Vec::new(),
            entity_counts: Vec::new(),
            brick_counts: Vec::new(),
            component_counts: Vec::new(),
            wire_counts: Vec::new(),
        }
    }

    pub fn add(&mut self, owner: &Owner) {
        self.user_ids.push(owner.user_id);
        self.user_names.push(owner.user_name.clone());
        self.display_names.push(owner.display_name.clone());
        self.entity_counts.push(0);
        self.brick_counts.push(0);
        self.component_counts.push(0);
        self.wire_counts.push(0);
    }

    pub fn inc_entities(&mut self, index: usize) {
        if let Some(c) = self.entity_counts.get_mut(index) {
            *c += 1;
        }
    }
    pub fn inc_bricks(&mut self, index: usize) {
        if let Some(c) = self.brick_counts.get_mut(index) {
            *c += 1;
        }
    }
    pub fn inc_components(&mut self, index: usize, count: u32) {
        if let Some(c) = self.component_counts.get_mut(index) {
            *c += count;
        }
    }
    pub fn inc_wires(&mut self, index: usize) {
        if let Some(c) = self.wire_counts.get_mut(index) {
            *c += 1;
        }
    }
}

impl AsBrdbValue for OwnerTableSoA {
    fn as_brdb_struct_prop_array(
        &self,
        schema: &crate::schema::BrdbSchema,
        _struct_name: crate::schema::BrdbInterned,
        prop_name: crate::schema::BrdbInterned,
    ) -> Result<crate::schema::as_brdb::BrdbArrayIter, crate::errors::BrdbSchemaError> {
        match prop_name.get(schema).unwrap() {
            "UserIds" => Ok(self.user_ids.as_brdb_iter()),
            "UserNames" => Ok(self.user_names.as_brdb_iter()),
            "DisplayNames" => Ok(self.display_names.as_brdb_iter()),
            "EntityCounts" => Ok(self.entity_counts.as_brdb_iter()),
            "BrickCounts" => Ok(self.brick_counts.as_brdb_iter()),
            "ComponentCounts" => Ok(self.component_counts.as_brdb_iter()),
            "WireCounts" => Ok(self.wire_counts.as_brdb_iter()),
            _ => unreachable!(),
        }
    }
}
