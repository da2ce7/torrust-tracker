use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use crate::errors::wrappers::IoError;
use crate::errors::FilePathError;

pub fn get_file_at(at: &PathBuf, mode: &OpenOptions) -> Result<(File, PathBuf), FilePathError> {
    let file = mode.open(at).map_err(|error| FilePathError::FilePathIsNotAvailable {
        input: at.to_owned(),
        source: IoError::from(error).into(),
    })?;

    let at = Path::new(at)
        .canonicalize()
        .map_err(|error| FilePathError::FilePathIsUnresolvable {
            input: at.to_owned(),
            source: IoError::from(error).into(),
        })?;

    Ok((file, at))
}
