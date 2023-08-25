use flate2::read::GzDecoder;
use futures_util::TryStreamExt;
use reqwest::StatusCode;
use std::io::BufReader;
use tar::Archive;
use tokio_util::io::{StreamReader, SyncIoBridge};
use zstd::Decoder;

use crate::params::UnpackingParams;
use crate::{read_zstandard_immutable_dictionary, StdResult};

pub async fn download_unpack_tar_gz_archive(
    archive_url: &str,
    params: UnpackingParams,
) -> StdResult<()> {
    let http_client = reqwest::Client::new();
    let response = http_client.get(archive_url).send().await?;

    match response.status() {
        StatusCode::OK => {
            let read = StreamReader::new(
                response
                    .bytes_stream()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
            );

            let dest_dir = params.destination.to_path_buf();
            tokio::task::spawn_blocking(move || -> StdResult<()> {
                let file_tar_gz_decoder = GzDecoder::new(SyncIoBridge::new(read));
                let mut archive = Archive::new(file_tar_gz_decoder);
                archive.unpack(&dest_dir)?;
                Ok(())
            })
            .await?
        }
        status_code => Err(format!("[{status_code}] download failed").into()),
    }
}

pub async fn download_unpack_ztsd_archive(
    archive_url: &str,
    params: UnpackingParams,
) -> StdResult<()> {
    let http_client = reqwest::Client::new();
    let response = http_client.get(archive_url).send().await?;

    match response.status() {
        StatusCode::OK => {
            let read = StreamReader::new(
                response
                    .bytes_stream()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
            );

            let dest_dir = params.destination.to_path_buf();
            tokio::task::spawn_blocking(move || -> StdResult<()> {
                let file_zstd_decoder = match params.dictionary {
                    None => Decoder::new(SyncIoBridge::new(read))?,
                    Some(_) => Decoder::with_dictionary(
                        BufReader::new(SyncIoBridge::new(read)),
                        &read_zstandard_immutable_dictionary()?,
                    )?,
                };
                let mut archive = Archive::new(file_zstd_decoder);
                archive.unpack(&dest_dir)?;
                Ok(())
            })
            .await?
        }
        status_code => Err(format!("[{status_code}] download failed").into()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;
    use tokio::process::{Child, Command};

    use crate::compress::{create_tar_gz_archive, create_zstandard_archive};
    use crate::params::{CompressionParams, UnpackingParams};
    use crate::tests_utils::{compute_sha256_digest, get_temp_dir, EXPECTED_MITHRIL_LOGO_SVG_SHA};
    use crate::StdResult;

    use super::*;

    const SERVER_URL: &str = "127.0.0.1";

    fn start_python_server(run_dir: &Path, server_port: &str) -> StdResult<Child> {
        let mut command = Command::new("python3");
        command
            .current_dir(run_dir)
            .args(["-m", "http.server", "--bind", SERVER_URL, server_port])
            .kill_on_drop(true);
        Ok(command.spawn()?)
    }

    #[tokio::test]
    async fn create_and_unpack_while_downloading_gunzip_tarball() {
        let dir = get_temp_dir("tar-gz-download");
        let archive = dir.join("logo.tar.gz");
        println!("Dir: {}", dir.display());
        let port = "8002";
        let _child = start_python_server(&dir, port).unwrap();

        // Wait for the python server to be ready
        tokio::time::sleep(Duration::from_millis(500)).await;

        create_tar_gz_archive(CompressionParams::gunzip(&archive))
            .expect("compression to a `tar.gz` should not fail");
        download_unpack_tar_gz_archive(
            &format!("http://{SERVER_URL}:{port}/logo.tar.gz"),
            UnpackingParams::gunzip(&dir),
        )
        .await
        .expect("downloading and unpacking a `tar.gz` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_MITHRIL_LOGO_SVG_SHA);
    }

    #[tokio::test]
    async fn create_and_unpack_while_downloading_zstandard_tarball() {
        let dir = get_temp_dir("tar-zst-download");
        let archive = dir.join("logo.tar.zst");
        println!("Dir: {}", dir.display());
        let port = "8003";
        let _child = start_python_server(&dir, port).unwrap();

        // Wait for the python server to be ready
        tokio::time::sleep(Duration::from_millis(500)).await;

        create_zstandard_archive(CompressionParams::zstandard_multithread(&archive, 9, 8))
            .expect("compression to a `tar.zst` should not fail");
        download_unpack_ztsd_archive(
            &format!("http://{SERVER_URL}:{port}/logo.tar.zst"),
            UnpackingParams::zstandard(&dir),
        )
        .await
        .expect("downloading and unpacking a `tar.zst` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_MITHRIL_LOGO_SVG_SHA);
    }
}
