use git2::{
    AutotagOption, CertificateCheckStatus, Error, FetchOptions, RemoteCallbacks, RemoteUpdateFlags,
};

use crate::GitRepository;

pub struct FetchConfig {
    remote: Option<String>,
    flags: FetchFlagsInternal,
}

impl FetchConfig {
    pub fn new(remote: Option<String>) -> Self {
        Self {
            remote,
            flags: FetchFlagsInternal::default(),
        }
    }

    pub fn add_flag(&mut self, flag: FetchFlags) {
        match flag {
            FetchFlags::Unshallow(unshallow) => self.flags.unshallow = unshallow,
        }
    }
}

#[derive(Default)]
pub(crate) struct FetchFlagsInternal {
    unshallow: bool,
}

pub enum FetchFlags {
    Unshallow(bool),
}

impl GitRepository {
    pub fn git_fetch(&self, config: FetchConfig) -> Result<(), Error> {
        if let Some(repository) = &self.repository {
            let mut callbacks = RemoteCallbacks::new();
            let mut fetch_options = FetchOptions::new();
            callbacks.credentials(move |_a: &str, _b, _c| self.cred.get_cred());

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

            let remote = if let Some(remote) = &config.remote {
                remote.to_string()
            } else if let Some(branch) = repository.head()?.shorthand() {
                let config = repository.config()?;
                let rem = config.get_str(&format!("branch.{}.remote", branch));
                let remote = if let Ok(rem) = rem {
                    rem.to_string()
                } else {
                    String::from("origin")
                };
                remote
            } else {
                String::from("origin")
            };

            let mut remote = repository.find_remote(&remote)?;

            remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None)?;

            // unshallow
            if config.flags.unshallow {
                fetch_options.depth(2147483647);
            }
            remote.download::<&str>(&[], Some(&mut fetch_options))?;
            remote.disconnect()?;
            remote.update_tips(
                None,
                RemoteUpdateFlags::UPDATE_FETCHHEAD,
                AutotagOption::Auto,
                None,
            )?;
            return Ok(());
        }

        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}
