# Lean Rust runtime artifact

Default Dockerfile packages the exact tested static linux/amd64 executable without network or compilation. Full source and lockfile retained.

Replaces Axum with direct Hyper HTTP/1, one current-thread Tokio network executor and a dedicated SQLite writer. SQLite optional modules pruned for size; SQLite WAL synchronous=FULL remains asserted on the application connection. SQL COMMIT precedes feed publication and acknowledgement. No benchmark-specific behaviour.

Binary SHA256: 8c4b7c7f4bbaab9bd2e4c368081828f311eac73ba0e7efc02146f8090bed73de
Application SHA256: 53ec851199df2a4485ce6d2a892a9593f80889bd62c15152d3878479b07f57ef
Tested scratch image: sha256:dcacb1bb5a820b5cf7d9bf0d06c551e53aa481ebfe0be2c4314f1a1bf50ea9bb
Verified uncompressed layer TAR bytes: 1464832.

Validation: archive validator and Docker load; 145 API checks, SIGKILL/restart feed equality and 6 restart checks passed. Extra HTTP/Unicode/chunked/pipelining and 1000 persistent connection regressions passed. Short 500VU diagnostic: candidate/control p99 both12ms, zero errors; not proof of speedup or official capacity. Generator2CPU, SUT1CPU2GiB, runc rather than official runsc. No sustained reliability claim for this build.

Build exact submitted package:
```
docker build --network=none -t devfest-lean:runtime starter
```
Dockerfile.source and Dockerfile.source.tested preserve the actually exercised source-build configuration. They require the provisioned local devfest-rust:builder image with compiler/dependencies, not a publicly available base. Default submission builds FROM scratch and needs no such builder. Source remains fully auditable; no claim of clean-environment source reproduction without provisioning that base.

Nonroot10001:10001, persistent /data, read-only root compatible. Previous releases preserved in Git history. Official approval/import/measurement remain separate gates.
