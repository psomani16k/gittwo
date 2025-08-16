use crate::GitRepository;
use git2::{CertificateCheckStatus, Error, RemoteCallbacks, build::CheckoutBuilder};

#[derive(Clone)]
pub struct CheckoutConfig {
    pub(crate) spec: String,
    pub(crate) flags: CheckoutFlagsInternal,
}

impl CheckoutConfig {
    pub fn new(spec: String) -> Self {
        CheckoutConfig {
            spec: spec,
            flags: CheckoutFlagsInternal::default(),
        }
    }

    pub fn add_flags(&mut self, flag: CheckoutFlags) {
        match flag {}
    }
}

#[derive(Default, Clone)]
pub(crate) struct CheckoutFlagsInternal {}

pub enum CheckoutFlags {}

impl GitRepository {
    pub fn git_checkout(&self, config: CheckoutConfig) -> Result<(), Error> {
        if let Some(repository) = &self.repository {
            // skip user verification if configured so
            if self.skip_owner_validation {
                unsafe {
                    git2::opts::set_verify_owner_validation(false)?;
                };
            }

            // prepare checkout
            let mut checkout_builder = CheckoutBuilder::new();

            // trying locally present branch
            match repository.find_branch(&config.spec, git2::BranchType::Local) {
                Ok(local_branch) => {
                    let reference = local_branch.get();
                    let name = match reference.name() {
                        Some(name) => name,
                        None => &config.spec,
                    };
                    repository.set_head(name)?;
                    checkout_builder.safe();
                    repository.checkout_head(Some(&mut checkout_builder))?;
                    return Ok(());
                }
                Err(_) => {}
            };

            // trying locally present tags
            let tag = format!("refs/tags/{}", &config.spec);
            match repository.find_reference(&tag) {
                Ok(tag) => {
                    let name = match tag.name() {
                        Some(name) => name,
                        None => &config.spec,
                    };
                    repository.set_head(name)?;
                    checkout_builder.safe();
                    repository.checkout_head(Some(&mut checkout_builder))?;
                    return Ok(());
                }
                Err(_) => {}
            };

            // trying remote branches and tags
            let remotes = repository.remotes()?;
            for remote in &remotes {
                if let Some(remote) = remote {
                    let mut remote = repository.find_remote(remote)?;
                    let mut callback = RemoteCallbacks::new();
                    // continue even if cert checks fail, if configured so
                    if self.bypass_certificate_check {
                        callback
                            .certificate_check(|_, _| Ok(CertificateCheckStatus::CertificateOk));
                    }
                    callback.credentials(move |_a: &str, _b, _c| self.cred.get_cred());
                    remote.connect_auth(git2::Direction::Fetch, Some(callback), None)?;
                    if let Ok(remote_heads) = remote.list() {
                        let branch_full = format!("refs/heads/{}", &config.spec);
                        for remote_head in remote_heads {
                            if branch_full == remote_head.name() {
                                let target_commit = remote_head.oid();
                                let target_commit = repository.find_commit(target_commit)?;
                                let mut remote = remote.clone();
                                let refspec = format!(
                                    "{}:refs/remotes/{}/{}",
                                    branch_full,
                                    remote.name().unwrap(),
                                    &config.spec
                                );
                                remote.fetch(&[refspec], None, None)?;
                                let mut local_branch =
                                    repository.branch(&config.spec, &target_commit, false)?;
                                let upstream =
                                    format!("{}/{}", remote.name().unwrap(), &config.spec);
                                local_branch.set_upstream(Some(&upstream))?;
                                repository.set_head(&branch_full)?;
                                checkout_builder.safe();
                                repository.checkout_head(Some(&mut checkout_builder))?;
                                return Ok(());
                            }

                            let tag_full = format!("refs/tags/{}", &config.spec);
                            if tag_full == remote_head.name() {
                                let tag_ref = format!("{}:{}", tag_full, tag_full);
                                let mut remote = remote.clone();
                                remote.fetch(&[tag_ref], None, None)?;
                                let reference = repository.find_reference(&tag_full)?;
                                let name = match reference.name() {
                                    Some(name) => name,
                                    None => &config.spec,
                                };
                                repository.set_head(name)?;
                                checkout_builder.safe();
                                repository.checkout_head(Some(&mut checkout_builder))?;
                                return Ok(());
                            }
                        }
                    }
                    remote.disconnect();
                }
            }

            // checkout to local branch

            // try commits
            match repository.revparse_single(&config.spec) {
                Ok(obj) => {
                    repository.set_head_detached(obj.id())?;
                    repository.checkout_tree(&obj, Some(&mut checkout_builder))?;
                    return Ok(());
                }
                Err(_) => {}
            }

            let err_msg = format!("Failed to resolve spec: {}", &config.spec);
            return Err(Error::from_str(&err_msg));
        }

        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}

#[cfg(test)]
mod checkout_test {
    use std::{env, path::Path, process::Command};

