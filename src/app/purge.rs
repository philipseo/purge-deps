use std::path::Path;

use super::error::Error;

/// Delete a single file and log the path.
///
/// Example: `package-lock.json` → `Deleting file: "package-lock.json"`.
/// Dry-run never calls this; walk prints `Would delete file` instead.
pub fn delete_file(path: &Path) -> Result<(), Error> {
    println!("Deleting file: {:?}", path);
    std::fs::remove_file(path).map_err(|source| Error::Delete {
        path: path.to_path_buf(),
        source,
    })
}

/// Delete a directory tree and log the path.
///
/// Example: `node_modules/` → `Deleting folder: "node_modules"`.
pub fn delete_dir(path: &Path) -> Result<(), Error> {
    println!("Deleting folder: {:?}", path);
    std::fs::remove_dir_all(path).map_err(|source| Error::Delete {
        path: path.to_path_buf(),
        source,
    })
}
