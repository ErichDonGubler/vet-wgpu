use serde::Deserialize;

use crate::data_provider::github::{RepoPath, Username};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Config {
    pub tags: TagsConfig,
    pub github: GhConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct TagsConfig {
    pub from: String,
    pub to: String,
    // TODO: It'd be nice to eventually support automatic inference of `from` and `to` by reading
    // an `audit.toml`, or something that otherwise indicated the "latest" trusted release/commit.
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GhConfig {
    #[serde(flatten)]
    pub repo_path: RepoPath,
    pub trusted_reviewers: Vec<Username>,
}
