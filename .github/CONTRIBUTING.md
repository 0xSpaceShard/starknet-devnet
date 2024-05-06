## Pull requests

These guidelines are intended for external contributors.

> :warning: IMPORTANT NOTE :warning:
>
> All contributions are expected to be of the highest possible quality! That means the PR is thoroughly tested and documented, and without blindly generated ChatGPT code and documentation! PRs that do not comply with these rules stated here shall not be considered!

### Should you create a PR?

It is advised to [create an issue](https://github.com/0xSpaceShard/starknet-devnet-rs/issues/new/choose) before creating a PR. Creating an issue is the best way to reach somebody with repository-specific experience who can provide more info on how a problem/idea can be addressed and if a PR is needed.

### Development Docs

The readme contains a section which may be of use to contributors on [this link](https://github.com/0xSpaceShard/starknet-devnet-rs/?tab=readme-ov-file#development).

### Checklist

The [PR template](pull_request_template.md) contains a checklist. It is important to go through the checklist to ensure the expected quality standards and to ensure the CI workflow succeeds once it is executed.

### Review

Once a PR is created, somebody from the team will review it. When a reviewer leaves a comment, the PR author should not mark the conversation as resolved. This is because the repository has a setting that prevents merging if there are unresolved conversations - let the reviewer resolve. The author can reply back with:

- a request for clarification from the reviewer
- a link to the commit which addresses the reviewer's observation (simply pasting the sha-digest is enough)

This is an example of a good author-reviewer correspondence: [link](https://github.com/0xSpaceShard/starknet-devnet-rs/pull/310#discussion_r1457142002).
