//! TODO: Elaborate on a `ExtractionStage` -> `Report`

// TODO: Q: What about commits merged that aren't PRs?

use std::{
    collections::BTreeMap, collections::BTreeSet, fmt::Display, future::Future, io, mem, pin::Pin,
};

use arg_enum_proc_macro::ArgEnum;
use enum_kinds::EnumKind;
use format::lazy_format;
use octocrab::models::{pulls::ReviewState, ReviewId};

use crate::data_provider::{
    git::Commit,
    github::{PullRequestId, Username},
};

pub(crate) trait DataSource {
    fn query_commit_prs<'a>(
        &'a mut self,
        commits: &'a mut dyn Iterator<Item = Commit>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<(Commit, BTreeSet<PullRequestId>)>>> + 'a>>;

    fn query_pr_reviewers<'a>(
        &'a mut self,
        prs: &'a mut dyn Iterator<Item = PullRequestId>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<BTreeMap<PullRequestId, PullRequest>>> + 'a>>;
}

// pub trait Database
// where
//     Self: DataSource,
// {
//     fn deserialize_starter(&self) -> anyhow::Result<StarterStage>;
//     fn serialize_state(&self, state: &ExtractionStage) -> anyhow::Result<()>;
// }

/// TODO: a grouping of stages of querying GitHub for information
#[derive(Clone, Debug, EnumKind)]
#[enum_kind(ExtractionStageName, derive(ArgEnum, Ord, PartialOrd))]
pub(crate) enum ExtractionStage {
    // NOTE: Needs:
    // * Local checkout to get commit info
    Starter(StarterStage),
    // NOTE: Needs:
    // * GH-provided set of PRs for each commit in `commits`
    CommitPullRequests(CommitPullRequestsStage),
    // NOTE: Needs:
    // * GH-provided set of reviewers for each PR
    PullRequestReviewers(PullRequestReviewersStage),
}

impl ExtractionStage {
    pub async fn run_until(
        &mut self,
        stage: ExtractionStageName,
        data_source: &mut dyn DataSource,
    ) -> anyhow::Result<()> {
        while ExtractionStageName::from(&*self) < stage {
            match self {
                Self::Starter(StarterStage {
                    trusted_reviewers,
                    commits,
                }) => {
                    let prs_by_commit =
                        Box::pin(data_source.query_commit_prs(&mut commits.iter().cloned()))
                            .await?;
                    *self = Self::CommitPullRequests(CommitPullRequestsStage {
                        trusted_reviewers: mem::take(trusted_reviewers),
                        prs_by_commit,
                    });
                }
                Self::CommitPullRequests(CommitPullRequestsStage {
                    trusted_reviewers,
                    prs_by_commit,
                }) => {
                    let pr_reviews = Box::pin(
                        data_source.query_pr_reviewers(
                            &mut prs_by_commit
                                .iter()
                                .flat_map(|(_commit, prs)| prs.iter().cloned()),
                        ),
                    )
                    .await?;
                    *self = Self::PullRequestReviewers(PullRequestReviewersStage {
                        trusted_reviewers: mem::take(trusted_reviewers),
                        prs_by_commit: mem::take(prs_by_commit),
                        pr_reviews,
                    })
                }
                Self::PullRequestReviewers(_stage) => {
                    // Nothing left to do, so, uh...twiddle our thumbs here.
                }
            }
        }

        Ok(())
    }

