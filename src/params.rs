use std::path::{Path, PathBuf};

pub struct CompressionParams {
    /// Destination where the archive should be packed
    pub destination: PathBuf,
    /// The compression level (zstandard only)
    pub level: i32,
    /// Zstandard dictionary
    pub dictionary: Option<Vec<u8>>,
    /// Number of thread for zstandard compression, if None multithread will be disabled
    pub threads: Option<u32>,
}

impl CompressionParams {
    pub fn zstandard(destination: &Path, level: i32) -> Self {
        Self::zstandard_with_dict(destination, level, None)
    }

    pub fn zstandard_multithread(destination: &Path, level: i32, threads: u32) -> Self {
        Self {
            destination: destination.to_path_buf(),
            level,
            dictionary: None,
            threads: Some(threads),
        }
    }

    pub fn zstandard_with_dict(
        destination: &Path,
        level: i32,
        dictionary: Option<Vec<u8>>,
    ) -> Self {
        Self {
            destination: destination.to_path_buf(),
            level,
            dictionary,
            threads: None,
        }
    }

    pub fn gunzip(destination: &Path) -> Self {
        Self {
            destination: destination.to_path_buf(),
            level: 0,
            dictionary: None,
            threads: None,
        }
    }
}

pub struct UnpackingParams {
    /// Directory where the archive should be unpacked
    pub destination: PathBuf,
    /// Zstandard dictionary
    pub dictionary: Option<Vec<u8>>,
}

impl UnpackingParams {
    pub fn zstandard(destination: &Path) -> Self {
        Self::zstandard_with_dict(destination, None)
    }

    pub fn zstandard_with_dict(destination: &Path, dictionary: Option<Vec<u8>>) -> Self {
        Self {
            destination: destination.to_path_buf(),
            dictionary,
        }
    }

    pub fn gunzip(destination: &Path) -> Self {
        Self {
            destination: destination.to_path_buf(),
            dictionary: None,
        }
    }
}
