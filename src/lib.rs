pub mod basic_unpack;
pub mod compress;
pub mod download_unpack;
pub mod params;

pub type StdResult<T> = Result<T, Box<dyn std::error::Error + Sync + Send>>;

pub fn read_zstandard_immutable_dictionary() -> StdResult<Vec<u8>> {
    use std::io::Read;

    let mut dictionary = std::fs::File::open("./dictionary")?;
    let mut result = vec![];
    dictionary.read_to_end(&mut result)?;

    Ok(result)
}

#[cfg(test)]
pub mod tests_utils {
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};
    use std::{fs, io};

    use super::*;

    pub const EXPECTED_MITHRIL_LOGO_SVG_SHA: &str =
        "d29d4ae5320b168a45a524639c45419c5e1185c4c92d2c2bcb02a8657a0369ec";

    pub fn get_temp_dir(dir_name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("compression_prototype")
            .join(dir_name);

        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }

        let _ = fs::create_dir_all(&dir);

        dir
    }

    pub fn compute_sha256_digest(filepath: &Path) -> StdResult<String> {
        let mut file = fs::File::open(filepath)?;
        let mut hasher = Sha256::new();
        io::copy(&mut file, &mut hasher)?;
        Ok(hex::encode(hasher.finalize()))
    }
}
