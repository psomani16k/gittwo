use git2::Error;

use crate::GitRepository;

pub struct FetchConfig {
    flags: FetchFlagsInternal,
    remote: 
}

impl FetchConfig {}

#[derive(Default)]
pub(crate) struct FetchFlagsInternal {
    deepen: Option<usize>,
    unshallow: bool,
}

pub enum FetchFlags {
    Deepen(usize),
    Unshallow(bool),
}

impl GitRepository {
    pub fn git_fetch(&self, config: FetchConfig) -> Result<(), Error> {
        if let Some(repository) = &self.repository {
        }

        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}
