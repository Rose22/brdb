use brdb::{
    Brdb, Brz, Guid, IntoReader, OwnerTableSoA, pending::BrPendingFs, schemas::OWNER_TABLE_SOA,
};
use std::path::PathBuf;

/// Opens a world and replaces its owners with PUBLIC
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("world.brdb");
    let dst = PathBuf::from("world_patched.brz");

    assert!(src.exists());

    let db = Brdb::open(src)?.into_reader();

    let owners = db.owners_soa()?;

    // Parse the owners from BrdbValues
    let mut new_soa = OwnerTableSoA::try_from(&owners.to_value())?;

    // Modify the owner ids
    new_soa
        .display_names
        .iter_mut()
        .for_each(|id| *id = "PUBLIC".to_owned());
    new_soa
        .user_names
        .iter_mut()
        .for_each(|id| *id = "PUBLIC".to_owned());
    new_soa
        .user_ids
        .iter_mut()
        .for_each(|id| *id = Guid::default());

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
