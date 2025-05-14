use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

use git2::{
    AutotagOption, CertificateCheckStatus, Error, FetchOptions, Remote, RemoteCallbacks,
    build::RepoBuilder,
};

use crate::GitRepository;

#[derive(Clone)]
/// A struct used to specify various details needed to clone a repository.
pub struct CloneConfig {
    pub(crate) clone_dir_name: String,
    pub(crate) parent_path: PathBuf,
    pub(crate) url: String,
    pub(crate) sender: Option<mpsc::Sender<(usize, String)>>,
    pub(crate) flags: CloneFlagsInternal,
}

impl CloneConfig {
    /// Takes in a `url` to clone the repository from and
    /// a `parent_dir` to clone the repository into.
    ///
    /// The clone process will make a directory with the
    /// repository name in the `parent_dir` and clone the repository
    /// into this dir, just like how git does.
    pub fn new(url: impl Into<String>, parent_dir: &Path) -> Self {
        let target_dir: String = url.into();
        let url = target_dir.clone();
        let target_dir = target_dir.split("/").last().unwrap();
        let target_dir = match target_dir.strip_suffix(".git") {
            Some(t) => t,
            None => target_dir,
        };

        CloneConfig {
            clone_dir_name: target_dir.to_string(),
            parent_path: parent_dir.to_path_buf(),
            url,
            flags: CloneFlagsInternal::default(),
            sender: None,
        }
    }

    // getters

    /// Returns the url set for the repository to be cloned.
    pub fn get_url(&self) -> &str {
        &self.url
    }

    /// Returns the consumer (Receiver) end of a mpsc. Intended to receive
    /// git cli like update messages with an associated index.
    pub fn get_update_channel(&mut self) -> mpsc::Receiver<(usize, String)> {
        let (sender, receiver) = mpsc::channel();
        self.sender = Some(sender);
        receiver
    }

    /// Returns the directory where the repository is to be cloned.
    pub fn get_parent_path(&self) -> &Path {
        &self.parent_path
    }

    /// Returns the name of the directory where the content of the repository will
    /// be. The final path of the repository will be the parent_path/clone_dir_name/
    pub fn get_clone_dir_name(&self) -> String {
        if self.flags.bare {
            let mut dir = self.clone_dir_name.clone();
            dir += ".git";
            return dir;
        }
        self.clone_dir_name.clone()
    }

    // setters

    /// Set a custom name for the cloned repository. This will only change
    /// the name of the directory that will be formed, parent path will still
    /// be where the directory is created.
    pub fn custom_clone_directory(&mut self, dir: impl Into<String>) {
        self.clone_dir_name = dir.into();
    }

    /// "Pass" a flag to the git clone command.
    pub fn add_flag(&mut self, flag: CloneFlags) -> &Self {
        match flag {
            CloneFlags::Branch(branch) => self.flags.branch = branch,
            CloneFlags::Depth(depth) => self.flags.depth = depth,
            CloneFlags::SingleBranch(single) => self.flags.single_branch = single,
            CloneFlags::Bare(bare) => self.flags.bare = bare,
        }
        self
    }
}

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CloneFlagsInternal {
    pub(crate) branch: Option<String>, // Branch(String),
    pub(crate) depth: Option<usize>,   // Depth(NonZeroUsize),
    pub(crate) single_branch: bool,    // SingleBranch,
    pub(crate) bare: bool,
}

/// An enum representing the various flags that can be added to the `git clone` command.
pub enum CloneFlags {
    /// `--branch` or `-b` flag for git clone.
    /// Some(branch) will set the flag, None will unset it.
    /// Defaults to None.
    Branch(Option<String>),

    /// `--depth n` flag for git clone.
    /// Some(n) will set depth to n, None will unset the flag
    /// Defaults to None.
    Depth(Option<usize>),

    /// `--single-branch` flag for git clone.
    /// true will set the flag, false will unset it.
    /// Defaults to false.
    SingleBranch(bool),

    /// `--bare` flag for git clone.
    /// true will set the flag, false will unset it.
    /// Defaults to false.
    Bare(bool),
}

impl GitRepository {
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
        if self.skip_owner_validation {
            unsafe {
                git2::opts::set_verify_owner_validation(false)?;
            };
        }

        // continue even if cert checks fail, if configured so
        if self.bypass_certificate_check {
            callbacks.certificate_check(|_, _| Ok(CertificateCheckStatus::CertificateOk));
        }

        // setting up credentials
        let cred = self.cred.clone();
        let cred2 = self.cred.clone();
        callbacks.credentials(move |_a: &str, _b, _c| cred.get_cred());
        callbacks2.credentials(move |_a: &str, _b, _c| cred2.get_cred());

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
