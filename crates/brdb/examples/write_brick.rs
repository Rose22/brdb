use brdb::{BrFsReader, Brdb, Brick, IntoReader, World};
use std::path::PathBuf;

/// Writes a world with one brick to example_brick
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("./example_brick.brdb");

    let mut world = World::new();
    world.meta.bundle.description = "Example World".to_string();
    world.bricks.push(Brick {
        position: (0, 0, 6).into(),
        color: (255, 0, 0).into(),
        ..Default::default()
    });

    world.write_brdb(&path)?;

    let db = Brdb::new(&path)?.into_reader();

    println!("file structure: {}", db.get_fs()?.render());

    let soa = db.brick_chunk_soa(1, (0, 0, 0).into())?;
    let color = soa.colors_and_alphas[0];
    assert_eq!(color.r, 255);
    assert_eq!(color.g, 0);
    assert_eq!(color.b, 0);
    assert_eq!(color.a, 5);

    Ok(())
}
