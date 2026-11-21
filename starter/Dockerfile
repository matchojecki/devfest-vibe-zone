# Runtime packaging uses the exact tested linux/amd64 executable.
# Rebuild from source with Dockerfile.source; provenance is in ARTIFACT.md.
FROM scratch
COPY --chmod=0555 bin/devfest-lean /devfest-lean
COPY --chown=10001:10001 runtime-data/ /data/
ENV PORT=8080 DATA_DIR=/data
USER 10001:10001
VOLUME ["/data"]
EXPOSE 8080
ENTRYPOINT ["/devfest-lean"]
