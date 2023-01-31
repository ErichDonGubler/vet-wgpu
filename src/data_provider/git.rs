use std::{
    fmt::{self, Debug, Display, Formatter},
    io,
    path::Path,
};

use anyhow::{anyhow, ensure, Context};
use git2::{Object, Oid};

pub(crate) struct Repository {
    inner: git2::Repository,
}

pub(crate) struct Tag<'a> {
    inner: Object<'a>,
}

impl<'a> Tag<'a> {
    fn new(inner: Object<'a>) -> Self {
        Self { inner }
    }

    // pub fn as_inner(&self) -> &Object<'_> {
    //     let Self { inner } = self;
    //     inner
    // }

    pub fn id(&self) -> impl Display + '_ {
        let Self { inner } = self;
        inner.id()
    }
}

impl Debug for Tag<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { inner } = self;
        Debug::fmt(inner, f)
    }
}

impl Repository {
    pub fn discover(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        git2::Repository::discover(path.as_ref())
            .map(|inner| Self { inner })
            .map_err(|e| {
                anyhow!(
                    "failed to open Git repo at local checkout path: {}",
                    e.message()
                )
            })
    }

    pub fn get_tag<'a>(&'a self, what: &'static str, spec: &str) -> anyhow::Result<Tag<'a>> {
        let Self { inner } = self;
        let (obj, ref_) = inner.revparse_ext(spec).map_err(|e| {
            anyhow!(
                "failed to find object {spec:?} in local checkout: {}",
                e.message()
            )
        })?;
        log::trace!("{what:?} object found: {:?}", obj);
        ensure!(
            ref_.map_or(false, |ref_| ref_.is_tag()),
            "{what:} object is not a tag"
        );
        Ok(Tag::new(obj))
    }

    pub fn iter_rev_list(
        &self,
        from: &Tag<'_>,
        to: &Tag<'_>,
    ) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Commit>> + '_> {
        let Self { inner } = self;
        let mut rev_walker = inner
            .revwalk()
            .context("failed to initialize revision walker")?;
        rev_walker
            .push_range(&format!("{}..{}", from.id(), to.id()))
            .context("failed to push range to revision walker")?;

        Ok(rev_walker.map(|res| {
            res.map(|inner| Commit { inner })
                .context("failed to walk through revision")
        }))
    }

    pub fn iter_rev_list_str(
        &self,
        from: &str,
        to: &str,
    ) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Commit>> + '_> {
        let from = self.get_tag("from", from)?;
        log::debug!("`from` resolves to {from:?}");

        let to = self.get_tag("to", to)?;
        log::debug!("`to` resolves to {to:?}");

        self.iter_rev_list(&from, &to)
    }

    pub fn print_rev_list_str(
        &self,
        from: &str,
        to: &str,
        output: &mut dyn io::Write,
    ) -> anyhow::Result<()> {
        let mut walk_err_happened = false;
        for res in self.iter_rev_list_str(&from, &to)? {
            if let Err(e) = res.and_then(|commit| {
                writeln!(output, "{commit}").context("failed to write commit hash to output")
            }) {
                walk_err_happened = true;
                log::error!("{e:?}");
            }
        }

        ensure!(
            !walk_err_happened,
            "one or more errors occurred while walking through revisions; see `log` for more info",
        );

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Commit {
    inner: Oid,
}

impl Display for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { inner } = self;
        Display::fmt(inner, f)
    }
}
