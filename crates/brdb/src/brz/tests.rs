use std::error::Error;

use crate::{Brz, BrzIndexData};

#[test]
fn test_read_write() -> Result<(), Box<dyn Error>> {
    let empty = Brz {
        index_data: BrzIndexData::default(),
        blob_data: Vec::new(),
    };

    let buf_uncompressed = empty.to_vec(None)?;
    let out_uncompressed = Brz::read_slice(&buf_uncompressed)?;
    assert_eq!(out_uncompressed.blob_data.len(), 0);
    assert_eq!(out_uncompressed.index_data.num_folders, 0);
    assert_eq!(out_uncompressed.index_data.num_files, 0);
    assert_eq!(out_uncompressed.index_data.num_blobs, 0);
    assert_eq!(out_uncompressed.index_data.folder_parent_ids.len(), 0);
    assert_eq!(out_uncompressed.index_data.folder_names.len(), 0);
    assert_eq!(out_uncompressed.index_data.file_parent_ids.len(), 0);
    assert_eq!(out_uncompressed.index_data.file_content_ids.len(), 0);
    assert_eq!(out_uncompressed.index_data.file_names.len(), 0);
    assert_eq!(out_uncompressed.index_data.compression_methods.len(), 0);
    assert_eq!(out_uncompressed.index_data.sizes_uncompressed.len(), 0);
    assert_eq!(out_uncompressed.index_data.sizes_compressed.len(), 0);
    assert_eq!(out_uncompressed.index_data.blob_hashes.len(), 0);
    assert_eq!(out_uncompressed.index_data.blob_ranges.len(), 0);
    assert_eq!(out_uncompressed.index_data.blob_total_size, 0);

    let buf_compressed = empty.to_vec(Some(14))?;
    let out_compressed = Brz::read_slice(&buf_compressed)?;
    assert_eq!(out_compressed.blob_data.len(), 0);
    assert_eq!(out_compressed.index_data.num_folders, 0);
    assert_eq!(out_compressed.index_data.num_files, 0);
    assert_eq!(out_compressed.index_data.num_blobs, 0);
    assert_eq!(out_compressed.index_data.folder_parent_ids.len(), 0);
    assert_eq!(out_compressed.index_data.folder_names.len(), 0);
    assert_eq!(out_compressed.index_data.file_parent_ids.len(), 0);
    assert_eq!(out_compressed.index_data.file_content_ids.len(), 0);
    assert_eq!(out_compressed.index_data.file_names.len(), 0);
    assert_eq!(out_compressed.index_data.compression_methods.len(), 0);
    assert_eq!(out_compressed.index_data.sizes_uncompressed.len(), 0);
    assert_eq!(out_compressed.index_data.sizes_compressed.len(), 0);
    assert_eq!(out_compressed.index_data.blob_hashes.len(), 0);
    assert_eq!(out_compressed.index_data.blob_ranges.len(), 0);
    assert_eq!(out_compressed.index_data.blob_total_size, 0);

    assert_eq!(57, buf_compressed.len()); // (the compressed value would be 62 but that's larger than the uncompressed)
    assert_eq!(57, buf_uncompressed.len());
    Ok(())
}