    use crate::{GitRepository, configs::checkout_config::CheckoutConfig};

    #[test]
    fn checkout() {
        let dir_name = "./temp_test/checkout/";

        // create temp directories
        Command::new("mkdir")
            .args(["-p", dir_name])
            .output()
            .unwrap();

        // clone a git repository
        let _ = Command::new("git")
            .args([
                "-C",
                dir_name,
                "clone",
                "https://github.com/rust-lang/git2-rs.git",
            ])
            .output()
            .expect("git cli needs to be installed for comparing test results");

        // ----------------------
        // 1. CHECKING OUT BRANCH
        // ----------------------

        // perform an action to checkout a branch similar to
        // "git checkout curl"
        let path = env::current_dir().unwrap();
        let dir_name = dir_name.to_owned() + "./git2-rs";
        let path = path.join(&dir_name);
        let repo = GitRepository::open(Path::new(&path)).unwrap();
        let checkout_config = CheckoutConfig::new("curl".to_string());
        repo.git_checkout(checkout_config).unwrap();
        // verify the above actions.
        let out_1 = Command::new("git")
            .args(["-C", &dir_name, "branch"])
            .output()
            .expect("git cli needs to be installed for comparing test results");

        // -------------------
        // 2. CHECKING OUT TAG
        // -------------------

        // perform an action to checkout a branch similar to
        // "git checkout libgit2-sys-0.14.2"
        let checkout_config = CheckoutConfig::new("libgit2-sys-0.14.2".to_string());
        repo.git_checkout(checkout_config).unwrap();

        // verify the above actions.
        let out_2 = Command::new("git")
            .args(["-C", &dir_name, "branch"])
            .output()
            .expect("git cli needs to be installed for comparing test results");

        // ----------------------
        // 3. CHECKING OUT COMMIT
        // ----------------------

        // perform an action to checkout a branch similar to
        // "git checkout d1b40aa"
        let checkout_config = CheckoutConfig::new("d1b40aa".to_string());
        repo.git_checkout(checkout_config).unwrap();

        // verify the above actions.
        let out_3 = Command::new("git")
            .args(["-C", &dir_name, "branch"])
            .output()
            .expect("git cli needs to be installed for comparing test results");

        // ---------------------------
        // 4. CHECKING OUT nonExistant
        // ---------------------------

        // perform an action to checkout a branch similar to
        // "git checkout nonExistant"
        let checkout_config = CheckoutConfig::new("nonExistant".to_string());
        let out_4 = repo.git_checkout(checkout_config);

        // clean up
        Command::new("rm")
            .args(["-rf", &dir_name])
            .output()
            .unwrap();

        assert_eq!(
            String::from_utf8_lossy(&out_1.stdout),
            "* curl
  master\n"
        );

        assert_eq!(
            String::from_utf8_lossy(&out_2.stdout),
            "* (HEAD detached at libgit2-sys-0.14.2)
  curl
  master\n"
        );

        assert_eq!(
            String::from_utf8_lossy(&out_3.stdout),
            "* (HEAD detached at d1b40aa)
  curl
  master\n"
        );

        assert!(out_4.is_err());
    }
}
