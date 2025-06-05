use git2::Error;

use crate::GitRepository;

pub struct PullConfig {
    flags: PullFlagsInternal,
}

pub(crate) struct PullFlagsInternal {
    rebase: Option<PullFlagRebaseOptions>,
}

pub enum PullFlags {
    Rebase(PullFlagRebaseOptions),
}

pub enum PullFlagRebaseOptions {
    True,
    False,
    Merges,
}

impl GitRepository {
    pub fn git_pull(&self, config: PullConfig) -> Result<(), Error> {
        Ok(())
    }
}
