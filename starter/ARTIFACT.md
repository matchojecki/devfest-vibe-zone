# Reproducible source and tested runtime artifact

The default Dockerfile packages a prebuilt static linux/amd64 executable to avoid downloading and unpacking a Rust toolchain inside the competition's bounded import workspace. It performs no network access and no RUN instructions.

The executable is extracted unchanged from tested Rust #8 image `sha256:cbc1e0620358728adc495e0cf93f1b36265d76630a750f1bc53180c52491ad46`.
Binary SHA256: `ff369c3b145aadc30677a251f3f446c27a56eae0024f454bfe005d88a348071e`.
Source commit: `ef3ccc088c32ba1e871a5eb6e6b6f9ede1d50a0a`.
The Rust source, Cargo manifests and lockfile remain in this directory. No application changes or performance experiments are included.

To rebuild from source in a sufficiently provisioned environment:

```sh
docker build -f starter/Dockerfile.source -t devfest-rust:source starter
```

To assemble the tested runtime artifact:

```sh
docker build --network=none -t devfest-rust:runtime starter
```

The default image retains nonroot UID10001, persistent /data, the same entrypoint and FULL WAL application settings. A different packaging image digest is expected even though the executable is identical. Official importer success remains subject to the organiser's actual environment; local artifact validation does not prove the cause of the earlier failure.
