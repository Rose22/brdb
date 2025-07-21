use brdb::{Brdb, IntoReader, OwnerTableSoA};
use serde::Serialize;
use std::{
    env,
    path::{self, PathBuf},
    process,
};

#[derive(Serialize)]
struct Row {
    user_id: String,
    user_name: String,
    display_name: String,
    entity_counts: u32,
    brick_counts: u32,
    component_counts: u32,
    wire_counts: u32,
}

/// Opens a world and prints out the counts
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().into_iter();
    args.next();
    let Some(file) = args.next() else {
        eprintln!("missing world file arg");
        process::exit(1);
    };

    let dst = env::current_dir()?.join(PathBuf::from(&file));
    if !dst.exists() {
        eprintln!("file {} does not exist", dst.display());
        process::exit(1);
    }

    eprintln!("Opening world: {}", path::absolute(&dst)?.display());

    let db = Brdb::open(dst)?.into_reader();

    let owners_soa = db.owners_soa()?.to_value();
    let owners = OwnerTableSoA::try_from(&owners_soa)?;

    let rows = (0..owners.user_ids.len())
        .map(|i| {
            let user_id = &owners.user_ids[i];
            let display_name = &owners.display_names[i];
            let user_name = &owners.user_names[i];
            let entity_count = owners.entity_counts[i];
            let brick_count = owners.brick_counts[i];
            let component_count = owners.component_counts[i];
            let wire_count = owners.wire_counts[i];
            Row {
                user_id: user_id.uuid().to_string(),
                user_name: user_name.clone(),
                display_name: display_name.clone(),
                entity_counts: entity_count,
                brick_counts: brick_count,
                component_counts: component_count,
                wire_counts: wire_count,
            }
        })
        .collect::<Vec<_>>();
    print!("{}", serde_json::to_string_pretty(&rows)?);

    Ok(())
}
