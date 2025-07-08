/// Reads a world and prints out its SQLite schema
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from("./world.brdb");

    println!("{}", brdb::Brdb::open(path)?.sqlite_schema()?);

    Ok(())
}
