# Rust competition entry

The default starter/Dockerfile now builds the Rust implementation in starter/src/main.rs. The Python implementation is retained as historical baseline code, not the production entrypoint.

## Architecture

Axum/hyper on Tokio serves concurrent HTTP requests. One dedicated thread owns a reused SQLite connection and serializes writes. SQLite WAL and synchronous=FULL are asserted at startup. Each operation commits before its committed feed snapshot is published and before a successful response is sent. Feed responses use immutable serialized snapshots. All posts and likes remain stored in SQLite; the feed is rebuilt from authoritative data on restart. This release does not use group commit.

The multi-stage build uses a pinned builder base and Cargo.lock. The runtime is a static musl binary in scratch, UID/GID 10001, linux/amd64. PORT and DATA_DIR are configurable.

## Build and checks

```sh
docker build --platform linux/amd64 -t devfest-rust:local starter
python3 bench/runner.py devfest-rust:local --smoke --pull never
```

The unchanged upstream checker passed 145 prepare checks and 6 SIGKILL/restart verification checks against the tested image. Additional differential checks covered 84 cases against the Python baseline. A concurrent crash ledger reconciled all 1,327 acknowledged posts and 1,252 acknowledged likes across five process/container kills. These are finite tests, not proof against hardware failure or physical power loss.

## Local diagnostic performance, not an official score

Dedicated four-vCPU host, API limited to one CPU and 2 GiB RAM, disk-backed data, generator limited to one CPU and 1.5 GiB RAM. Identical bulk fixtures of 10,000 posts and 100,000 likes, unchanged k6 workload, 20-second ramp and 20-second hold. Two subsequent matched repetitions pinned immutable image IDs.

At 1,000 VUs Rust passed with zero errors and p99 414.11 ms and 406.44 ms. The submitted Python baseline exceeded the 500 ms threshold at p99 952 ms and 876 ms. Short diagnostic runs do not establish sustained maximum capacity or an official event result. The host and storage differ from the official standard profile.

Tested Rust image: sha256:cbc1e0620358728adc495e0cf93f1b36265d76630a750f1bc53180c52491ad46. Docker-reported uncompressed size: 4,920,118 bytes. A rebuild can have a different image ID; organiser approval must apply to the exact built version.

Source integrity for this release:
- starter/src/main.rs: 208b5f40a4aeff5deedc7bb544131931792579167574820e5b019152b8a09087
- starter/Cargo.toml: 5d785583d03a141159128f00077d03acec5d9e7ba6da2b7ade52f1563683ef6a
- starter/Cargo.lock: f3784c31a17afdf22c1afed81581ede29f80467648dc496211366c55b5529249
- starter/Dockerfile: 17f2c823f9c5d25863b07f8f0d3ff9b881a7e191d976a13061f6c2871d80de77
