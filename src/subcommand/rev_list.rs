use std::{io, path::PathBuf};

use clap::Parser;

use crate::{
    config::{Config, TagsConfig},
    data_provider::git,
};

#[derive(Debug, Parser)]
pub(crate) struct RevListCommand {
    #[clap(long = "local-checkout")]
    local_checkout_path: PathBuf,
}

impl RevListCommand {
    pub(crate) async fn run(
        self,
        config: Config,
        mut output: Box<dyn io::Write>,
    ) -> anyhow::Result<()> {
        let Config {
            github: _,
            tags: TagsConfig { from, to },
        } = config;

        let Self {
            local_checkout_path,
        } = self;

        let local_checkout = git::Repository::discover(&local_checkout_path)?;

        local_checkout.print_rev_list_str(&from, &to, &mut *output)?;
        Ok(())
    }
}
