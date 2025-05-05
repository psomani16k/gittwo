use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::mpsc,
};

pub struct CloneConfig {
    pub(crate) parent_path: PathBuf,
    pub(crate) url: String,
    pub(crate) sender: Option<mpsc::Sender<(usize, String)>>,
    pub(crate) flags: CloneFlagsInternal,
    pub(crate) skip_owner_validation: bool,
    pub(crate) bypass_certificate_check: bool,
}

impl CloneConfig {
    pub fn new(url: String, parent_dir: &Path) -> Self {
        CloneConfig {
            parent_path: parent_dir.to_path_buf(),
            url,
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

    // setters
    pub fn skip_owner_validation(&mut self, skip: bool) {
        self.skip_owner_validation = skip;
    }

    pub fn bypass_certificate_check(&mut self, bypass: bool) {
        self.bypass_certificate_check = bypass;
    }

    pub fn add_flag(&mut self, flag: CloneFlags) {
        match flag {
            CloneFlags::Branch(branch) => self.flags.branch = Some(branch),
            CloneFlags::Depth(depth) => self.flags.depth = Some(depth),
            CloneFlags::SingleBranch => self.flags.single_branch = Some(()),
            CloneFlags::Bare(bare) => self.flags.bare = bare,
        }
    }
}

pub(crate) struct CloneFlagsInternal {
    pub(crate) branch: Option<String>,      // Branch(String),
    pub(crate) depth: Option<NonZeroUsize>, // Depth(NonZeroUsize),
    pub(crate) single_branch: Option<()>,   // SingleBranch,
    pub(crate) bare: bool,
}

impl Default for CloneFlagsInternal {
    fn default() -> Self {
        CloneFlagsInternal {
            branch: None,
            depth: None,
            single_branch: None,
            bare: false,
        }
    }
}

pub enum CloneFlags {
    Branch(String),
    Depth(NonZeroUsize),
    SingleBranch,
    Bare(bool),
}
