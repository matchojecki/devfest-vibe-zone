# Tested size-optimised Rust runtime

Default Dockerfile assembles the exact tested static linux/amd64 executable without network access or compilation. Full application source and Cargo.lock are retained.

Application source remains byte-identical to commit ef3ccc088c32ba1e871a5eb6e6b6f9ede1d50a0a. Changes are build settings only: Rust opt-level z, fat LTO, SQLite native size optimisation and removal of unused SQLite extensions. SQLite WAL and synchronous=FULL remain unchanged and asserted at startup. Thread-safety and API armour retained.

Binary SHA256: f8b2d785ca4b5086a8186927a5efe7c5ccb8cf8000fae3e79efb46d1cf977cea
Tested runtime image: sha256:27424170fdbcb14fbda4dca47dd4c9a45cbd3297182bf046fc3e7c341dc492cf
Verified uncompressed layer TAR bytes: 1612800.

Validation: archive validator and Docker load passed; 145 API checks passed, measured feed identical after SIGKILL/restart, then 6 restart checks passed. Local runc checks, not official runsc execution. No latency/capacity measurement was performed for this exact build before publication. No speedup claim.

Build runtime:
```
docker build --network=none -t devfest-rust:runtime starter
```

Source build in a sufficiently provisioned environment:
```
docker build -f starter/Dockerfile.source -t devfest-rust:source starter
```
Dockerfile.source uses the pinned upstream toolchain. Dockerfile.source.tested records the actually exercised source build using the existing provisioned devfest-rust:builder image and offline locked dependencies. The portable source build was not independently exercised. Default submission packaging uses the tested binary, not the source-build path.

Nonroot UID10001, read-only compatible root, persistent /data, same entrypoint and API behaviour. Earlier image and release remain preserved in Git history. Organiser approval and official measurement are separate gates.
