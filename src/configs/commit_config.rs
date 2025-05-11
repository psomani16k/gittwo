use std::path::Path;

use crate::GitRepository;
use git2::{Error, Repository, Signature, StatusOptions};

pub struct CommitConfig {
    name: String,
    email: String,
    flags: CommitFlagsInternals,
}

impl CommitConfig {
    pub fn new(name: String, email: String) -> Self {
        CommitConfig {
            name,
            email,
            flags: CommitFlagsInternals::default(),
        }
    }

    pub fn with_message(name: String, email: String, message: String) -> Self {
        let mut config = CommitConfig {
            name,
            email,
            flags: CommitFlagsInternals::default(),
        };
        config.add_flags(CommitFlags::Message(message));
        return config;
    }
    pub fn add_flags(&mut self, flag: CommitFlags) {
        match flag {
            CommitFlags::Message(msg) => self.flags.message = msg,
            CommitFlags::AllowEmptyMessage(allow) => self.flags.allow_empty_message = allow,
        }
    }

    fn get_signature(&self) -> Result<Signature, Error> {
        Signature::now(&self.name, &self.email)
    }
}

#[derive(Default, Clone)]
pub(crate) struct CommitFlagsInternals {
    message: String,
    allow_empty_message: bool,
}

pub enum CommitFlags {
    Message(String),
    AllowEmptyMessage(bool),
}

impl GitRepository {
    pub fn git_commit(&self, config: CommitConfig) -> Result<(), Error> {
        unsafe {
            git2::opts::set_verify_owner_validation(self.skip_owner_validation)?;
        };
        if let Some(repository) = &self.repository {
            if !config.flags.allow_empty_message && config.flags.message.is_empty() {
                return Err(Error::from_str(
                    "Aborting commit due to empty commit message.",
                ));
            }
            if !GitRepository::has_indexed_files(&repository) {
                return Ok(());
            }

            // if config.flags.message == "" && !config.flags.allow_empty_message {}

            let mut index = repository.index()?;
            let tree = index.write_tree()?;
            let tree = repository.find_tree(tree)?;
            let signature = config.get_signature()?;

            if let Ok(parent_commit) = repository.head() {
                repository.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    &config.flags.message,
                    &tree,
                    &[&parent_commit.peel_to_commit().unwrap()],
                )?;
            } else {
                repository.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    &config.flags.message,
                    &tree,
                    &[],
                )?;
            }

            return Ok(());
        }
        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }

    fn has_indexed_files(repo: &Repository) -> bool {
        let mut opts = StatusOptions::new();
        opts.include_ignored(false);
        opts.include_untracked(true).recurse_untracked_dirs(true);
        opts.exclude_submodules(true);

        let statuses = repo.statuses(Some(&mut opts)).unwrap();
        let statuses = statuses.iter();
        for i in statuses {
            let file_status = i.status();
            if file_status.is_index_renamed()
                || file_status.is_index_modified()
                || file_status.is_index_new()
                || file_status.is_index_deleted()
                || file_status.is_index_typechange()
            {
                return true;
            }
        }
        return false;
    }

    pub fn can_commit(repo_dir: &str) -> bool {
        unsafe {
            let _ = git2::opts::set_verify_owner_validation(false);
        };
        let repo = Repository::open(Path::new(&repo_dir)).unwrap();
        let mut opts = StatusOptions::new();
        opts.include_ignored(false);
        opts.include_untracked(true).recurse_untracked_dirs(true);
        opts.exclude_submodules(true);

        let statuses = repo.statuses(Some(&mut opts)).unwrap();
        let statuses = statuses.iter();
        for i in statuses {
            let file_status = i.status();
            if file_status.is_index_renamed()
                || file_status.is_index_modified()
                || file_status.is_index_new()
                || file_status.is_index_deleted()
                || file_status.is_index_typechange()
            {
                return true;
            }
        }
        return false;
    }
}
