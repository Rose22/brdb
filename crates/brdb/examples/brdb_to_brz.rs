use brdb::{Brdb, Brz};
use std::path::PathBuf;

/// Reads a brdb and outputs a brz
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("./world.brdb");
    let dst = PathBuf::from("./world.brz");

    let pending = Brdb::open(src)?.to_pending()?;
    Brz::write_pending(dst, pending)?;

    Ok(())
}
