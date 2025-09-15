use brdb::{AsBrdbValue, Brdb, IntoReader, pending::BrPendingFs, schema::BrdbValue};
use std::path::PathBuf;

/// Opens a world and turns off shadow casting for all bricks
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("world.brdb");
    let dst = PathBuf::from("world_patched.brdb");

    assert!(src.exists());

    let db = Brdb::open(src)?.into_reader();

    let mut grid_ids = vec![1];

    // Iterate all entity chunks to find dynamic brick grids...
    // This could totally be a helper function
    for index in db.entity_chunk_index()? {
        for e in db.entity_chunk(index)? {
            // Ensure the chunk is a dynamic brick grid
            if !e
                .data
                .get_schema_struct()
                .is_some_and(|s| s.0.as_ref() == "Entity_DynamicBrickGrid")
            {
                continue;
            }
            let Some(id) = e.id else {
                continue;
            };
            grid_ids.push(id);
        }
    }

    let component_schema = db.components_schema()?;
    let mut grids_files = vec![];

    // Iterate all grids (there can be bricks on entities)
    for grid in &grid_ids {
        let chunks = db.brick_chunk_index(*grid)?;
        let mut chunk_files = vec![];
        let mut num_grid_modified = 0;

        // Iterate all chunks in the grid
        for index in chunks {
            let mut num_chunk_modified = 0;
            if index.num_components == 0 {
                println!("ignoring grid {grid} chunk {} with no components", *index);
                continue;
            }

            // Iterate all the components in the chunk
            let (mut soa, components) = db.component_chunk(*grid, *index)?;
            for mut s in components {
                // Disable the shadow casting property if it's present and true
                if s.prop("bCastShadows")
                    .is_ok_and(|v| v.as_brdb_bool().unwrap_or_default())
                {
                    println!(
                        "grid {grid} chunk {} mutating component {}",
                        *index,
                        s.get_name()
                    );
                    s.set_prop("bCastShadows", BrdbValue::Bool(false))?;
                    num_grid_modified += 1;
                    num_chunk_modified += 1;
                }

                soa.unwritten_struct_data.push(Box::new(s));
            }

            if num_chunk_modified == 0 {
                continue;
            }

            chunk_files.push((
                format!("{}.mps", *index),
                // ComponentChunkSoA::to_bytes ensures the extra data is written after the SoA data
                BrPendingFs::File(Some(soa.to_bytes(&component_schema)?)),
            ));
        }

        if num_grid_modified == 0 {
            println!("grid {grid} has no shadow-casting components, skipping");
            continue;
        } else {
            println!(
                "grid {grid} has {num_grid_modified} shadow-casting components in {} files",
                chunk_files.len()
            );
        }

        grids_files.push((
            grid.to_string(),
            BrPendingFs::Folder(Some(vec![(
                "Components".to_string(),
                BrPendingFs::Folder(Some(chunk_files)),
            )])),
        ))
    }

    let patch = BrPendingFs::Root(vec![(
        "World".to_owned(),
        BrPendingFs::Folder(Some(vec![(
            "0".to_string(),
            BrPendingFs::Folder(Some(vec![(
                "Bricks".to_string(),
                BrPendingFs::Folder(Some(vec![(
                    "Grids".to_string(),
                    BrPendingFs::Folder(Some(grids_files)),
                )])),
            )])),
        )])),
    )]);

    // Use .to_pending_patch() if you want to update the same world
    let pending = db.to_pending()?.with_patch(patch)?;
    if dst.exists() {
        std::fs::remove_file(&dst)?;
    }
    Brdb::new(&dst)?.write_pending("Disable Shadow Casting", pending)?;

    // Ensure all the components can be read
    let db = Brdb::open(dst)?.into_reader();
    for grid in grid_ids {
        let chunks = db.brick_chunk_index(grid)?;
        for index in chunks {
            if index.num_components == 0 {
                continue;
            }
            let (_soa, _components) = db.component_chunk(grid, *index)?;
        }
    }

    Ok(())
}
