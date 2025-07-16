use brdb::{BrFsReader, Brdb, IntoReader, pending::BrPendingFs};
use std::path::PathBuf;

/// Copies the Bundle.json from dst.brdb to src.brdb, allowing you to reupload
/// src.brdb to the gallery in place of dst.brdb.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let meta_dst = PathBuf::from("dst.brdb");
    let meta_src = PathBuf::from("src.brdb");

    let src_f = Brdb::open(meta_src)?.into_reader();
    let dst_f = Brdb::open(meta_dst)?.into_reader();

    println!(
        "replacing: {}",
        String::from_utf8(dst_f.read_file("Meta/Bundle.json")?).unwrap()
    );
    println!(
        "with: {}",
        String::from_utf8(src_f.read_file("Meta/Bundle.json")?).unwrap()
    );

    let patch = BrPendingFs::Root(vec![(
        "Meta".to_owned(),
        BrPendingFs::Folder(Some(vec![(
            "Bundle.json".to_string(),
            BrPendingFs::File(Some(src_f.read_file("Meta/Bundle.json")?)),
        )])),
    )]);

    dst_f.write_pending(
        "Replace Bundle",
        dst_f.to_pending_patch()?.with_patch(patch)?,
    )?;

    Ok(())
}
