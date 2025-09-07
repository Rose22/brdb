use brdb::{
    BitFlags, Brdb, Brz, EntityChunkSoA, IntoReader, pending::BrPendingFs,
    schemas::ENTITY_CHUNK_SOA,
};
use std::path::PathBuf;

/// Opens a world and replaces its owners with PUBLIC
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("world.brdb");
    let dst = PathBuf::from("world_patched.brz");

    assert!(src.exists());

    let db = Brdb::open(src)?.into_reader();

    let chunks = db.entity_chunk_index()?;
    let entity_schema = db.entities_schema()?;
    let mut chunk_files = vec![];
    for index in chunks {
        let chunk = db.entity_chunk_soa(index)?.to_value();
        let mut soa: EntityChunkSoA = (&chunk).try_into()?;
        soa.physics_locked_flags = BitFlags::new_full(soa.locations.len());
        chunk_files.push((
            format!("{index}.mps"),
            BrPendingFs::File(Some(entity_schema.write_brdb(ENTITY_CHUNK_SOA, &soa)?)),
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

    // use .to_pending_patch() if you want to update the same world
    let pending = db.to_pending()?.with_patch(patch)?;
    if dst.exists() {
        std::fs::remove_file(&dst)?;
    }
    Brz::write_pending(&dst, pending)?;

    Ok(())
}
