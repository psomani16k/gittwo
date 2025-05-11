use std::fmt::format;

use git2::{CertificateCheckStatus, Error, PushOptions, RemoteCallbacks};

use crate::GitRepository;

pub struct PushConfig {
    remote: Option<String>,
    branch: Option<String>,
    flags: PushFlagsInternal,
}

impl PushConfig {
    pub fn new() -> Self {
        Self {
            remote: None,
            branch: None,
            flags: PushFlagsInternal::default(),
        }
    }

    pub fn set_remote_and_branch(&mut self, remote: Option<String>, branch: Option<String>) {
        self.remote = remote;
        self.branch = branch;
    }

    pub fn add_flag(&mut self, flag: PushFlags) {
        match flag {
            PushFlags::SetUpstream(set) => self.flags.set_upstream = set,
        }
    }
}

#[derive(Default)]
pub(crate) struct PushFlagsInternal {
    set_upstream: bool,
}

pub enum PushFlags {
    SetUpstream(bool),
}

impl GitRepository {
    pub fn git_push(&self, config: PushConfig) -> Result<(), Error> {
        // skip user verification if configured so
        if self.skip_owner_validation {
            unsafe {
                git2::opts::set_verify_owner_validation(false)?;
            };
        }

        // if the repository is valid
        if let Some(repository) = &self.repository {
            let mut callbacks = RemoteCallbacks::new();
            let mut options = PushOptions::new();
            let mut remote =
                repository.find_remote(&config.remote.unwrap_or(String::from("origin")))?;

            // continue even if cert checks fail, if configured so
            if self.bypass_certificate_check {
                callbacks.certificate_check(|_, _| Ok(CertificateCheckStatus::CertificateOk));
            }

            let cred = self.cred.clone();

            callbacks.credentials(move |_a: &str, _b, _c| cred.get_cred());

            remote.connect_auth(git2::Direction::Push, Some(callbacks), None)?;


            return Ok(());
        }
        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}
