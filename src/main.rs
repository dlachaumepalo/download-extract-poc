use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use futures_util::TryStreamExt;
use reqwest::StatusCode;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};
use tar::{Archive, Builder};
use tokio_util::io::{StreamReader, SyncIoBridge};
use zstd::{Decoder, Encoder};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Sync + Send>>;

struct CompressionParams {
    /// Destination where the archive should be packed
    destination: PathBuf,
    /// The compression level (zstandard only)
    level: i32,
    /// Zstandard dictionary
    dictionary: Option<Vec<u8>>,
    /// Number of thread for zstandard compression, if None multithread will be disabled
    threads: Option<u32>,
}

impl CompressionParams {
    fn zstandard(destination: &Path, level: i32) -> Self {
        Self::zstandard_with_dict(destination, level, None)
    }

    fn zstandard_multithread(destination: &Path, level: i32, threads: u32) -> Self {
        Self {
            destination: destination.to_path_buf(),
            level,
            dictionary: None,
            threads: Some(threads),
        }
    }

    fn zstandard_with_dict(destination: &Path, level: i32, dictionary: Option<Vec<u8>>) -> Self {
        Self {
            destination: destination.to_path_buf(),
            level,
            dictionary,
            threads: None,
        }
    }

    fn gunzip(destination: &Path) -> Self {
        Self {
            destination: destination.to_path_buf(),
            level: 0,
            dictionary: None,
            threads: None,
        }
    }
}

struct UnpackingParams {
    /// Directory where the archive should be unpacked
    destination: PathBuf,
    /// Zstandard dictionary
    dictionary: Option<Vec<u8>>,
}

impl UnpackingParams {
    fn zstandard(destination: &Path) -> Self {
        Self::zstandard_with_dict(destination, None)
    }

    fn zstandard_with_dict(destination: &Path, dictionary: Option<Vec<u8>>) -> Self {
        Self {
            destination: destination.to_path_buf(),
            dictionary,
        }
    }

    fn gunzip(destination: &Path) -> Self {
        Self {
            destination: destination.to_path_buf(),
            dictionary: None,
        }
    }
}

fn create_tar_gz_archive(params: CompressionParams) -> Result<()> {
    let archive_file = File::create(params.destination)?;
    let archive_encoder = GzEncoder::new(archive_file, Compression::default());
    let mut archive_builder = Builder::new(archive_encoder);

    archive_builder.append_dir_all(".", "assets")?;

    archive_builder.into_inner()?.finish()?;

    Ok(())
}

fn unpack_tar_gz_archive(archive: &Path, params: UnpackingParams) -> Result<()> {
    let file_tar_gz = File::open(archive)?;
    let file_tar_gz_decoder = GzDecoder::new(file_tar_gz);
    let mut archive = Archive::new(file_tar_gz_decoder);
    archive.unpack(&params.destination)?;

    Ok(())
}

async fn download_unpack_tar_gz_archive(archive_url: &str, params: UnpackingParams) -> Result<()> {
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
            tokio::task::spawn_blocking(move || -> Result<()> {
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

fn create_zstd_archive(params: CompressionParams) -> Result<()> {
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

fn unpack_zstd_archive(archive: &Path, params: UnpackingParams) -> Result<()> {
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

async fn download_unpack_ztsd_archive(archive_url: &str, params: UnpackingParams) -> Result<()> {
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
            tokio::task::spawn_blocking(move || -> Result<()> {
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

fn read_zstandard_immutable_dictionary() -> Result<Vec<u8>> {
    let mut dictionary = File::open("./dictionary")?;
    let mut result = vec![];
    dictionary.read_to_end(&mut result)?;

    Ok(result)
}

fn main() -> Result<()> {
    // copy mithril svg logo to asset folder
    // sha256:
    // `sha256sum mithril-explorer/public/logo.svg`
    // d29d4ae5320b168a45a524639c45419c5e1185c4c92d2c2bcb02a8657a0369ec  mithril-explorer/public/logo.svg

    // Program - Step 1:
    // create a tar.gz archive of the logo
    // create a zstandard archive of the logo (tar.ztsd ?)
    // unpack tar.gz logo and check the sha256 checksum
    // unpack zstandard logo and check the sha256 checksum

    // Step 2:
    // Have a small webserver serving both archives (the server run in a dedicated thread)
    // Alternatively we can use python: `python3 -m http.server --bind 127.0.0.1 8001` this command start a server in the current folder
    // redo the unpack but "streamly" without downloading the archive first

    let archive = Path::new("logo.tar.gz");
    create_tar_gz_archive(CompressionParams::gunzip(archive))?;
    unpack_tar_gz_archive(archive, UnpackingParams::gunzip(Path::new(".")))?;

    let archive = Path::new("logo.tar.zst");
    create_zstd_archive(CompressionParams::zstandard(archive, 0))?;
    unpack_zstd_archive(archive, UnpackingParams::zstandard(Path::new(".")))?;

    Ok(())
}

#[cfg(test)]

mod tests {
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self},
        io,
        path::PathBuf,
        time::Duration,
    };
    use tokio::process::{Child, Command};

    use super::*;

    const EXPECTED_SHA: &str = "d29d4ae5320b168a45a524639c45419c5e1185c4c92d2c2bcb02a8657a0369ec";
    const SERVER_URL: &str = "127.0.0.1";

    fn get_temp_dir(dir_name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("compression_prototype")
            .join(dir_name);

        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }

        let _ = fs::create_dir_all(&dir);

        dir
    }

    fn compute_sha256_digest(filepath: &Path) -> Result<String> {
        let mut file = fs::File::open(filepath)?;
        let mut hasher = Sha256::new();
        io::copy(&mut file, &mut hasher)?;
        Ok(hex::encode(hasher.finalize()))
    }

    fn start_python_server(run_dir: &Path, server_port: &str) -> Result<Child> {
        let mut command = Command::new("python3");
        command
            .current_dir(run_dir)
            .args(["-m", "http.server", "--bind", SERVER_URL, server_port])
            .kill_on_drop(true);
        Ok(command.spawn()?)
    }

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
        assert_eq!(hash, EXPECTED_SHA);
    }

    #[test]
    fn create_and_unpack_zstandard_tarball() {
        let dir = get_temp_dir("tar-zst");
        let archive = dir.join("logo.tar.zst");
        println!("Dir: {}", dir.display());

        create_zstd_archive(CompressionParams::zstandard_multithread(&archive, 9, 8))
            .expect("compression to a `tar.zst` should not fail");
        unpack_zstd_archive(&archive, UnpackingParams::zstandard(&dir))
            .expect("unpacking a `tar.zst` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_SHA);
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
        assert_eq!(hash, EXPECTED_SHA);
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

        create_zstd_archive(CompressionParams::zstandard_multithread(&archive, 9, 8))
            .expect("compression to a `tar.zst` should not fail");
        download_unpack_ztsd_archive(
            &format!("http://{SERVER_URL}:{port}/logo.tar.zst"),
            UnpackingParams::zstandard(&dir),
        )
        .await
        .expect("downloading and unpacking a `tar.zst` should not fail");

        let hash = compute_sha256_digest(&dir.join("logo.svg")).unwrap();
        assert_eq!(hash, EXPECTED_SHA);
    }
}
