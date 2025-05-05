use std::path::{Path, PathBuf};

use git2::{
    AutotagOption, CertificateCheckStatus, Error, ErrorClass, ErrorCode, FetchOptions, Remote,
    RemoteCallbacks, Repository, build::RepoBuilder,
};

use crate::CloneConfig;

use super::credentials::{GitCredentials, GitHttpsCredentials};

pub struct GitRepository {
    repository: Option<Repository>,
    repo_path: Option<PathBuf>,
    cred: GitCredentials,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let repo = Repository::open(path)?;
        return Ok(GitRepository {
            cred: GitCredentials::Default,
            repo_path: Some(path.to_path_buf()),
            repository: Some(repo),
        });
    }

    pub fn new() -> Self {
        GitRepository {
            cred: GitCredentials::Default,
            repo_path: None,
            repository: None,
        }
    }

    pub fn set_user_pass(&mut self, user: impl Into<String>, pass: impl Into<String>) {
        let http_cred = GitHttpsCredentials::new(Some(user.into()), Some(pass.into()));
        self.cred = GitCredentials::Https(http_cred);
    }

    pub fn set_user(&mut self, user: impl Into<String>) {
        let http_cred = GitHttpsCredentials::new(Some(user.into()), None);
        self.cred = GitCredentials::Https(http_cred);
    }

    pub fn git_clone(&mut self, config: CloneConfig) -> Result<(), Error> {
        // skip user verification if configured so
        if config.skip_owner_validation {
            unsafe {
                git2::opts::set_verify_owner_validation(false)?;
            };
        }

        // continue even if cert checks fail, if configured so
        let mut callbacks = RemoteCallbacks::new();
        if config.bypass_certificate_check {
            callbacks.certificate_check(|_, _| Ok(CertificateCheckStatus::CertificateOk));
        }

        // setting up credentials
        let cred = self.cred.clone();
        callbacks.credentials(move |_a: &str, _b, _c| {
            return cred.get_cred();
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut remote = Remote::create_detached(config.get_url())?;
        remote.connect(git2::Direction::Fetch)?;

        // getting the name of the repository
        let repo_path;

        if let Some(repo_name) = remote.name() {
            let parent_path = config.get_parent_path();
            repo_path = parent_path.join(repo_name);
            self.repo_path = Some(repo_path.clone());
        } else {
            return Err(Error::new(
                ErrorCode::NotFound,
                ErrorClass::Object,
                "remote name not found",
            ));
        }

        let mut repo_builder = RepoBuilder::new();

        // setting flags
        if let Some(depth) = config.flags.depth {
            let depth: i32 = depth.get() as i32;
            fetch_options.depth(depth);
        }

        if let Some(()) = config.flags.single_branch {
            fetch_options.download_tags(AutotagOption::None);
        }

        if let Some(branch) = &config.flags.branch {
            repo_builder.branch(branch);
        }

        repo_builder.bare(config.flags.bare);

        // setting fetch options and cloning
        repo_builder.fetch_options(fetch_options);
        self.repository = Some(repo_builder.clone(config.get_url(), &repo_path)?);

        Ok(())
    }
}
