use crate::GitRepository;
use git2::{
    AutotagOption, CertificateCheckStatus, Error, FetchOptions, Remote, RemoteCallbacks,
    build::RepoBuilder,
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

#[derive(Clone)]
/// Specifies details for a `git clone` operation.
pub struct CloneConfig {
    pub(crate) clone_dir_name: String,
    pub(crate) parent_path: PathBuf,
    pub(crate) url: String,
    pub(crate) sender: Option<mpsc::Sender<(usize, String)>>,
    pub(crate) flags: CloneFlagsInternal,
}

impl CloneConfig {
    /// Creates a new `CloneConfig` with the repository URL and the parent directory for the clone.
    ///
    /// A new directory, named after the repository (e.g., "gittwo" from
    /// "https://github.com/psomani16k/gittwo.git"), will be created inside `parent_dir`. The repository will be cloned into this new directory.
    pub fn new(url: String, parent_dir: &Path) -> Self {
        let target_dir: String = url;
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

    /// Returns the URL of the repository to be cloned.
    pub fn get_url(&self) -> &str {
        &self.url
    }

    /// Returns the receiver end of a multi-producer, single-consumer (mpsc) channel.
    /// This channel is used to receive progress updates during the clone operation,
    /// similar to Git CLI output, with an associated index of each line.
    pub fn get_update_channel(&mut self) -> mpsc::Receiver<(usize, String)> {
        let (sender, receiver) = mpsc::channel();
        self.sender = Some(sender);
        receiver
    }

    /// Returns the parent directory where the repository will be cloned.
    pub fn get_parent_path(&self) -> &Path {
        &self.parent_path
    }

    /// Returns the name of the directory for the cloned repository.
    /// The full path will be `parent_path/clone_dir_name/`.
    pub fn get_clone_dir_name(&self) -> String {
        if self.flags.bare {
            let mut dir = self.clone_dir_name.clone();
            dir += ".git";
            return dir;
        }
        self.clone_dir_name.clone()
    }

    // setters

    /// Sets a custom name for the directory where the repository will be cloned.
    /// This overrides the default name derived from the repository URL.
    pub fn custom_clone_directory(&mut self, dir: impl Into<String>) {
        self.clone_dir_name = dir.into();
    }

    /// Configures a specific flag for the `git clone` operation.
    pub fn add_flag(&mut self, flag: CloneFlags) -> &Self {
        match flag {
            CloneFlags::Branch(branch) => self.flags.branch = branch,
            CloneFlags::Depth(depth) => self.flags.depth = depth,
            CloneFlags::SingleBranch(single) => self.flags.single_branch = single,
            CloneFlags::Bare(bare) => self.flags.bare = bare,
            CloneFlags::Recursive(rec) => self.flags.recursive = rec,
        }
        self
    }
}

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Internal representation of clone flags, not intended for direct public use.
pub(crate) struct CloneFlagsInternal {
    pub(crate) branch: Option<String>,
    pub(crate) depth: Option<usize>,
    pub(crate) single_branch: bool,
    pub(crate) bare: bool,
    pub(crate) recursive: Option<Vec<String>>,
}

/// Represents flags that can be applied to a `git clone` command.
/// See [git clone documentation](https://git-scm.com/docs/git-clone) for more details on each flag.
pub enum CloneFlags {
    /// Corresponds to the [`--branch`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt-code--branchcodeemltnamegtem)
    /// or `-b` flag.
    /// `Some(branch_name)` specifies the branch to checkout. `None` uses the remote's default branch.
    ///
    /// Defaults to `None`.
    Branch(Option<String>),

    /// Corresponds to the [`--depth <depth>`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt)
    /// flag.
    /// `Some(n)` creates a shallow clone with a history truncated to `n` commits. `None` implies a full clone.
    ///
    /// Defaults to `None`.
    Depth(Option<usize>),

    /// Corresponds to the [`--single-branch`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---single-branch)
    /// flag.
    /// `true` clones only the history leading to the tip of a single branch (either the one specified by `--branch` or the remote's default).
    /// `false` clones all branches.
    ///
    /// Defaults to `false`.
    SingleBranch(bool),

    /// Corresponds to the [`--bare`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---bare)
    /// flag.
    /// `true` creates a bare Git repository (no working directory). `false` creates a standard repository.
    ///
    /// Defaults to `false`.
    Bare(bool),

    /// Corresponds to the [`--recursive`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---recursive)
    /// or [`--recurse-submodules[=<pathspec>]`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---recurse-submodulesltpathspecgt) flag.
    /// `Some(pathspecs)` initializes submodules matching the pathspecs. An empty vector initializes all submodules.
    /// `None` does not initialize submodules.
    ///
    /// Defaults to `None`.
    Recursive(Option<Vec<String>>),
}

impl GitRepository {
    /// Clones a Git repository based on the provided `CloneConfig`.
    ///
    /// If GitRepository was created using `GitRepository::new()` this will allow you to clone a
    /// remote repository to the provided directory. If GitRepository was created using
    /// `GitRepository::open()` calling this function will return an error.
    pub fn git_clone(&mut self, config: CloneConfig) -> Result<(), Error> {
        if self.repository.is_some() {
            return Err(git2::Error::from_str(
                "git_clone() called on a pre-existing repository.",
            ));
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
        let repository = repo_builder.clone(config.get_url(), &repo_path)?;

        if let Some(pathspecs) = config.flags.recursive {
            if pathspecs.is_empty() {
                let submodule = repository.submodules()?;
                for mut sub in submodule {
                    // git seems to ignore errors in cloning submodule
                    // TODO: investigate this
                    let _ = sub.clone(None);
                }
            }
            for pathspec in pathspecs {
                if let Ok(mut submodule) = repository.find_submodule(&pathspec) {
                    // git seems to ignore errors in cloning submodule
                    // TODO: investigate this
                    let _ = submodule.clone(None);
                }
            }
        }
        self.repository = Some(repository);

        Ok(())
    }
}

#[cfg(test)]
mod clone_test {
    use super::{CloneConfig, CloneFlags};
    use crate::GitRepository;
    use std::{io::BufRead, path::Path, process::Command};

    #[test]
    fn git_clone_depth_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_depth"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_depth"),
        );
        config.add_flag(CloneFlags::Depth(Some(1)));
        repo.git_clone(config).unwrap();

        // verify that a single commit is cloned in
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/clone_depth/git2-rs/",
                "rev-list",
                "--count",
                "--all",
            ])
            .output()
            .unwrap();

        Command::new("rm")
            .args(["-rf", "./temp_test/clone_depth/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "1\n");
    }

    #[test]
    fn git_clone_bare_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_bare"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_bare"),
        );
        config.add_flag(CloneFlags::Bare(true));
        repo.git_clone(config).unwrap();

        // verify that repository is bare
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/clone_bare/git2-rs.git/",
                "rev-parse",
                "--is-bare-repository",
            ])
            .output()
            .unwrap();

        // delete the repository
        Command::new("rm")
            .args(["-rf", "./temp_test/clone_bare/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "true\n");
    }

    #[test]
    fn git_clone_branch_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_branch"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_branch/"),
        );
        config.add_flag(CloneFlags::Branch(Some(String::from("curl"))));
        repo.git_clone(config).unwrap();

        // verify that a single commit is cloned in
        let out = Command::new("git")
            .args(["-C", "./temp_test/clone_branch/git2-rs/", "branch"])
            .output()
            .unwrap();

        Command::new("rm")
            .args(["-rf", "./temp_test/clone_branch/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "* curl\n");
    }

    #[test]
    fn git_clone_single_branch_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_single_branch"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_single_branch/"),
        );
        config.add_flag(CloneFlags::SingleBranch(true));
        repo.git_clone(config).unwrap();

        // verify that a single commit is cloned in
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/clone_single_branch/git2-rs/",
                "branch",
                "--remotes",
            ])
            .output()
            .unwrap();

        let out = out.stdout.lines().count();
        Command::new("rm")
            .args(["-rf", "./temp_test/clone_single_branch/"])
            .output()
            .unwrap();

        assert_eq!(out, 2);
    }
}
