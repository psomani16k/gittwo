use crate::GitRepository;
use git2::{Error, Repository, RepositoryInitOptions};
use std::path::{Path, PathBuf};

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

    pub fn get_dir(&self) -> &Path {
        &self.dir
    }

    pub fn set_dir(&mut self, dir: &Path) {
        self.dir = dir.to_path_buf();
    }

    pub fn add_flag(&mut self, flag: InitFlags) -> &Self {
        match flag {
            InitFlags::InitialBranch(branch) => self.flags.initial_branch = branch,
            InitFlags::Bare(bare) => self.flags.bare = bare,
            InitFlags::SeparateGitDir(path) => self.flags.separate_git_dir = Some(path),
        };
        self
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct InitFlagsInternal {
    initial_branch: Option<String>,
    bare: bool,
    separate_git_dir: Option<PathBuf>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InitFlags {
    InitialBranch(Option<String>),
    Bare(bool),
    SeparateGitDir(PathBuf),
}

impl GitRepository {
    pub fn git_init(&mut self, config: InitConfig) -> Result<(), Error> {
        unsafe {
            git2::opts::set_verify_owner_validation(self.skip_owner_validation)?;
        };
        let mut init_opts = RepositoryInitOptions::new();

        init_opts.bare(config.flags.bare);
        if let Some(branch) = config.flags.initial_branch {
            init_opts.initial_head(&branch);
        }

        if let Some(path) = config.flags.separate_git_dir {
            init_opts.workdir_path(&path);
        }

        let repository = Repository::init_opts(config.dir, &init_opts)?;
        self.repository = Some(repository);
        Ok(())
    }
}

#[cfg(test)]
mod init_test {
    use std::{path::Path, process::Command};

    use crate::GitRepository;

    use super::{InitConfig, InitFlags};

    #[test]
    fn git_init_initial_branch_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/init_initial_branch/"])
            .output()
            .unwrap();

        // creating an empty repository with initial branch
        let mut repo = GitRepository::new();
        let mut config = InitConfig::new(Path::new("./temp_test/init_initial_branch/"));
        config.add_flag(InitFlags::InitialBranch(Some(String::from("test"))));
        repo.git_init(config).unwrap();

        // verifying the repository is formed and has correct branch
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/init_initial_branch/",
                "branch",
                "--show-current",
            ])
            .output()
            .unwrap();

        // delete the repository
        Command::new("rm")
            .args(["-rf", "./temp_test/init_initial_branch/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "test\n");
    }
}
