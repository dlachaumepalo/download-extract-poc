use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use tar::Builder;
use zstd::Encoder;

use crate::params::CompressionParams;
use crate::{read_zstandard_immutable_dictionary, StdResult};

pub fn create_tar_gz_archive(params: CompressionParams) -> StdResult<()> {
    let archive_file = File::create(params.destination)?;
    let archive_encoder = GzEncoder::new(archive_file, Compression::default());
    let mut archive_builder = Builder::new(archive_encoder);

    archive_builder.append_dir_all(".", "assets")?;

    archive_builder.into_inner()?.finish()?;

    Ok(())
}

pub fn create_zstandard_archive(params: CompressionParams) -> StdResult<()> {
    let archive_file = File::create(params.destination)?;
    let mut archive_encoder = match params.dictionary {
        None => Encoder::new(archive_file, params.level)?,
        Some(_) => Encoder::with_dictionary(
            archive_file,
            params.level,
            &read_zstandard_immutable_dictionary()?,
        )?,
    };

    if let Some(threads) = params.threads {
        archive_encoder.multithread(threads)?;
    }

    let mut archive_builder = Builder::new(archive_encoder);

    archive_builder.append_dir_all(".", "assets")?;

    archive_builder.into_inner()?.finish()?;

    Ok(())
}
