use brdb::{BrFsReader, Brz, IntoReader};
use std::path::PathBuf;

/// Reads a brz and prints out some of its information
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("./world.brz");

    // The API for reading a brz is identical to reading a Brdb
    let db = Brz::open(path)?.into_reader();

    let data = db.global_data()?;
    println!("Basic Brick assets: {:?}", data.basic_brick_asset_names);
    println!("Wire ports: {:?}", data.component_wire_port_names);
    println!("Component types: {:?}", data.component_type_names);
    println!("Component structs: {:?}", data.component_data_struct_names);
    println!("Component schemas: {}", db.components_schema()?);

    let chunks = db.brick_chunk_index(1)?;
    println!("Brick chunks: {chunks:?}");
    for chunk in chunks {
        let soa = db.brick_chunk_soa(1, chunk.index)?;
        println!("Brick soa: {soa}");
        if chunk.num_components > 0 {
            let (soa, components) = db.component_chunk_soa(1, chunk.index)?;
            println!("Components soa: {soa}");
            for c in components {
                println!("Component: {c}");
            }
        }
        if chunk.num_wires > 0 {
            let soa = db.wire_chunk_soa(1, chunk.index)?;
            println!("Wires soa: {soa}");
        }
    }

    println!("Files: {}", db.get_fs()?.render());

    Ok(())
}
