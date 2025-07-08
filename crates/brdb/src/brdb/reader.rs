use rusqlite::params;

use crate::{BrFsReader, Brdb, errors::BrFsError, fs::BrFs, tables::BrBlob};

impl BrFsReader for Brdb {
    fn get_fs(&self) -> Result<BrFs, BrFsError> {
        self.tree(None, 0)
    }

    fn find_folder(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError> {
        let res = self.conn.query_one(
            format!(
                "SELECT folder_id FROM folders WHERE {} AND name = ?1 AND deleted_at IS NULL;",
                match parent_id {
                    Some(parent_id) => format!("parent_id = {parent_id}"),
                    None => "parent_id IS NULL".to_owned(),
                }
            )
            .as_str(),
            params![name],
            |row| row.get(0),
        );
        match res {
            Ok(folder_id) => Ok(Some(folder_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BrFsError::Sqlite(e)),
        }
    }

    fn find_file(&self, parent_id: Option<i64>, name: &str) -> Result<Option<i64>, BrFsError> {
        let res = self.conn.query_one(
            format!(
                "SELECT content_id FROM files WHERE {} AND name = ?1 AND deleted_at IS NULL;",
                match parent_id {
                    Some(parent_id) => format!("parent_id = {parent_id}"),
                    None => "parent_id IS NULL".to_owned(),
                }
            )
            .as_str(),
            params![name],
            |row| row.get(0),
        );
        match res {
            Ok(file_id) => Ok(Some(file_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BrFsError::Sqlite(e)),
        }
    }

    fn find_blob(&self, content_id: i64) -> Result<BrBlob, BrFsError> {
        let res = self
        .conn
        .query_one(
            "SELECT blob_id, compression, size_uncompressed, size_compressed, delta_base_id, hash, content
            FROM blobs
            WHERE blob_id = ?1;",
            params![content_id],
            |row| {
                Ok(BrBlob {
                    blob_id: row.get(0)?,
                    compression: row.get(1)?,
                    size_uncompressed: row.get(2)?,
                    size_compressed: row.get(3)?,
                    delta_base_id: row.get(4)?,
                    hash: row.get(5)?,
                    content: row.get(6)?,
                })
            })?;
        Ok(res)
    }
}
