use flate2::read::GzDecoder;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tar::Archive;
use zstd::Decoder;

use crate::params::UnpackingParams;
use crate::{read_zstandard_immutable_dictionary, StdResult};

pub fn unpack_tar_gz_archive(archive: &Path, params: UnpackingParams) -> StdResult<()> {
    let file_tar_gz = File::open(archive)?;
    let file_tar_gz_decoder = GzDecoder::new(file_tar_gz);
    let mut archive = Archive::new(file_tar_gz_decoder);
    archive.unpack(&params.destination)?;

    Ok(())
}

pub fn unpack_zstandard_archive(archive: &Path, params: UnpackingParams) -> StdResult<()> {
    let file_zstd = File::open(archive)?;
    let file_zstd_decoder = match params.dictionary {
        None => Decoder::new(file_zstd)?,
        Some(_) => Decoder::with_dictionary(
            BufReader::new(file_zstd),
            &read_zstandard_immutable_dictionary()?,
        )?,
    };
    let mut archive = Archive::new(file_zstd_decoder);
    archive.unpack(params.destination)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::compress::{create_tar_gz_archive, create_zstandard_archive};
    use crate::params::CompressionParams;
    use crate::tests_utils::{compute_sha256_digest, get_temp_dir, EXPECTED_MITHRIL_LOGO_SVG_SHA};

    use super::*;

    #[test]
    fn create_and_unpack_gunzip_tarball() {
        let dir = get_temp_dir("tar-gz");
        let archive = dir.join("logo.tar.gz");
        println!("Dir: {}", dir.display());

        create_tar_gz_archive(CompressionParams::gunzip(&archive))
            .expect("compression to a `tar.gz` should not fail");
        unpack_tar_gz_archive(&archive, UnpackingParams::gunzip(&dir))
            .expect("unpacking a `tar.gz` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_MITHRIL_LOGO_SVG_SHA);
    }

    #[test]
    fn create_and_unpack_zstandard_tarball() {
        let dir = get_temp_dir("tar-zst");
        let archive = dir.join("logo.tar.zst");
        println!("Dir: {}", dir.display());

        create_zstandard_archive(CompressionParams::zstandard_multithread(&archive, 9, 8))
            .expect("compression to a `tar.zst` should not fail");
        unpack_zstandard_archive(&archive, UnpackingParams::zstandard(&dir))
            .expect("unpacking a `tar.zst` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_MITHRIL_LOGO_SVG_SHA);
    }
}
