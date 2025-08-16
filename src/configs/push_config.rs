use crate::GitRepository;
use git2::{BranchType, CertificateCheckStatus, Error, PushOptions, RemoteCallbacks};

#[derive(Default, Clone)]
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

    pub fn with_remote_and_branch(remote: Option<String>, branch: Option<String>) -> Self {
        Self {
            remote,
            branch,
            flags: PushFlagsInternal::default(),
        }
    }

    pub fn set_remote_and_branch(&mut self, remote: Option<String>, branch: Option<String>) {
        self.remote = remote;
        self.branch = branch;
    }

    pub fn add_flag(&mut self, flag: PushFlags) -> &Self {
        match flag {
            PushFlags::SetUpstream(set) => self.flags.set_upstream = set,
            PushFlags::All(all) => self.flags.all = all,
        };
        self
    }
}

#[derive(Default, Clone)]
pub(crate) struct PushFlagsInternal {
    set_upstream: bool,
    all: bool,
}

pub enum PushFlags {
    SetUpstream(bool),
    All(bool),
}

impl GitRepository {
    pub fn git_push(&self, config: PushConfig) -> Result<(), Error> {
        // if the repository is valid
        if let Some(repository) = &self.repository {
            // skip user verification if configured so
            if self.skip_owner_validation {
                unsafe {
                    git2::opts::set_verify_owner_validation(false)?;
                };
            }

            let remote_name = config.remote;
            let remote_branch_name = config.branch;

            let mut callbacks = RemoteCallbacks::new();
            let mut options = PushOptions::new();
            let mut remote = match &remote_name {
                Some(rem) => repository.find_remote(rem)?,
                None => repository.find_remote("origin")?,
            };

            // continue even if cert checks fail, if configured so
            if self.bypass_certificate_check {
                callbacks.certificate_check(|_, _| Ok(CertificateCheckStatus::CertificateOk));
            }

            // setup credentials
            let cred = self.cred.clone();
            callbacks.credentials(move |_a: &str, _b, _c| cred.get_cred());

            options.remote_callbacks(callbacks);

            let branch = repository.head()?;
            let src_branch = match branch.name() {
                Some(branch) => branch,
                None => {
                    return Err(Error::from_str(
                        "Could not resolve the reference pointed by HEAD",
                    ));
                }
            };

            let dest_branch = match &remote_branch_name {
                Some(branch) => format!("refs/heads/{}", branch),
                None => {
                    let dest = repository.branch_upstream_remote(src_branch)?;
                    dest.as_str().unwrap_or(src_branch).to_string()
                }
            };

            let refspec = format!("{}:{}", src_branch, dest_branch);

            let mut refspec = vec![refspec];
            // +-------+
            // | FLAGS |
            // +-------+

            // set-upstream
            if config.flags.set_upstream {
                if let (Some(remote_name), Some(branch_name)) = (&remote_name, &remote_branch_name)
                {
                    let mut branch = repository
                        .find_branch(branch.shorthand().unwrap(), git2::BranchType::Local)?;

                    let rem = format!("{}/{}", remote_name, branch_name);
                    branch.set_upstream(Some(&rem))?;
                } else {
                    return Err(Error::from_str(
                        "The current branch has no upstream branch, please provide remote and branch to PushConfig",
                    ));
                }
            }

            // all
            if config.flags.all {
                let branches = repository.branches(None)?;
                refspec = vec![];
                for branch in branches {
                    let branch = branch.unwrap();
                    if branch.1 == BranchType::Local {
                        let local_branch = branch.0;
                        let remote_branch = local_branch.upstream()?;
                        let local_branch = String::from_utf8_lossy(local_branch.name_bytes()?);
                        let remote_branch = String::from_utf8_lossy(remote_branch.name_bytes()?);
                        let spec = format!("{}:{}", local_branch, remote_branch);
                        refspec.push(spec);
                    }
                }
            }

            // +------+
            // | PUSH |
            // +------+

            remote.push(&refspec, Some(&mut options))?;

            return Ok(());
        }
        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}
