use crate::GitRepository;
use git2::Error;
use std::path::Path;

pub struct AddConfig {
    spec: Vec<String>,
    flags: AddFlagsInternals,
}

impl AddConfig {
    pub fn new(spec: Vec<String>) -> Self {
        AddConfig {
            spec,
            flags: AddFlagsInternals::default(),
        }
    }

    pub fn get_specs(&self) -> &Vec<String> {
        &self.spec
    }

    pub fn add_flag(&mut self, flag: AddFlags) -> &Self {
        match flag {
            AddFlags::Update(update) => self.flags.update = update,
            AddFlags::DryRun(dry) => self.flags.dry_run = dry,
        }
        self
    }
}

#[derive(Default, Clone, Copy)]
pub(crate) struct AddFlagsInternals {
    update: bool,
    dry_run: bool,
}

pub enum AddFlags {
    Update(bool),
    DryRun(bool),
}

impl GitRepository {
    pub fn git_add(&self, config: AddConfig) -> Result<(), Error> {
        unsafe {
            git2::opts::set_verify_owner_validation(self.skip_owner_validation)?;
        };

        if let Some(repository) = &self.repository {
            let mut index = repository.index()?;

            let dry = config.flags.dry_run;
            let callback = &mut |path: &Path, _matched_spec: &[u8]| -> i32 {
                let status = repository.status_file(path).unwrap();

                let ret = if status.contains(git2::Status::WT_MODIFIED)
                    || status.contains(git2::Status::WT_NEW)
                    || status.contains(git2::Status::WT_DELETED)
                {
                    0
                } else {
                    1
                };

                if dry { 1 } else { ret }
            };

            let callback = if config.flags.update {
                Some(callback as &mut git2::IndexMatchedPath)
            } else {
                None
            };

            if config.flags.update {
                index.update_all(config.spec.iter(), callback)?;
            } else {
                index.add_all(config.spec.iter(), git2::IndexAddOption::DEFAULT, callback)?;
            }
            index.write()?;

            return Ok(());
        }

        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}
