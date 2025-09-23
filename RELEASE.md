# Version release

To release a new Devnet version, follow these steps:

1. Increment the semver in Cargo.toml of those Devnet crates that have changed. Use `scripts/check_crate_changes.sh` for this.

2. Add a documentation entry for the incoming version (without the v- prefix) by running:

   ```
   $ npm --prefix website run docusaurus docs:version <VERSION>
   ```

   - Feel free to delete documentation entries of old release candidates. E.g. if releasing 0.5.0, delete all files containing in them or in their names references to 0.5.0-rc.2. Or if releasing 0.5.0-rc.2, delete the entry of 0.5.0-rc.1.

3. Create a PR styled after [this one](https://github.com/0xSpaceShard/starknet-devnet-rs/pull/473).

4. The publishing of crates, Docker images and documentation website is done automatically in the CI when the PR is merged into the main branch.

   - This relies on the `CRATES_IO_API_KEY` environment variable to contain a crates.io token with write access.
   - If you are creating a pre-release, possibly from a side branch of a PR, CircleCI sets an environment variable indicating that the workflow is a part of a pull request. The documentation framework (Docusaurus) recognizes this and prevents the documentation from being deployed. Either deploy from your local machine (`npm run deploy`), or try manipulating the env var.

5. When the CI workflow is done, create a git tag of the form `v<VERSION>`, push it and create a GitHub release with notes describing changes since the last release.

6. Attach the [binary artifacts built in CI](https://circleci.com/docs/artifacts/#artifacts-overview) to the release. Use `scripts/fetch_ci_binaries.py` to fetch all artifacts of a CI workflow.

7. Adapt [starknet-devnet-js](https://github.com/0xSpaceShard/starknet-devnet-js) to the newly released Devnet. Check out one of the [old adaptation PRs](https://github.com/0xSpaceShard/starknet-devnet-js/pulls?q=is%3Apr+is%3Aclosed) for reference.

8. Update `starknet-foundry` to use the latest Devnet, if possible. Use [this PR](https://github.com/foundry-rs/starknet-foundry/pull/3434) for reference.
