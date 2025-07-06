use brdb::{Brdb, Brick, Entity, World};
use std::path::PathBuf;

/// Writes a world a brick on a floating brick grid to example_entity.brdb
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("./example_entity.brdb");

    // Ensures the memory db can be created without errors
    let db = Brdb::new(&path)?.into_reader();
    let mut world = World::new();
    world.meta.bundle.description = "Example World".to_string();
    world.add_brick_grid(
        Entity {
            frozen: true,
            location: (0.0, 0.0, 40.0).into(),
            ..Default::default()
        },
        [Brick {
            position: (0, 0, 3).into(),
            color: (0, 255, 0).into(),
            ..Default::default()
        }],
    );

    db.save("example world", &world)?;

    println!("{}", db.get_fs()?.render());

    Ok(())
}
