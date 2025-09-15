use brdb::{Brdb, EntityChunkSoA, IntoReader, pending::BrPendingFs};
use std::path::PathBuf;

/// Opens a world and freezes all its entities
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("world.brdb");
    let dst = PathBuf::from("world_patched.brdb");

    assert!(src.exists());

    let db = Brdb::open(src)?.into_reader();

    let chunks = db.entity_chunk_index()?;
    let entity_schema = db.entities_schema()?;
    let global_data = db.global_data()?;
    let mut chunk_files = vec![];

    for index in chunks {
        // Entity_chunk loads entities and their entity data
        let entities = db.entity_chunk(index)?;

        // Re-assemble the soa. using add_entity ensures the extra data is correctly handled
        let mut soa = EntityChunkSoA::default();
        for mut e in entities.into_iter() {
            e.frozen = true;
            soa.add_entity(&global_data, &e, e.id.unwrap() as u32);
        }

        chunk_files.push((
            format!("{index}.mps"),
            // EntityChunkSoA::to_bytes ensures the extra data is written after the SoA data
            BrPendingFs::File(Some(soa.to_bytes(&entity_schema)?)),
        ));
    }

    let patch = BrPendingFs::Root(vec![(
        "World".to_owned(),
        BrPendingFs::Folder(Some(vec![(
            "0".to_string(),
            BrPendingFs::Folder(Some(vec![(
                "Entities".to_string(),
                BrPendingFs::Folder(Some(vec![(
                    "Chunks".to_string(),
                    BrPendingFs::Folder(Some(chunk_files)),
                )])),
            )])),
        )])),
    )]);

    // Use .to_pending_patch() if you want to update the same world
    let pending = db.to_pending()?.with_patch(patch)?;
    if dst.exists() {
        std::fs::remove_file(&dst)?;
    }
    Brdb::new(&dst)?.write_pending("Freeze Entities", pending)?;

    // Ensure entities can be read
    let db = Brdb::open(dst)?.into_reader();
    let chunks = db.entity_chunk_index()?;
    for index in chunks {
        let _ = db.entity_chunk(index)?;
    }

    Ok(())
}
