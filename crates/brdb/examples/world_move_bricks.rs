use brdb::{Brdb, IntoReader, Position, UnsavedGrid};
use std::path::PathBuf;

/// Reads a world and prints out some of its information
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("world.brdb");
    let dst = PathBuf::from("world_patched.brdb");

    println!("Warning - This code will break if the brick chunk struct changes!!");

    let db = Brdb::open(path)?.into_reader();

    let data = db.global_data()?;
    let mut grid = UnsavedGrid::default();

    let mut total_bricks = 0;
    for chunk in db.brick_chunk_index(1)? {
        for brick in db
            .brick_chunk_soa(1, chunk.index)?
            .iter_bricks(chunk.index, data.clone())
        {
            // If we wanted wires/components, we'd need to track the bricks here by their chunk index and brick index
            total_bricks += 1;

            let mut brick = brick?;
            brick.position += Position::new(3000, 0, 0);
            grid.add_brick(data.as_ref(), &brick);
        }

        if chunk.num_components > 0 {
            println!("sorry, this example doesn't handle components");
        }
        if chunk.num_wires > 0 {
            println!("sorry, this example doesn't handle wires");
        }
    }
    println!("{total_bricks} bricks");

    let mut pending = db.to_pending()?;

    // Replace the main grid (1) with the grid we created
    *pending.cd_mut("World/0/Bricks/Grids/1")? = grid.to_pending(
        data.proc_brick_starting_index(),
        db.components_schema()?.as_ref(),
    )?;

    if dst.exists() {
        std::fs::remove_file(&dst)?;
    }
    Brdb::new(&dst)?.write_pending("Move the bricks", pending)?;

    // Verify bricks can be read
    let db = Brdb::open(dst)?.into_reader();
    for chunk in db.brick_chunk_index(1)? {
        let _ = db.brick_chunk_soa(1, chunk.index)?;
    }

    Ok(())
}
