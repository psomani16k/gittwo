use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

#[derive(Clone)]
/// A struct used to specify various details needed to clone a repository.
pub struct CloneConfig {
    pub(crate) clone_dir_name: String,
    pub(crate) parent_path: PathBuf,
    pub(crate) url: String,
    pub(crate) sender: Option<mpsc::Sender<(usize, String)>>,
    pub(crate) flags: CloneFlagsInternal,
    pub(crate) skip_owner_validation: bool,
    pub(crate) bypass_certificate_check: bool,
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
        let target_dir = target_dir.split("/").into_iter().last().unwrap();
        let target_dir = match target_dir.strip_suffix(".git") {
            Some(t) => t,
            None => target_dir,
        };

        CloneConfig {
            clone_dir_name: target_dir.to_string(),
            parent_path: parent_dir.to_path_buf(),
            url: url.into(),
            flags: CloneFlagsInternal::default(),
            sender: None,
            skip_owner_validation: false,
            bypass_certificate_check: false,
        }
    }

    // getters

    /// Returns true if owner validation is to be skipped, false otherwise.
    pub fn get_skip_owner_validation(&self) -> bool {
        return self.skip_owner_validation;
    }

    /// Returns the url set for the repository to be cloned.
    pub fn get_url(&self) -> &str {
        return &self.url;
    }

    /// Returns true if certification checks are to be bypassed, false otherwise.
    pub fn get_bypass_certificate_check(&self) -> bool {
        return self.bypass_certificate_check;
    }

    /// Returns the consumer (Receiver) end of a mpsc. Intended to receive
    /// git cli like update messages with an associated index.
    pub fn get_update_channel(&mut self) -> mpsc::Receiver<(usize, String)> {
        let (sender, receiver) = mpsc::channel();
        self.sender = Some(sender);
        return receiver;
    }

    /// Returns the directory where the repository is to be cloned.
    pub fn get_parent_path(&self) -> PathBuf {
        return self.parent_path.clone();
    }

    /// Returns the name of the directory where the content of the repository will
    /// be. The final path of the repository will be the parent_path/clone_dir_name/
    pub fn get_clone_dir_name(&self) -> String {
        if self.flags.bare {
            let mut dir = self.clone_dir_name.clone();
            dir = dir + ".git";
            return dir;
        }
        return self.clone_dir_name.clone();
    }

    // setters

    /// Set true to skip owner validation.
    pub fn skip_owner_validation(&mut self, skip: bool) {
        self.skip_owner_validation = skip;
    }

    /// Set true to skip certification checks.
    pub fn bypass_certificate_check(&mut self, bypass: bool) {
        self.bypass_certificate_check = bypass;
    }

    /// Set a custom name for the cloned repository. This will only change
    /// the name of the directory that will be formed, parent path will still
    /// be where the directory is created.
    pub fn custom_clone_directory(&mut self, dir: impl Into<String>) {
        self.clone_dir_name = dir.into();
    }

    /// "Pass" a flag to the git clone command.
    pub fn add_flag(&mut self, flag: CloneFlags) {
        match flag {
            CloneFlags::Branch(branch) => self.flags.branch = branch,
            CloneFlags::Depth(depth) => self.flags.depth = depth,
            CloneFlags::SingleBranch(single) => self.flags.single_branch = single,
            CloneFlags::Bare(bare) => self.flags.bare = bare,
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub(crate) struct CloneFlagsInternal {
    pub(crate) branch: Option<String>, // Branch(String),
    pub(crate) depth: Option<usize>,   // Depth(NonZeroUsize),
    pub(crate) single_branch: bool,    // SingleBranch,
    pub(crate) bare: bool,
}

impl Default for CloneFlagsInternal {
    fn default() -> Self {
        CloneFlagsInternal {
            branch: None,
            depth: None,
            single_branch: false,
            bare: false,
        }
    }
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
