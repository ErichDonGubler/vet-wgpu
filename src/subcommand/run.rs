use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
};

use anyhow::Context;
use clap::Parser;

use crate::{
    analysis::{DataSource, ExtractionStage, ExtractionStageName, PullRequest, StarterStage},
    config::{Config, GhConfig, TagsConfig},
    data_provider::{
        git::{self, Commit},
        github::{self, PullRequestId, RepoPath},
    },
};

#[derive(Debug, Parser)]
pub(crate) struct AnalyzeCommand {
    #[clap(subcommand)]
    subcommand: AnalyzeSubcommand,
}

#[derive(Debug, Parser)]
pub(crate) enum AnalyzeSubcommand {
    /// Starts a new analysis from scratch. Requires a connection to GitHub.
    ///
    /// TODO: Note authentication requirements for private repos.
    FromScratch {
        #[clap(long = "local-checkout")]
        local_checkout_path: PathBuf,
    },
    /// Resume analysis from local JSON files, and continue with data from GitHub. WARNING: Does
    /// not work yet!
    Resume { stage: ExtractionStageName },
    // TODO: Automatic command that just figures out what the latest stage is based on the presence
    // of files; basically
}

impl AnalyzeCommand {
    pub async fn run(self, config: Config, gh_auth_token: Option<String>) -> anyhow::Result<()> {
        let Self { subcommand } = self;
        let Config {
            tags,
            github:
                GhConfig {
                    repo_path,
                    trusted_reviewers,
                },
        } = config;

        let mut lazy_gh_client = LazyGithubClient::Uninit {
            repo_path,
            auth_token: gh_auth_token,
        };

        let mut extraction = match subcommand {
            AnalyzeSubcommand::FromScratch {
                local_checkout_path,
            } => {
                // TODO(erichdongubler): I really don't like explicitly naming TOML field paths. Any
                // way to just do it automatically?

                let local_checkout = git::Repository::discover(local_checkout_path)?;

                let TagsConfig { from, to } = tags;

                // let to = match to {
                //     Some(to) => to,
                //     None => {
                //         async {
                //             let most_recent_release = lazy_gh_client
                //                 .try_init()?
                //                 .recent_release_tags_descending(1)
                //                 .await
                //                 .context("failed to get most recent release from GitHub")?
                //                 .pop()
                //                 .context("no releases found from GitHub repository")?;
                //             anyhow::Result::<_>::Ok(most_recent_release)
                //         }
                //         .await
                //         .context(
                //             "failed to fetch `tags.to` automatically after finding it was omitted \
                //             in configuration",
                //         )?
                //         // TODO: validate `from` tag, see if we can find it in the (may need to adjust
                //         // number of recent releases requested)
                //     }
                // };

                let rev_list = local_checkout
                    // TODO: Validate that this works as expected.
                    .iter_rev_list_str(&from, &to)?
                    .collect::<Result<Vec<_>, _>>()?;

                ExtractionStage::Starter(StarterStage {
                    commits: rev_list,
                    trusted_reviewers,
                })
            }
            AnalyzeSubcommand::Resume { stage } => {
                todo!("restore data from local database, then use GH data source for `report`")
            }
        };

        match extraction
            .report(&mut lazy_gh_client)
            .await
            .context("failed to generate report, poop")
        {
            Ok(report) => println!("{}", report.outstanding()),
            Err(e) => log::error!("{e:#}"),
        }

        extraction.save().context("failed to save extracted data")?;

        Ok(())
    }
}

#[derive(Debug)]
struct LocalJsonCache {
    dir_path: PathBuf,
}

impl LocalJsonCache {
    pub fn from_dir_path(dir_path: PathBuf) -> Self {
        // TODO: Maybe do some validation?
        Self { dir_path }
    }
}

impl DataSource for LocalJsonCache {
    fn query_commit_prs<'a>(
        &'a mut self,
        _commits: &'a mut dyn Iterator<Item = Commit>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<(Commit, BTreeSet<PullRequestId>)>>> + 'a>>
    {
        todo!("use `serde` to grab commit-PR mapping(s) from local JSON")
    }

    fn query_pr_reviewers<'a>(
        &'a mut self,
        _prs: &'a mut dyn Iterator<Item = crate::data_provider::github::PullRequestId>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<BTreeMap<PullRequestId, PullRequest>>> + 'a>>
    {
        todo!("use `serde` to grab PR reviews from local JSON")
    }
}

enum LazyGithubClient {
    Uninit {
        repo_path: RepoPath,
        auth_token: Option<String>,
    },
    Init(github::Client),
}

impl LazyGithubClient {
    fn try_init(&mut self) -> anyhow::Result<&mut github::Client> {
        match self {
            Self::Init(client) => Ok(client),
            Self::Uninit {
                repo_path,
                auth_token,
            } => {
                let mut octocrab = octocrab::OctocrabBuilder::new();
                if let Some(auth_token) = auth_token {
                    octocrab = octocrab.personal_token(auth_token.clone());
                }
                // TODO: analyze error; if possibly an auth error, warn that no auth token was
                // provided, maybe that's the problem.
                let octocrab = octocrab.build().context("failed to build GitHub client")?;
                let client = github::Client::new(octocrab, repo_path.clone());

                *self = Self::Init(client);
                match self {
                    Self::Init(client) => Ok(client),
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl DataSource for LazyGithubClient {
    fn query_commit_prs<'a>(
        &'a mut self,
        commits: &'a mut dyn Iterator<Item = Commit>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<(Commit, BTreeSet<PullRequestId>)>>> + 'a>>
    {
        Box::pin(async {
            self.try_init()
                .map(|client| client.query_commit_prs(commits))?
                .await
        })
    }

    fn query_pr_reviewers<'a>(
        &'a mut self,
        prs: &'a mut dyn Iterator<Item = crate::data_provider::github::PullRequestId>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<BTreeMap<PullRequestId, PullRequest>>> + 'a>>
    {
        Box::pin(async {
            self.try_init()
                .map(|client| client.query_pr_reviewers(prs))?
                .await
        })
    }
}
