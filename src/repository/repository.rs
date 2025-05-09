use std::path::{Path, PathBuf};

use git2::{
    AutotagOption, CertificateCheckStatus, Error, FetchOptions, Remote, RemoteCallbacks,
    Repository, build::RepoBuilder,
};

use crate::CloneConfig;

use super::credentials::{GitCredentials, GitHttpsCredentials};

pub struct GitRepository {
    repository: Option<Repository>,
    repo_path: Option<PathBuf>,
    cred: GitCredentials,
}

impl GitRepository {
    /// Create a `GitRepository` from an existing repository.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let repo = Repository::open(path)?;
        return Ok(GitRepository {
            cred: GitCredentials::Default,
            repo_path: Some(path.to_path_buf()),
            repository: Some(repo),
        });
    }

    /// Create an empty `GitRepository` object. Use this for cloning a repository.
    pub fn new() -> Self {
        GitRepository {
            cred: GitCredentials::Default,
            repo_path: None,
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

    /// Performs an action equivalent to `git clone`. Returns `Ok(())` if the repository
    /// is already cloned. If not, clones the repository and makes `self` ready for other
    /// operations on the repository.
    pub fn git_clone(&mut self, config: CloneConfig) -> Result<(), Error> {
        if self.repository.is_some() {
            return Ok(());
        }
        let mut fetch_options = FetchOptions::new();
        let mut repo_builder = RepoBuilder::new();
        let mut callbacks = RemoteCallbacks::new();
        let mut callbacks2 = RemoteCallbacks::new();
        let mut remote = Remote::create_detached(config.url.clone())?;

        // skip user verification if configured so
        if config.skip_owner_validation {
            unsafe {
                git2::opts::set_verify_owner_validation(false)?;
            };
        }

        // continue even if cert checks fail, if configured so
        if config.bypass_certificate_check {
            callbacks.certificate_check(|_, _| Ok(CertificateCheckStatus::CertificateOk));
        }

        // setting up credentials
        let cred = self.cred.clone();
        let cred2 = self.cred.clone();
        callbacks.credentials(move |_a: &str, _b, _c| {
            return cred.get_cred();
        });
        callbacks2.credentials(move |_a: &str, _b, _c| {
            return cred2.get_cred();
        });

        let remote = remote.connect_auth(git2::Direction::Fetch, Some(callbacks2), None)?;
        let mut def_branch: Vec<u8> = vec![];
        remote.default_branch()?.clone_into(&mut def_branch);
        let def_branch = String::from_utf8(def_branch);
        let mut def_branch = def_branch.unwrap_or("main".to_string());
        def_branch = def_branch.split("/").last().unwrap_or("main").to_string();

        // getting the name of the repository
        let repo_path = config.get_parent_path().join(config.get_clone_dir_name());

        // +---------------+
        // | SETTING FLAGS |
        // +---------------+

        // branch
        if let Some(branch) = &config.flags.branch {
            def_branch = branch.to_string();
        }

        // depth
        if let Some(depth) = config.flags.depth {
            let depth: i32 = depth as i32;
            fetch_options.depth(depth);
            fetch_options.download_tags(AutotagOption::None);
            let branch = def_branch.clone();
            repo_builder.remote_create(move |repo, name, url| {
                let refspec = format!("+refs/heads/{0:}:refs/remotes/origin/{0:}", branch);
                repo.remote_with_fetch(name, url, &refspec)
            });
        }

        // single-branch
        if config.flags.single_branch {
            fetch_options.download_tags(AutotagOption::None);
            let branch = def_branch.clone();
            repo_builder.remote_create(move |repo, name, url| {
                let refspec = format!("+refs/heads/{0:}:refs/remotes/origin/{0:}", branch);
                repo.remote_with_fetch(name, url, &refspec)
            });
        }

        // bare
        let repo_builder = repo_builder.bare(config.flags.bare);

        // +--------------+
        // | CLONING REPO |
        // +--------------+

        let repo_builder = repo_builder.branch(&def_branch);

        fetch_options.remote_callbacks(callbacks);

        // setting fetch options and cloning
        let repo_builder = repo_builder.fetch_options(fetch_options);
        self.repository = Some(repo_builder.clone(config.get_url(), &repo_path)?);

        Ok(())
    }
}
