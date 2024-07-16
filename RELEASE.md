# Version release

To release a new Devnet version, follow these steps:

1. Increment the semver in Cargo.toml of those Devnet crates that have changed. Use `scripts/check_crate_changes.sh` for this.

2. Add a documentation entry for the incoming version by running:

   ```
   $ npm --prefix website run docusaurus docs:version <VERSION>
   ```

3. Create a PR styled after [this one](https://github.com/0xSpaceShard/starknet-devnet-rs/pull/473).

4. The publishing of crates, Docker images and documentation website is done automatically in the CI when the PR is merged into the main branch.

5. When the CI workflow is done, create a git tag of the form `v<VERSION>`, push it and create a GitHub release with notes describing changes since the last release.

6. Attach the [binary artifacts built in CI](https://circleci.com/docs/artifacts/#artifacts-overview) to the release. Use `scripts/fetch_ci_binaries.py` to fetch all artifacts of a CI workflow.