    /// Generates a [`Report`], fetching data that's not already cached from `data_source`.
    ///
    /// This method uses all data from stages
    /// [`run_until`](Self::run_until) calls.
    pub async fn report<'a>(
        &'a mut self,
        data_source: &mut dyn DataSource,
    ) -> anyhow::Result<Report<'a>> {
        self.run_until(ExtractionStageName::PullRequestReviewers, data_source)
            .await?;

        let PullRequestReviewersStage {
            trusted_reviewers,
            prs_by_commit,
            pr_reviews,
        } = match &*self {
            Self::PullRequestReviewers(stage) => stage,
            _ => unreachable!("this should succeed after running to the final stage"),
        };

        let (prs_by_commit, LeastTrusted(overall_trust_level)) = prs_by_commit
            .iter()
            .map(|commit_and_prs| {
                let (commit, prs) = commit_and_prs;
                let LeastTrusted(commit_trust_level) =
                    prs.iter()
                        .map(move |pr_id| {
                            let user_trust = |user| {
                                if trusted_reviewers.contains(user) {
                                    TrustLevel::Trusted
                                } else {
                                    TrustLevel::Unknown
                                }
                            };
                            // OPT: we could probably cache individual PR trust results, or just
                            // flatten into a normalized list of PRs and _then_ query?
                            let PullRequest { reviews } = pr_reviews.get(pr_id).unwrap();
                            [todo!("get trust level from committer of {}", commit)]
                                .into_iter()
                                .map(user_trust)
                                .chain(reviews.iter().filter_map(
                                    |(user, Review { id: _, state })| {
                                        match (user_trust(user), state) {
                                            // TODO: we don't really model "untrusted" actors yet,
                                            // but when we do, boy, does this need to change.
                                            // :sweat:
                                            (TrustLevel::Untrusted, _)
                                            // TODO: Validate assumption: we can ignore unknown reviewers
                                            | (TrustLevel::Unknown, _)
                                            | (
                                                TrustLevel::Trusted,
                                                ReviewState::Commented
                                                | ReviewState::Pending
                                                | ReviewState::Dismissed
                                                | _ // needed, because non-exhaustive
                                            ) => None,
                                            (TrustLevel::Trusted, ReviewState::Approved) => {
                                                Some(TrustLevel::Trusted)
                                            }
                                            // TODO: Validate assumption: we shouldn't trust things we
                                            // requested changes for but merged anyway
                                            (
                                                TrustLevel::Trusted,
                                                ReviewState::ChangesRequested,
                                            ) => Some(TrustLevel::Untrusted),
                                        }
                                    },
                                ))
                                .min()
                                .unwrap()
                        })
                        .collect();

                (
                    CommitReport {
                        orig_commit_and_associated_prs: commit_and_prs,
                        trust_level: commit_trust_level,
                    },
                    commit_trust_level,
                )
            })
            .unzip();

        Ok(Report {
            overall_trust_level,
            prs_by_commit,
            pr_reviews,
        })
    }

    pub(crate) fn save(&self) -> anyhow::Result<()> {
        todo!("save stuff to local JSON (which should be new args)")
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StarterStage {
    pub trusted_reviewers: Vec<Username>,
    pub commits: Vec<Commit>,
}

#[derive(Clone, Debug)]
pub(crate) struct CommitPullRequestsStage {
    pub trusted_reviewers: Vec<Username>,
    pub prs_by_commit: Vec<(Commit, BTreeSet<PullRequestId>)>,
}

#[derive(Clone, Debug)]
pub(crate) struct PullRequestReviewersStage {
    pub trusted_reviewers: Vec<Username>,
    pub prs_by_commit: Vec<(Commit, BTreeSet<PullRequestId>)>,
    pub pr_reviews: BTreeMap<PullRequestId, PullRequest>,
}

#[derive(Clone, Debug)]
pub struct Report<'a> {
    overall_trust_level: TrustLevel,
    prs_by_commit: Vec<CommitReport<'a>>,
    pr_reviews: &'a BTreeMap<PullRequestId, PullRequest>,
}

impl Report<'_> {
    pub fn outstanding(&self) -> impl Display + '_ {
        let Self {
            overall_trust_level,
            prs_by_commit,
            pr_reviews,
        } = self;
        lazy_format!(
            "OVERALL AUDIT: {overall_trust_level:?}\
            \
            Remaining PRs to audit per commit: {prs_by_commit:#?}"
        )
    }

    pub fn write_tsv(&self) -> io::Result<()> {
        todo!("write some CSV/TSV that mere mortals (and Google Sheets) can understand")
    }
}

#[derive(Clone, Debug)]
pub struct CommitReport<'a> {
    orig_commit_and_associated_prs: &'a (Commit, BTreeSet<PullRequestId>),
    trust_level: TrustLevel,
}

#[derive(Clone, Debug)]
pub(crate) struct PullRequest {
    // // TODO: Merging is an endorsement, right? Would it make sense to treat as an approval?
    // merged_by: Username,
    pub reviews: BTreeMap<Username, Review>,
}

#[derive(Clone, Debug)]
pub(crate) struct Review {
    pub id: ReviewId,
    pub state: ReviewState,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TrustLevel {
    Untrusted,
    Unknown,
    // TODO: Maybe add a level for "untrusted, but good track record"?
    Trusted,
}

#[derive(Debug)]
struct LeastTrusted(TrustLevel);

impl Default for LeastTrusted {
    fn default() -> Self {
        Self(TrustLevel::Trusted)
    }
}

impl Extend<TrustLevel> for LeastTrusted {
    fn extend<T: IntoIterator<Item = TrustLevel>>(&mut self, iter: T) {
        let Self(trust_level) = self;
        if let Some(min) = iter.into_iter().min() {
            *trust_level = min;
        }
    }
}

impl FromIterator<TrustLevel> for LeastTrusted {
    fn from_iter<T: IntoIterator<Item = TrustLevel>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}
