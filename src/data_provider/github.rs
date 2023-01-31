use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    str::FromStr,
};

use anyhow::Context;
use itertools::Itertools;
use octocrab::{models::User, Octocrab, Page};
use serde::Deserialize;

use crate::{
    analysis::{PullRequest, Review},
    data_provider::git::Commit,
};

pub(crate) use octocrab::models::PullRequestId;

// pub(crate) struct RecentReleaseFetcher {
//     state: RecentReleaseFetcherState,
// }

// enum RecentReleaseFetcherState {
//     Ready {
//         gh_auth_token: String,
//         org: String,
//         repo: String,
//     },
//     Fetched {
//         recent_release_tags_descending: Vec<String>,
//         idx: u8,
//     },
// }

// impl RecentReleaseFetcher {
//     pub fn new(gh_auth_token: String, org: String, repo: String) -> Self {
//         Self {
//             state: RecentReleaseFetcherState::Ready {
//                 gh_auth_token,
//                 org,
//                 repo,
//             },
//         }
//     }

//     pub async fn next(&mut self) -> anyhow::Result<String> {
//         let Self { state } = self;
//         let (recent_release_tags_descending, idx) = loop {
//             match state {
//                 RecentReleaseFetcherState::Ready {
//                     gh_auth_token,
//                     org,
//                     repo,
//                 } => {
//                     let gh_auth_token = mem::take(gh_auth_token);
//                     let recent_release_tags_descending =
//                         get_recent_release_tags_descending(gh_auth_token, org, repo, 2)
//                             .await
//                             .context("failed to get recent release tags")?;
//                     log::trace!("most recent GH release tags, descending order: {recent_release_tags_descending:?}");
//                     *state = RecentReleaseFetcherState::Fetched {
//                         recent_release_tags_descending,
//                         idx: 0,
//                     };
//                 }
//                 RecentReleaseFetcherState::Fetched {
//                     recent_release_tags_descending,
//                     idx,
//                 } => break (recent_release_tags_descending, idx),
//             }
//         };
//         let next = recent_release_tags_descending.pop().with_context(|| {
//             format!(
//                 "failed to get latest release at index {idx}; no more releases in retrieved set"
//             )
//         });
//         *idx += 1;
//         next
//     }
// pub(crate) async fn tag_arg_or_next_recent_release<'a>(
//     arg_name: &'static str,
//     arg: Option<String>,
//     local_checkout: &'a git2::Repository,
//     recent_releases: &mut RecentReleaseFetcher,
// ) -> anyhow::Result<git2::Object<'a>> {
//     let tag = match arg {
//         Some(arg) => arg,
//         None => {
//             let tag = recent_releases
//                 .next()
//                 .await
//                 .with_context(|| format!("failed to get infer `{arg_name:?}` argument"))?;
//             log::info!("resolving `{arg_name}` as tag {tag:?}");
//             tag
//         }
//     };
//     get_tag(arg_name, local_checkout, &tag)
// }
// }

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RepoPath {
    org: String,
    repo: String,
}

impl RepoPath {
    pub fn into_parts(self) -> (String, String) {
        let Self { org, repo } = self;
        (org, repo)
    }
}

impl FromStr for RepoPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (org, repo) = s
            .splitn(2, '/')
            .collect_tuple()
            .context("invalid repo path")?;
        Ok(Self {
            org: org.to_owned(),
            repo: repo.to_owned(),
        })
    }
}

// TODO: forward `Debug`
// TODO: impl `Display` using `"@{inner}"`.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Username(
    // TODO: replace with `ArcStr`
    String,
);

pub struct Client {
    inner: Octocrab,
    org: String,
    repo: String,
}

impl Client {
    pub(crate) fn new(inner: Octocrab, repo_path: RepoPath) -> Self {
        let (org, repo) = repo_path.into_parts();
        // TODO: validate repo path earlier?
        Self { inner, org, repo }
    }

    // pub(crate) async fn recent_release_tags_descending(
    //     &self,
    //     num_releases_per_page: u8,
    // ) -> anyhow::Result<Vec<String>> {
    //     let Self { inner, org, repo } = self;
    //     let releases = inner
    //         .repos(org, repo)
    //         .releases()
    //         .list()
    //         .per_page(num_releases_per_page)
    //         .send()
    //         .await
    //         .context("failed to fetch releases from GitHub")?;

    //     let release_tags = releases
    //         .into_iter()
    //         .map(|release| release.tag_name)
    //         .collect::<Vec<_>>();

    //     Ok(release_tags)
    // }
}

impl crate::analysis::DataSource for Client {
    fn query_commit_prs<'a>(
        &'a mut self,
        commits: &'a mut dyn Iterator<Item = Commit>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<(Commit, BTreeSet<PullRequestId>)>>> + 'a>>
    {
        // TODO: is this even correct?
        let Self { inner, org, repo } = self;
        Box::pin(async move {
            let mut commit_prs = Vec::new();
            for commit in commits {
                match inner
                    .repos(&*org, &*repo)
                    .list_pulls(commit.to_string())
                    .send()
                    .await
                {
                    Ok(Page {
                        items,
                        incomplete_results,
                        ..
                    }) => {
                        if incomplete_results.unwrap_or(false) {
                            log::warn!(
                                "output of PRs associated with commit were paged, this currently \
                                isn't handled correctly"
                            );
                        }
                        // TODO: We probably don't want to examine closed PRs (unless they're the
                        // only ones?), but we're not filtering this, ATM.
                        let pr_ids = items.into_iter().map(|pr| pr.id.into()).collect();
                        commit_prs.push((commit, pr_ids));
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
            Ok(commit_prs)
        })
    }

    fn query_pr_reviewers<'a>(
        &'a mut self,
        prs: &'a mut dyn Iterator<Item = PullRequestId>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<BTreeMap<PullRequestId, PullRequest>>> + 'a>>
    {
        let Self { inner, org, repo } = self;
        // TODO: is this even correct?
        Box::pin(async move {
            let mut pr_reviews = BTreeMap::new();
            for pr_id in prs {
                match inner.pulls(&*org, &*repo).list_reviews(101).await {
                    Ok(Page {
                        items,
                        incomplete_results,
                        ..
                    }) => {
                        if incomplete_results.unwrap_or(false) {
                            log::warn!(
                                "output of PRs associated with commit were paged, this currently \
                                isn't handled correctly"
                            );
                        }
                        // TODO: We probably don't want to examine "old" reviews, but we're not
                        // filtering this, ATM. Currently, we just filter to the "last" review
                        // entry returned by the API. :grimacing:
                        let reviews = items
                            .into_iter()
                            .filter_map(|review| {
                                let octocrab::models::pulls::Review {
                                    id,
                                    user: User { login, .. },
                                    state,
                                    ..
                                } = review;

                                let opt =
                                    state.map(|state| (Username(login), Review { id, state }));
                                if opt.is_none() {
                                    log::warn!("state of PR {pr_id}, review {id} was {state:?}");
                                }
                                opt
                            })
                            .collect();
                        pr_reviews.insert(pr_id, PullRequest { reviews });
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
            Ok(pr_reviews)
        })
    }
}
