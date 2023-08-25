use flate2::read::GzDecoder;
use flume::Receiver;
use futures_util::StreamExt;
use reqwest::StatusCode;
use std::io;
use std::io::BufReader;
use std::path::Path;
use tar::Archive;
use zstd::Decoder;

use crate::params::UnpackingParams;
use crate::{read_zstandard_immutable_dictionary, CompressionAlgorithm, StdResult};

// All credits and many thanks to https://stackoverflow.com/a/69967522 for most of the channel code

pub struct ChannelRead {
    receiver: Receiver<Vec<u8>>,
    current: io::Cursor<Vec<u8>>,
}

impl ChannelRead {
    fn new(receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            receiver,
            current: io::Cursor::new(vec![]),
        }
    }
}

impl io::Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current.position() == self.current.get_ref().len() as u64 {
            // We've exhausted the previous chunk, get a new one.
            if let Ok(vec) = self.receiver.recv() {
                self.current = io::Cursor::new(vec);
            }
            // If recv() "fails", it means the sender closed its part of
            // the channel, which means EOF. Propagate EOF by allowing
            // a read from the exhausted cursor.
        }
        self.current.read(buf)
    }
}

fn run_unpack(
    dest_dir: &Path,
    receiver: Receiver<Vec<u8>>,
    params: &UnpackingParams,
    algorithm: CompressionAlgorithm,
) -> StdResult<()> {
    let input = ChannelRead::new(receiver);

    match algorithm {
        CompressionAlgorithm::ZStandard => {
            let decoder = match params.dictionary {
                None => Box::new(Decoder::new(input)?),
                Some(_) => Box::new(Decoder::with_dictionary(
                    BufReader::new(input),
                    &read_zstandard_immutable_dictionary()?,
                )?),
            };

            let mut archive = Archive::new(decoder);
            archive.unpack(dest_dir)?;
        }
        CompressionAlgorithm::Gunzip => {
            let decoder = Box::new(GzDecoder::new(input));
            let mut archive = Archive::new(decoder);
            archive.unpack(dest_dir)?;
        }
    };

    Ok(())
}

pub async fn download_unpack_archive_with_channel(
    archive_url: &str,
    params: UnpackingParams,
    algorithm: CompressionAlgorithm,
) -> StdResult<()> {
    let http_client = reqwest::Client::new();
    let response = http_client.get(archive_url).send().await?;

    let (sender, receiver) = flume::bounded(5);

    let dest_dir = params.destination.to_path_buf();
    let unpack_thread = tokio::task::spawn_blocking(move || -> StdResult<()> {
        run_unpack(&dest_dir, receiver, &params, algorithm)
    });

    match response.status() {
        StatusCode::OK => {
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.or(Err("Error while downloading".to_string()))?;
                sender.send_async(chunk.to_vec()).await?;
            }
            drop(sender); // Signal EOF

            unpack_thread.await?
        }
        status_code => Err(format!("[{status_code}] download failed").into()),
    }
}

pub async fn download_unpack_tar_gz_archive_with_channel(
    archive_url: &str,
    params: UnpackingParams,
) -> StdResult<()> {
    download_unpack_archive_with_channel(archive_url, params, CompressionAlgorithm::Gunzip).await
}

pub async fn download_unpack_ztsd_archive_with_channel(
    archive_url: &str,
    params: UnpackingParams,
) -> StdResult<()> {
    download_unpack_archive_with_channel(archive_url, params, CompressionAlgorithm::ZStandard).await
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
        let dir = get_temp_dir("tar-gz-download-channel");
        let archive = dir.join("logo.tar.gz");
        println!("Dir: {}", dir.display());
        let port = "8004";
        let _child = start_python_server(&dir, port).unwrap();

        // Wait for the python server to be ready
        tokio::time::sleep(Duration::from_millis(500)).await;

        create_tar_gz_archive(CompressionParams::gunzip(&archive))
            .expect("compression to a `tar.gz` should not fail");
        download_unpack_tar_gz_archive_with_channel(
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
        let dir = get_temp_dir("tar-zst-download-channel");
        let archive = dir.join("logo.tar.zst");
        println!("Dir: {}", dir.display());
        let port = "8005";
        let _child = start_python_server(&dir, port).unwrap();

        // Wait for the python server to be ready
        tokio::time::sleep(Duration::from_millis(500)).await;

        create_zstandard_archive(CompressionParams::zstandard_multithread(&archive, 9, 8))
            .expect("compression to a `tar.zst` should not fail");
        download_unpack_ztsd_archive_with_channel(
            &format!("http://{SERVER_URL}:{port}/logo.tar.zst"),
            UnpackingParams::zstandard(&dir),
        )
        .await
        .expect("downloading and unpacking a `tar.zst` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_MITHRIL_LOGO_SVG_SHA);
    }
}
