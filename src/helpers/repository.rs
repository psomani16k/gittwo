use std::path::Path;

use git2::{Error, Repository};

use super::credentials::{CredType, GitCredentials, GitHttpsCredentials};

pub struct GitRepository {
    pub(crate) repository: Option<Repository>,
    pub(crate) cred: GitCredentials,
    pub(crate) skip_owner_validation: bool,
    pub(crate) bypass_certificate_check: bool,
}

impl GitRepository {
    // testtt
    /// Create a `GitRepository` from an existing repository.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let repo = Repository::open(path)?;
        Ok(GitRepository {
            cred: GitCredentials::Default,
            repository: Some(repo),
            skip_owner_validation: false,
            bypass_certificate_check: false,
        })
    }

    /// Create an empty `GitRepository` object. Use this for cloning a repository.
    pub fn new() -> Self {
        GitRepository {
            cred: GitCredentials::Default,
            repository: None,
            skip_owner_validation: false,
            bypass_certificate_check: false,
        }
    }

    /// Returns true if owner validation is to be skipped, false otherwise.
    pub fn get_skip_owner_validation(&self) -> bool {
        self.skip_owner_validation
    }

    /// Returns true if certification checks are to be bypassed, false otherwise.
    pub fn get_bypass_certificate_check(&self) -> bool {
        self.bypass_certificate_check
    }

    /// Set true to skip owner validation.
    pub fn skip_owner_validation(&mut self, skip: bool) {
        self.skip_owner_validation = skip;
    }

    /// Set true to skip certification checks.
    pub fn bypass_certificate_check(&mut self, bypass: bool) {
        self.bypass_certificate_check = bypass;
    }

    pub fn get_cred_type(&self) -> Result<CredType, Error> {
        match &self.cred {
            GitCredentials::Https(git_https_credentials) => git_https_credentials.get_cred_type(),
            GitCredentials::Default => Ok(CredType::Default),
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

    /// Returns `true` if the repository is cloned/init-ed and ready for other git operations.
    /// Returns `false` other wise.
    pub fn is_valid(&self) -> bool {
        return self.repository.is_some();
    }
}
