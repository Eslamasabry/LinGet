# Release artifacts and verification

The standard LinGet artifact is the terminal-first binary. It runs the TUI when invoked without a subcommand and does not link GTK, GDK, Libadwaita, Cairo, or Pango. The optional GUI is a separate `linget-gui-*` archive.

## Published targets

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- Optional GUI: `x86_64-unknown-linux-gnu`

Each tag publishes deterministic `.tar.gz` archives, a consolidated SHA-256 manifest, SPDX and CycloneDX JSON SBOMs, and GitHub build-provenance attestations.

## Verify a download

```bash
sha256sum --check linget-v0.2.0-checksums.txt --ignore-missing
gh attestation verify linget-v0.2.0-x86_64-unknown-linux-gnu.tar.gz \
  --repo Eslamasabry/LinGet
```

The installer downloads the checksum manifest and refuses an archive with a missing, malformed, or mismatched digest. A local archive must have a matching `.sha256` sidecar.

## Reproduce packaging

```bash
cargo build --locked --release --target x86_64-unknown-linux-gnu
scripts/package_release.sh --no-build --target x86_64-unknown-linux-gnu
scripts/verify_release.sh dist/linget-v*-x86_64-unknown-linux-gnu.tar.gz terminal
```

The package script normalizes file order, ownership, timestamps, locale, timezone, PAX metadata, and gzip headers. `SOURCE_DATE_EPOCH` defaults to the timestamp of the checked-out commit and can be set explicitly when comparing two builds. Packaging is byte-reproducible for identical input binaries and tracked assets; cross-machine Rust binary reproducibility is not yet independently certified.
