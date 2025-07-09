use brdb::{Brdb, Brz};
use std::path::PathBuf;

/// Reads a brz and outputs a brdb
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("./prefab.brz");
    let dst = PathBuf::from("./prefab.brdb");

    let pending = Brz::open(src)?.to_pending()?;
    Brdb::open(dst)?.write_pending("Import brz", pending)?;

    Ok(())
}
