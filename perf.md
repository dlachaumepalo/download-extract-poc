
## Context

Tests run on a computer with:
* Ubuntu 22.04.3 LTS
* 11th Gen Intel® Core™ i7-11800H @ 2.30GHz × 16 threads
* 32Gb of RAM

On a folder containing the first 20 immutables trio of the cardano mainnet for a total size of 400M.

## Recap

|                      | compress <br/>+ uncompress time | archive size |
|----------------------|---------------------------------|--------------|
| tar.gz               | 57.6s                           | 143M         |
| zstandard (level  3) | 6.5s                            | 137M         |
| zstandard (level  9) | 23.9s                           | 135M         |
| zstandard (level 22) | 1014.4s                         | 128M         |

## tar.gz

* execution time:
```shell
Starting 2 tests across 1 binary
    PASS [  57.247s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_gunzip_tarball
    PASS [  57.612s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_while_downloading_gunzip_tarball
```
* archives size:
```shell
╰─ ll -h /tmp/compression_prototype/**/*.tar.gz
-rw-rw-r-- 1 user user 143M août  25 10:21 /tmp/compression_prototype/tar-gz-download/logo.tar.gz
-rw-rw-r-- 1 user user 143M août  25 10:21 /tmp/compression_prototype/tar-gz/logo.tar.gz
```

## ZStandard

### Compression level 3 (default)

* execution time
```shell
$ cargo nextest run --no-fail-fast -E "test(zstandard)"
Finished test [unoptimized + debuginfo] target(s) in 0.03s
Starting 2 tests across 1 binary (2 skipped)
    PASS [   5.827s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_zstandard_tarball
    PASS [   6.489s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_while_downloading_zstandard_tarbal
```
* archives size:
```shell
$ ll -h /tmp/compression_prototype/**/*.tar.zst
-rw-rw-r-- 1 user user 137M août  25 10:56 /tmp/compression_prototype/tar-zst-download/logo.tar.zst
-rw-rw-r-- 1 user user 137M août  25 10:56 /tmp/compression_prototype/tar-zst/logo.tar.zst
```

### Compression level 9

* execution time
```shell
$ cargo nextest run --no-fail-fast -E "test(zstandard)"
Finished test [unoptimized + debuginfo] target(s) in 0.03s
Starting 2 tests across 1 binary (2 skipped)
    PASS [  23.280s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_while_downloading_ztsd_tarball
    PASS [  23.908s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_zstandard_tarball
```
* archives size:
```shell
$ ll -h /tmp/compression_prototype/**/*.tar.zst
-rw-rw-r-- 1 user user 135M août  25 10:52 /tmp/compression_prototype/tar-zst-download/logo.tar.zst
-rw-rw-r-- 1 user user 135M août  25 10:52 /tmp/compression_prototype/tar-zst/logo.tar.zst
```

### Compression level 22

* execution time
```shell
$ cargo nextest run --no-fail-fast -E "test(zstandard)"
Finished test [unoptimized + debuginfo] target(s) in 0.03s
Starting 2 tests across 1 binary (2 skipped)
    PASS [1014.090s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_while_downloading_ztsd_tarball
    PASS [1014.366s] download-extract-poc::bin/download-extract-poc tests::create_and_unpack_zstandard_tarball
```
* archives size:
```shell
$ ll -h /tmp/compression_prototype/**/*.tar.zst
-rw-rw-r-- 1 user user 128M août  25 10:37 /tmp/compression_prototype/tar-zst-download/logo.tar.zst
-rw-rw-r-- 1 user user 128M août  25 10:37 /tmp/compression_prototype/tar-zst/logo.tar.zst
```
