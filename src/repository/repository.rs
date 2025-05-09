use std::path::Path;

use git2::{Error, Repository};

use super::credentials::{GitCredentials, GitHttpsCredentials};

pub struct GitRepository {
    pub(crate) repository: Option<Repository>,
    pub(crate) cred: GitCredentials,
}

impl GitRepository {
    /// Create a `GitRepository` from an existing repository.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let repo = Repository::open(path)?;
        return Ok(GitRepository {
            cred: GitCredentials::Default,
            repository: Some(repo),
        });
    }

    /// Create an empty `GitRepository` object. Use this for cloning a repository.
    pub fn new() -> Self {
        GitRepository {
            cred: GitCredentials::Default,
            repository: None,
        }
    }

    /// Set credentials of the type username and password. Used when interacting
    /// with a remote repository over HTTPS.
    pub fn set_user_pass(&mut self, user: impl Into<String>, pass: impl Into<String>) {
        let http_cred = GitHttpsCredentials::new(Some(user.into()), Some(pass.into()));
        self.cred = GitCredentials::Https(http_cred);
    }

    /// Set credentials of the type username. Used when interacting
    /// with a remote repository over HTTPS.
    pub fn set_user(&mut self, user: impl Into<String>) {
        let http_cred = GitHttpsCredentials::new(Some(user.into()), None);
        self.cred = GitCredentials::Https(http_cred);
    }
}
