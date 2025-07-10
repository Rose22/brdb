use brdb::{AsBrdbValue, BrFsReader, Brick, Brz, IntoReader, World};
use std::path::PathBuf;

/// Writes a world with one brick to example_brick
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("./example_brick.brz");

    let mut world = World::new();
    world.meta.bundle.description = "Example World".to_string();
    world.bricks.push(Brick {
        position: (0, 0, 6).into(),
        color: (255, 0, 0).into(),
        ..Default::default()
    });

    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    world.write_brz(&path)?;

    let db = Brz::new(&path)?.into_reader();

    println!("{}", db.get_fs()?.render());

    let soa = db.brick_chunk_soa(1, (0, 0, 0).into())?;
    let color = soa.prop("ColorsAndAlphas")?.index(0)?.unwrap();
    assert_eq!(color.prop("R")?.as_brdb_u8()?, 255);
    assert_eq!(color.prop("G")?.as_brdb_u8()?, 0);
    assert_eq!(color.prop("B")?.as_brdb_u8()?, 0);
    assert_eq!(color.prop("A")?.as_brdb_u8()?, 5);

    Ok(())
}
