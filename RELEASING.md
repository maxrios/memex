# Releasing memex

This document describes how to cut a release. The release workflow at `.github/workflows/release.yml` runs on every `v*` tag, builds binaries for Linux, macOS (Intel + Apple Silicon), and Windows, attaches them with SHA-256 checksums to a draft GitHub release, and publishes the crate to crates.io.

## Prerequisites (one-time)

- A `CARGO_REGISTRY_TOKEN` secret on the GitHub repo, scoped to publish the `memex` crate on [crates.io](https://crates.io/). Generate at <https://crates.io/me> and add via repo Settings → Secrets and variables → Actions.

## Cutting a release

1. Pick a version. memex follows [SemVer](https://semver.org/): bump `MINOR` for new features, `PATCH` for bug fixes only. While the project is in `0.x`, MINOR bumps may include breaking changes.

2. From a clean `main`:

   ```
   git checkout main && git pull
   ```

3. Update `version` in `Cargo.toml` and run `cargo build` once so `Cargo.lock` picks up the new version.

4. In `CHANGELOG.md`, move entries from `## [Unreleased]` into a new dated section for the version, and update the version-comparison links at the bottom.

5. Verify locally:

   ```
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all-targets
   cargo package --no-verify
   ```

   All must pass.

6. Commit (one commit is fine for the version bump + changelog):

   ```
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "chore: release v<version>"
   git push
   ```

7. Tag and push the tag:

   ```
   git tag v<version>
   git push origin v<version>
   ```

8. Watch the release workflow in GitHub Actions. When it finishes:
   - Confirm the draft GitHub release has all four target archives + checksums attached.
   - Confirm `https://crates.io/crates/memex` shows the new version.
   - Promote the draft release to published.
