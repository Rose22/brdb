use brdb::{AsBrdbValue, Brdb, IntoReader, WireChunkSoA, schema::BrdbStruct};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
};
struct ComponentMeta {
    wire_inputs: HashSet<u16>,
    wire_outputs: HashSet<u16>,
}

struct BrickMeta {
    type_index: u32,
    components: Vec<(u16, BrdbStruct)>,
}

/// Reads a world and prints out some of its information
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("./world.brdb");

    let db = Brdb::open(path)?.into_reader();

    let data = db.global_data()?;
    let component_schema = db.components_schema()?;

    let chunks = db.brick_chunk_index(1)?;

    // Track seen brick types
    let mut brick_type_set = HashSet::new();

    // Track seen component types to map to their wire ports
    let mut component_map = HashMap::new();

    // Track brick -> component mappings
    let mut brick_map = HashMap::new();

    for chunk in &chunks {
        let soa = db.brick_chunk_soa(1, chunk.index)?;

        // Iterate basic bricks
        let pb_index = soa.procedural_brick_starting_index;
        for (i, t) in soa.brick_type_indices.into_iter().enumerate() {
            if t >= pb_index {
                continue;
            }
            if brick_type_set.contains(&t) {
                continue;
            }
            brick_type_set.insert(t);

            // Insert bricks of unique types
            brick_map.insert(
                (chunk.index, i),
                BrickMeta {
                    type_index: t,
                    components: Vec::new(),
                },
            );
        }

        if chunk.num_components > 0 {
            let (soa, components) = db.component_chunk_soa(1, chunk.index)?;
            let indices = soa.component_brick_indices;

            // Expand the type index/num instances into a flat list of type indices
            let type_indices = soa
                .component_type_counters
                .iter()
                .flat_map(|v| {
                    let index = v.type_index as u16;
                    (0..v.num_instances).map(move |_| index)
                })
                .collect::<Vec<_>>();

            // Add each component and its type to the brick map
            for (i, c) in components.iter().enumerate() {
                let brick_index = indices[i as usize].as_brdb_u32()?;
                let type_index = type_indices[i as usize];
                if let Some(brick) = brick_map.get_mut(&(chunk.index, brick_index as usize)) {
                    brick.components.push((type_index, c.clone()));
                } else {
                    continue;
                }

                // Register the component type if not already registered
                if !component_map.contains_key(&type_index) {
                    component_map.insert(
                        type_index,
                        ComponentMeta {
                            wire_inputs: HashSet::new(),
                            wire_outputs: HashSet::new(),
                        },
                    );
                }
            }
        }
    }

    // Add the wire ports to the component map
    for chunk in &chunks {
        if chunk.num_wires > 0 {
            let soa = db.wire_chunk_soa(1, chunk.index)?.to_value();
            let soa: WireChunkSoA = (&soa).try_into()?;
            for port in soa.local_wire_sources {
                if let Some(meta) = component_map.get_mut(&port.component_type_index) {
                    meta.wire_outputs.insert(port.port_index);
                }
            }
            for port in soa.local_wire_targets {
                if let Some(meta) = component_map.get_mut(&port.component_type_index) {
                    meta.wire_inputs.insert(port.port_index);
                }
            }
            for port in soa.remote_wire_sources {
                if let Some(meta) = component_map.get_mut(&port.component_type_index) {
                    meta.wire_outputs.insert(port.port_index);
                }
            }
            for port in soa.remote_wire_targets {
                if let Some(meta) = component_map.get_mut(&port.component_type_index) {
                    meta.wire_inputs.insert(port.port_index);
                }
            }
        }
    }

    // Print the brick -> component mappings
    for meta in brick_map.values() {
        let brick_type_str = data.basic_brick_asset_names[meta.type_index as usize].clone();
        for c in &meta.components {
            let component_type_str = data.component_type_names[c.0 as usize].clone();
            let c_entry = component_map.get(&c.0).unwrap();
            let wire_inputs = c_entry
                .wire_inputs
                .iter()
                .map(|i| {
                    format!(
                        "  {}",
                        data.component_wire_port_names[*i as usize].to_owned()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let wire_outputs = c_entry
                .wire_outputs
                .iter()
                .map(|i| {
                    format!(
                        "  {}",
                        data.component_wire_port_names[*i as usize].to_owned()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            let mut component_struct = String::new();
            for (name, properties) in &component_schema.structs {
                if name != &c.1.name {
                    continue;
                }

                let name = component_schema
                    .intern
                    .lookup(*name)
                    .unwrap_or("UnknownStruct".to_owned());
                writeln!(component_struct, "struct {name} {{")?;
                for (prop_name, prop_type) in properties {
                    let prop_name = component_schema
                        .intern
                        .lookup(*prop_name)
                        .unwrap_or("UnknownProperty".to_owned());
                    writeln!(
                        component_struct,
                        "    {prop_name}: {},",
                        prop_type.as_string(&component_schema)
                    )?;
                }
                writeln!(component_struct, "}}")?;
            }

            println!(
                "Brick: {}\nComponent: {}\n{}Inputs:\n{}\nOutputs:\n{}\n\n",
                brick_type_str, component_type_str, component_struct, wire_inputs, wire_outputs
            );
        }
    }

    Ok(())
}
