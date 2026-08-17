use std::io;
use std::path::PathBuf;

/// Recoverable failures while walking or deleting.
///
/// Example: a missing start path becomes [`Error::Walk`]; a permission
/// error on `remove_dir_all` becomes [`Error::Delete`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to walk directory: {0}")]
    Walk(String),
    #[error("failed to delete {path}: {source}")]
    Delete {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
