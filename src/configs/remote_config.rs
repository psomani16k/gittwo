use std::fmt::Display;

use crate::GitRepository;
use git2::Error;

/// A struct used to specify various details about the `git remote` command.
pub struct RemoteConfig {
    subcommand: Option<RemoteSubCommand>,
    flags: RemoteFlagsInternal,
}

impl RemoteConfig {
    /// Creates a RemoteConfig object with the passed subcommand.
    ///
    /// Example of remote config for adding the github repository of git to a current repository.
    /// ```ignore
    /// let remote_name = "origin".to_string();
    /// let remote_url = "https://github.com/git/git.git".to_string();
    /// let remote_config = RemoteConfig::new(Some(RemoteSubCommand::Add(remote_name,
    /// remote_url)));
    /// ```
    pub fn new(subcommand: Option<RemoteSubCommand>) -> Self {
        Self {
            subcommand,
            flags: RemoteFlagsInternal::default(),
        }
    }

    /// Set a subcommand to the config.
    /// Pass `Option::None` to unset any previously set subcommands.
    ///
    /// *NOTE:* setting a subcommand will RESET any flags applied previously.
    pub fn set_subcommand(&mut self, subcommand: Option<RemoteSubCommand>) {
        self.subcommand = subcommand;
        self.flags = RemoteFlagsInternal::default();
    }

    /// Set a flag to the config.
    /// Sets the flag if it is valid for a given subcommand, else returns an error.
    pub fn add_flag(&mut self, flag: RemoteFlags) -> Result<(), git2::Error> {
        let error = format!("No flag '{}' for subcommand '{:?}'.", flag, self.subcommand);
        let error = git2::Error::from_str(&error);
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                // only add flags available for a given subcommand, return error if flag and
                // subcommands do not match.
                RemoteSubCommand::Add(_, _) => match flag {
                    RemoteFlags::Track(tracking_branches) => {
                        self.flags.track = Some(tracking_branches)
                    }
                    _ => return Err(error),
                },
                RemoteSubCommand::SetHead(_, _) => match flag {
                    RemoteFlags::Delete(delete) => {
                        self.flags.delete = delete;
                    }
                    _ => return Err(error),
                },
                RemoteSubCommand::Remove(_) => return Err(error),
            };
        } else {
            match flag {
                _ => return Err(error),
            };
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum RemoteSubCommand {
    /// Set the `add` subcommand to the RemoteConfig.
    /// Takes two String inputs of remote name and remote url in this order.
    Add(String, String),

    /// Set the `remove` subcommand to the RemoteConfig.
    /// Takes the name of the remote to be removed as the input.
    Remove(String),

    /// Set the `set-head` subcommand to the RemoteConfig
    /// Takes two inputs of remote name and an optional branch in this order.
    /// The optional branch field can only be empty only if delete flag is set, else it will throw
    /// an error.
    SetHead(String, Option<String>),
}

#[derive(Default)]
pub(crate) struct RemoteFlagsInternal {
    track: Option<Vec<String>>,
    delete: bool,
}

#[derive(Clone, Debug)]
pub enum RemoteFlags {
    /// `-t <branch>` or `--track <branch>` flag for `git remote add`.
    /// Pass in all the branch names into the vector for tracking.
    /// Passing an empty vector unsets the flag.
    Track(Vec<String>),

    /// `-d` or `--delete` flag for `git remote set-head`.
    /// Pass in true to set the flag and false to unset it.
    /// Defaults to false.
    Delete(bool),
}

impl Display for RemoteFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteFlags::Track(items) => write!(f, "--track {:?}", items),
            RemoteFlags::Delete(delete) => write!(f, "--delete {}", delete),
        }
    }
}

impl GitRepository {
    pub fn git_remote(&self, config: RemoteConfig) -> Result<(), Error> {
        if let Some(repository) = &self.repository {
            if let Some(subcommand) = &config.subcommand {
                match subcommand {
                    RemoteSubCommand::Add(name, url) => {
                        // git remote add
                        repository.remote(name, url)?;

                        // -t flag
                        if let Some(t) = &config.flags.track {
                            let mut config = repository.config()?;
                            let key = format!("remote.{}.fetch", name);
                            config.remove_multivar(&key, ".*")?;
                            for branch in t {
                                let spec = format!(
                                    "+refs/heads/{}:refs/remotes/{}/{}",
                                    branch, name, branch
                                );
                                repository.remote_add_fetch(name, &spec)?;
                            }
                        }
                    }
                    RemoteSubCommand::Remove(name) => {
                        // git remote remove
                        repository.remote_delete(name)?;
                    }
                    RemoteSubCommand::SetHead(remote, branch) => {
                        // git remote set-head
                        if !config.flags.delete && branch.is_some() {
                            let name = format!("refs/remote/{}/HEAD", remote);
                            let branch = branch.clone().unwrap();
                            let target = format!("refs/remote/{}/{}", remote, branch);
                            repository.reference_symbolic(
                                &name,
                                &target,
                                true,
                                "set remote HEAD",
                            )?;
                        } else if config.flags.delete {
                            let name = format!("refs/remote/{}/HEAD", remote);
                            match repository.find_reference(&name) {
                                Ok(mut reference) => {
                                    reference.delete()?;
                                }
                                Err(ref e) if e.code() == git2::ErrorCode::NotFound => {}
                                Err(e) => return Err(e),
                            };
                        }
                    }
                }
            } else {
                // git remote
                todo!();
            }

            return Ok(());
        }

        Err(Error::from_str(
            "Repository not found or created, try opening a valid repository or cloning one",
        ))
    }
}

#[cfg(test)]
mod remote_test {
    use std::{env, path::Path, process::Command};

    use crate::{
        GitRepository,
        configs::remote_config::{RemoteConfig, RemoteFlags, RemoteSubCommand},
    };

    #[test]
    fn git_remote_add_track_test() {
        let dir_name = "./temp_test/remote_add_track/";

        // create temp directories
        Command::new("mkdir")
            .args(["-p", dir_name])
            .output()
            .unwrap();

        // create a empty repository
        let _ = Command::new("git")
            .args(["-C", dir_name, "init"])
            .output()
            .expect("git cli needs to be installed for comparing test results");

        // add a perform an action same as
        // "git remote add test https://github.com/rust-lang/git2-rs.git --track curl"
        let path = env::current_dir().unwrap();
        let path = path.join(dir_name);
        let repo = GitRepository::open(Path::new(&path)).unwrap();
        let mut remote_config = RemoteConfig::new(Some(RemoteSubCommand::Add(
            "test".to_string(),
            "https://github.com/rust-lang/git2-rs.git".to_string(),
        )));
        remote_config
            .add_flag(RemoteFlags::Track(vec!["curl".to_string()]))
            .unwrap();

        repo.git_remote(remote_config).unwrap();

        // verify the above actions.
        let out = Command::new("git")
            .args(["-C", dir_name, "remote", "show", "test"])
            .output()
            .expect("git cli needs to be installed for comparing test results");

        Command::new("rm").args(["-rf", dir_name]).output().unwrap();

        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "* remote test
  Fetch URL: https://github.com/rust-lang/git2-rs.git
  Push  URL: https://github.com/rust-lang/git2-rs.git
  HEAD branch: master
  Remote branch:
    curl new (next fetch will store in remotes/test)
"
        );
    }
}
