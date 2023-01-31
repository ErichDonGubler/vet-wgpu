use std::{
    future::{ready, Future},
    pin::Pin,
    sync::Arc,
};

use clap::Parser;

use crate::CliSubcommand;

#[derive(Debug, Parser)]
pub(crate) struct FetchCommitsCommand {}

impl FetchCommitsCommand {
    fn run(self) -> anyhow::Result<()> {
        todo!()
    }
}

impl CliSubcommand for FetchCommitsCommand {
    type AdditionalArgs = ();

    fn run(
        self,
        _config: Arc<crate::config::Config>,
        _additional_args: Self::AdditionalArgs,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>>>> {
        Box::pin(ready(self.run()))
    }
}
