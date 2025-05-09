use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::mpsc,
};

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
    pub fn get_skip_owner_validation(&self) -> bool {
        return self.skip_owner_validation;
    }

    pub fn get_url(&self) -> &str {
        return &self.url;
    }

    pub fn get_bypass_certificate_check(&self) -> bool {
        return self.bypass_certificate_check;
    }

    pub fn get_update_channel(&mut self) -> mpsc::Receiver<(usize, String)> {
        let (sender, receiver) = mpsc::channel();
        self.sender = Some(sender);
        return receiver;
    }

    pub fn get_parent_path(&self) -> PathBuf {
        return self.parent_path.clone();
    }

    pub fn get_clone_dir_name(&self) -> String {
        if self.flags.bare {
            let mut dir = self.clone_dir_name.clone();
            dir = dir + ".git";
            return dir;
        }
        return self.clone_dir_name.clone();
    }

    // setters
    pub fn skip_owner_validation(&mut self, skip: bool) {
        self.skip_owner_validation = skip;
    }

    pub fn bypass_certificate_check(&mut self, bypass: bool) {
        self.bypass_certificate_check = bypass;
    }

    pub fn custom_clone_directory(&mut self, dir: impl Into<String>) {
        self.clone_dir_name = dir.into();
    }

    pub fn add_flag(&mut self, flag: CloneFlags) {
        match flag {
            CloneFlags::Branch(branch) => self.flags.branch = branch,
            CloneFlags::Depth(depth) => self.flags.depth = depth,
            CloneFlags::SingleBranch(single) => self.flags.single_branch = single,
            CloneFlags::Bare(bare) => self.flags.bare = bare,
        }
    }
}

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
