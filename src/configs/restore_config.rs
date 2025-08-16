use git2::Error;

use crate::GitRepository;

#[derive(Clone)]
pub struct RestoreConfig {
    pub(crate) pathspecs: Vec<String>,
    pub(crate) flags: RestoreFlagsInternal,
}

impl RestoreConfig {
    pub fn new(pathspecs: Vec<String>) -> Self {
        RestoreConfig {
            pathspecs,
            flags: RestoreFlagsInternal::default(),
        }
    }

    pub fn add_flag(&mut self, flag: RestoreFlags) {
        match flag {
            RestoreFlags::Staged(staged) => self.flags.staged = staged,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct RestoreFlagsInternal {
    staged: bool,
}

#[derive(Clone, Copy)]
pub enum RestoreFlags {
    Staged(bool),
}

impl GitRepository {
    pub fn git_restore(&self, config: RestoreConfig) -> Result<(), Error> {
        if let Some(repository) = &self.repository {
            // restore
        }
        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}
