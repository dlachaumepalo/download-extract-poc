use std::path::Path;

use download_extract_poc::basic_unpack::{unpack_tar_gz_archive, unpack_zstandard_archive};
use download_extract_poc::compress::{create_tar_gz_archive, create_zstandard_archive};
use download_extract_poc::params::{CompressionParams, UnpackingParams};
use download_extract_poc::StdResult;

fn main() -> StdResult<()> {
    let archive = Path::new("logo.tar.gz");
    create_tar_gz_archive(CompressionParams::gunzip(archive))?;
    unpack_tar_gz_archive(archive, UnpackingParams::gunzip(Path::new(".")))?;

    let archive = Path::new("logo.tar.zst");
    create_zstandard_archive(CompressionParams::zstandard(archive, 0))?;
    unpack_zstandard_archive(archive, UnpackingParams::zstandard(Path::new(".")))?;

    Ok(())
}
