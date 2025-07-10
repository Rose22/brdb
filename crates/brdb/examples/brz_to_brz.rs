use brdb::Brz;
use std::path::PathBuf;

/// Reads a brdb and outputs a brz
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = PathBuf::from("./world.brz");
    let dst = PathBuf::from("./world_copy.brz");

    let pending = Brz::open(src)?.to_pending()?;
    Brz::write_pending(dst, pending)?;

    Ok(())
}
