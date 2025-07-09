use brdb::{
    AsBrdbValue, Brdb, Brz, Guid, IntoReader, OwnerTableSoA, pending::BrPendingFs,
    schemas::OWNER_TABLE_SOA,
};
use std::path::PathBuf;

/// Opens a world and replaces its owners with PUBLIC
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("world.brdb");
    let dst = PathBuf::from("world_patched.brz");

    assert!(src.exists());

    let db = Brdb::open(src)?.into_reader();

    let owners = db.owners_soa()?;
    // Make a new set of owners based on the owners soa
    let new_soa = OwnerTableSoA {
        display_names: owners
            .prop("DisplayNames")?
            .as_array()?
            .iter()
            .map(|_prev| "PUBLIC".to_owned())
            .collect(),
        user_names: owners
            .prop("UserNames")?
            .as_array()?
            .iter()
            .map(|_prev| "PUBLIC".to_owned())
            .collect(),
        user_ids: owners
            .prop("UserIds")?
            .as_array()?
            .iter()
            .map(|_prev| Guid::default())
            .collect(),
        brick_counts: owners
            .prop("BrickCounts")?
            .as_array()?
            .iter()
            .map(|i| i.as_brdb_u32())
            .collect::<Result<Vec<_>, _>>()?,
        component_counts: owners
            .prop("ComponentCounts")?
            .as_array()?
            .iter()
            .map(|i| i.as_brdb_u32())
            .collect::<Result<Vec<_>, _>>()?,
        entity_counts: owners
            .prop("EntityCounts")?
            .as_array()?
            .iter()
            .map(|i| i.as_brdb_u32())
            .collect::<Result<Vec<_>, _>>()?,
        wire_counts: owners
            .prop("WireCounts")?
            .as_array()?
            .iter()
            .map(|i| i.as_brdb_u32())
            .collect::<Result<Vec<_>, _>>()?,
    };

    // convert the owners struct of arrays into bytes using the owners schema
    let content = db.owners_schema()?.write_brdb(OWNER_TABLE_SOA, &new_soa)?;

    let patch = BrPendingFs::Root(vec![(
        "World".to_owned(),
        BrPendingFs::Folder(Some(vec![(
            "0".to_string(),
            BrPendingFs::Folder(Some(vec![(
                "Owners.mps".to_string(),
                BrPendingFs::File(Some(content)),
            )])),
        )])),
    )]);

    // use .to_pending_patch() if you want to update the same world
    let pending = db.to_pending()?.with_patch(patch)?;
    if dst.exists() {
        std::fs::remove_file(&dst)?;
    }
    Brz::write_pending(&dst, pending)?;

    println!("{}", Brz::open(&dst)?.into_reader().owners_soa()?);

    Ok(())
}
