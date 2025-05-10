use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use git2::{Error, Repository, RepositoryInitOptions};

use crate::GitRepository;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InitConfig {
    dir: PathBuf,
    flags: InitFlagsInternal,
}

impl InitConfig {
    pub fn new(dir: &Path) -> Self {
        InitConfig {
            dir: dir.to_path_buf(),
            flags: InitFlagsInternal::default(),
        }
    }

    pub fn get_dir(&self) -> PathBuf {
        self.dir.clone()
    }

    pub fn add_flags(&mut self, flag: InitFlags) {
        match flag {
            InitFlags::InitialBranch(branch) => self.flags.initial_branch = branch,
            InitFlags::Bare(bare) => self.flags.bare = bare,
        };
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InitFlagsInternal {
    initial_branch: Option<String>,
    bare: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InitFlags {
    InitialBranch(Option<String>),
    Bare(bool),
}

impl GitRepository {
    pub fn git_init(&mut self, config: InitConfig) -> Result<(), Error> {
        let mut init_opts = RepositoryInitOptions::new();

        init_opts.bare(config.flags.bare);
        if let Some(branch) = config.flags.initial_branch {
            init_opts.initial_head(&branch);
        }

        let _ = create_dir_all(&config.dir);
        let repository = Repository::init_opts(config.dir, &init_opts)?;
        self.repository = Some(repository);
        Ok(())
    }
}
