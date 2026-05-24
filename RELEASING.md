# Releasing memex

This document describes how to cut a release. The release workflow at `.github/workflows/release.yml` runs on every `v*` tag, builds binaries for Linux, macOS (Intel + Apple Silicon), and Windows, attaches them with SHA-256 checksums to a draft GitHub release, and publishes the crate to crates.io.

Each release is itself tracked as a memex node so the graph captures *what shipped* alongside *how it was built*, and the release-prep PR satisfies `memex-check` naturally — no `[skip memex]` bypass needed.

## Prerequisites (one-time)

- A `CARGO_REGISTRY_TOKEN` secret on the GitHub repo, scoped to publish the `memex` crate on [crates.io](https://crates.io/). Generate at <https://crates.io/me> and add via repo Settings → Secrets and variables → Actions.

## Cutting a release

1. **Pick a version.** memex follows [SemVer](https://semver.org/): bump `MINOR` for new features, `PATCH` for bug fixes only. While the project is in `0.x`, MINOR bumps may include breaking changes.

2. **Branch from a clean `main`:**

   ```
   git checkout main && git pull
   git checkout -b release/v<version>
   ```

3. **Create the release node.** Parent is the tip of `main` (find via `memex node list` or `memex graph view`):

   ```
   memex node create --parent <main-tip-id> --goal "Release v<version>" --tags release
   ```

4. **Bump the version.** Edit `version` in `Cargo.toml`, then run `cargo build` once so `Cargo.lock` picks up the new version.

5. **Roll the changelog.** In `CHANGELOG.md`, move entries from `## [Unreleased]` into a new dated section for the version, and update the version-comparison links at the bottom.

6. **Record release context on the node.** At minimum:

   ```
   memex node edit --artifact "Cargo.toml"
   memex node edit --artifact "CHANGELOG.md"
   ```

   If anything notable was deferred from this release, capture it as an open thread; if a scope decision was made (e.g. holding a feature back, choosing a version-bump tier), record it as a decision.

7. **Verify locally:**

   ```
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all-targets
   cargo package --no-verify
   ```

   All must pass.

8. **Resolve the node:**

   ```
   memex node resolve
   ```

9. **Commit, push, open PR, merge.** The release-prep commit includes the version bump, changelog roll, and the new node JSON:

   ```
   git add Cargo.toml Cargo.lock CHANGELOG.md .memex/nodes/*.json
   git commit -m "chore: release v<version>"
   git push -u origin release/v<version>
   gh pr create --title "chore: release v<version>" --body "Release-prep PR. See node <short-id> for context."
   ```

   Once CI and `memex-check` go green, merge the PR.

10. **Tag the merged commit:**

    ```
    git checkout main && git pull
    git tag v<version>
    git push origin v<version>
    ```

11. **Watch the release workflow** in GitHub Actions. When it finishes:
    - Confirm the draft GitHub release has all four target archives + checksums attached.
    - Confirm `https://crates.io/crates/memex` shows the new version.
    - Promote the draft release to published.
