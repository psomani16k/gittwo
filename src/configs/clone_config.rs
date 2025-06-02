use crate::{GitRepository, helpers::channel::ChannelHelper};
use git2::{
    AutotagOption, CertificateCheckStatus, Error, FetchOptions, Remote, RemoteCallbacks,
    build::RepoBuilder,
};

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

#[cfg(not(feature = "tokio-channels"))]
use std::sync::mpsc as std_mpsc;

#[cfg(feature = "tokio-channels")]
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Clone)]
/// Specifies details for a `git clone` operation.
pub struct CloneConfig {
    pub(crate) clone_dir_name: String,
    pub(crate) parent_path: PathBuf,
    pub(crate) url: String,
    pub(crate) flags: CloneFlagsInternal,
    pub(crate) sender: Option<ChannelHelper<(usize, String)>>,
}

impl CloneConfig {
    /// Creates a new `CloneConfig` with the repository URL and the parent directory for the clone.
    ///
    /// A new directory, named after the repository (e.g., "gittwo" from
    /// "https://github.com/psomani16k/gittwo.git"), will be created inside `parent_dir`. The repository will be cloned into this new directory.
    pub fn new(url: String, parent_dir: &Path) -> Self {
        let target_dir: String = url;
        let url = target_dir.clone();
        let target_dir = target_dir.split("/").last().unwrap();
        let target_dir = match target_dir.strip_suffix(".git") {
            Some(t) => t,
            None => target_dir,
        };

        CloneConfig {
            clone_dir_name: target_dir.to_string(),
            parent_path: parent_dir.to_path_buf(),
            url,
            flags: CloneFlagsInternal::default(),
            sender: None,
        }
    }

    // getters

    /// Returns the URL of the repository to be cloned.
    pub fn get_url(&self) -> &str {
        &self.url
    }

    #[cfg(not(feature = "tokio-channels"))]
    /// Returns the receiver end of a multi-producer, single-consumer (mpsc) channel.
    /// This channel is used to receive progress updates during the clone operation,
    /// similar to Git CLI output, with an associated index of each line.
    ///
    /// NOTE: messages are send at the sender only if a receiver is initilized (by calling this
    /// function), messages will be sent regardless of whether they are received.
    /// sent regardless
    pub fn get_update_channel(&mut self) -> std_mpsc::Receiver<(usize, String)> {
        let (sender, receiver) = std_mpsc::channel();
        let sender = ChannelHelper::StdChannel(sender);
        self.sender = Some(sender);
        receiver
    }

    #[cfg(feature = "tokio-channels")]
    /// Returns the receiver end of an unbounded multi-producer, single-consumer (mpsc) channel from the tokio
    /// crate.
    /// This channel is used to receive progress updates during the clone operation,
    /// similar to Git CLI output, with an associated index of each line.
    ///
    /// NOTE: messages are send at the sender only if a receiver is initilized (by calling this
    /// function), messages will be sent regardless of whether they are received.
    /// sent regardless
    pub fn get_update_channel(&mut self) -> tokio_mpsc::UnboundedReceiver<(usize, String)> {
        let (sender, receiver) = tokio_mpsc::unbounded_channel();
        let sender = ChannelHelper::TokioChannel(sender);
        self.sender = Some(sender);
        receiver
    }

    /// Returns the parent directory where the repository will be cloned.
    pub fn get_parent_path(&self) -> &Path {
        &self.parent_path
    }

    /// Returns the name of the directory for the cloned repository.
    /// The full path will be `parent_path/clone_dir_name/`.
    pub fn get_clone_dir_name(&self) -> String {
        if self.flags.bare {
            let mut dir = self.clone_dir_name.clone();
            dir += ".git";
            return dir;
        }
        self.clone_dir_name.clone()
    }

    // setters

    /// Sets a custom name for the directory where the repository will be cloned.
    /// This overrides the default name derived from the repository URL.
    pub fn custom_clone_directory(&mut self, dir: impl Into<String>) {
        self.clone_dir_name = dir.into();
    }

    /// Configures a specific flag for the `git clone` operation.
    pub fn add_flag(&mut self, flag: CloneFlags) -> &Self {
        match flag {
            CloneFlags::Branch(branch) => self.flags.branch = branch,
            CloneFlags::Depth(depth) => self.flags.depth = depth,
            CloneFlags::SingleBranch(single) => self.flags.single_branch = single,
            CloneFlags::Bare(bare) => self.flags.bare = bare,
            CloneFlags::Recursive(rec) => self.flags.recursive = rec,
        }
        self
    }
}

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Internal representation of clone flags, not intended for direct public use.
pub(crate) struct CloneFlagsInternal {
    pub(crate) branch: Option<String>,
    pub(crate) depth: Option<usize>,
    pub(crate) single_branch: bool,
    pub(crate) bare: bool,
    pub(crate) recursive: Option<Vec<String>>,
}

/// Represents flags that can be applied to a `git clone` command.
/// See [git clone documentation](https://git-scm.com/docs/git-clone) for more details on each flag.
pub enum CloneFlags {
    /// Corresponds to the [`--branch`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt-code--branchcodeemltnamegtem)
    /// or `-b` flag.
    /// `Some(branch_name)` specifies the branch to checkout. `None` uses the remote's default branch.
    ///
    /// Defaults to `None`.
    Branch(Option<String>),

    /// Corresponds to the [`--depth <depth>`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt)
    /// flag.
    /// `Some(n)` creates a shallow clone with a history truncated to `n` commits. `None` implies a full clone.
    ///
    /// Defaults to `None`.
    Depth(Option<usize>),

    /// Corresponds to the [`--single-branch`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---single-branch)
    /// flag.
    /// `true` clones only the history leading to the tip of a single branch (either the one specified by `--branch` or the remote's default).
    /// `false` clones all branches.
    ///
    /// Defaults to `false`.
    SingleBranch(bool),

    /// Corresponds to the [`--bare`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---bare)
    /// flag.
    /// `true` creates a bare Git repository (no working directory). `false` creates a standard repository.
    ///
    /// Defaults to `false`.
    Bare(bool),

    /// Corresponds to the [`--recursive`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---recursive)
    /// or [`--recurse-submodules[=<pathspec>]`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---recurse-submodulesltpathspecgt) flag.
    /// `Some(pathspecs)` initializes submodules matching the pathspecs. An empty vector initializes all submodules.
    /// `None` does not initialize submodules.
    ///
    /// Defaults to `None`.
    Recursive(Option<Vec<String>>),
}

impl GitRepository {
    /// Clones a Git repository based on the provided `CloneConfig`.
    ///
    /// If GitRepository was created using `GitRepository::new()` this will allow you to clone a
    /// remote repository to the provided directory. If GitRepository was created using
    /// `GitRepository::open()` calling this function will return an error.
    pub fn git_clone(&mut self, config: CloneConfig) -> Result<(), Error> {
        if self.repository.is_some() {
            return Err(git2::Error::from_str(
                "git_clone() called on a pre-existing repository.",
            ));
        }

        let mut remote_update_index = 1;
        let mut transfer_update_index = 100;
        let mut progress_helper = ProgressCallbackHelper::default();
        let mut fetch_options = FetchOptions::new();
        let mut repo_builder = RepoBuilder::new();
        let mut callbacks = RemoteCallbacks::new();
        let mut callbacks2 = RemoteCallbacks::new();
        let mut remote = Remote::create_detached(config.url.clone())?;

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

        // setting up credentials
        let cred = self.cred.clone();
        let cred2 = self.cred.clone();
        callbacks.credentials(move |_a: &str, _b, _c| cred.get_cred());
        callbacks2.credentials(move |_a: &str, _b, _c| cred2.get_cred());

        let remote = remote.connect_auth(git2::Direction::Fetch, Some(callbacks2), None)?;
        let mut def_branch: Vec<u8> = vec![];
        remote.default_branch()?.clone_into(&mut def_branch);
        let def_branch = String::from_utf8(def_branch);
        let mut def_branch = def_branch.unwrap_or("main".to_string());
        def_branch = def_branch.split("/").last().unwrap_or("main").to_string();

        // getting the name of the repository
        let repo_path = config.get_parent_path().join(config.get_clone_dir_name());

        // +----------------------------+
        // | SETTING UP UPDATES CHANNEL |
        // +----------------------------+

        #[cfg(not(feature = "tokio-channels"))]
        if config.sender.is_some() {
            let sender = config.sender.clone().unwrap();
            let initial_msg = format!("Cloning into '{}'...", config.get_clone_dir_name());
            let _ = sender.send((0, initial_msg));
            callbacks.sideband_progress(move |stats| {
                remote_update_index =
                    ProgressCallbackHelper::update_remote(remote_update_index, stats, &sender);
                true
            });

            let sender = config.sender.clone().unwrap();

            callbacks.transfer_progress(move |stats| {
                if transfer_update_index == 100 {
                    let total_objects = stats.total_objects();
                    let received_objects = stats.received_objects();
                    let received_bytes = stats.received_bytes();
                    transfer_update_index = progress_helper.update_receiving(
                        received_objects,
                        total_objects,
                        received_bytes,
                        &sender,
                        transfer_update_index,
                    );
                } else if transfer_update_index == 101 {
                    let indexed_deltas = stats.indexed_deltas();
                    let total_deltas = stats.total_deltas();
                    transfer_update_index = progress_helper.update_resolving(
                        indexed_deltas,
                        total_deltas,
                        &sender,
                        transfer_update_index,
                    );
                }
                true
            });
        }

        #[cfg(feature = "tokio-channels")]
        if config.sender.is_some() {
            let sender = config.sender.clone().unwrap();
            let initial_msg = format!("Cloning into '{}'...", config.get_clone_dir_name());
            let _ = sender.send((0, initial_msg));
            callbacks.sideband_progress(move |stats| {
                remote_update_index =
                    ProgressCallbackHelper::update_remote(remote_update_index, stats, &sender);
                true
            });

            let sender = config.sender.clone().unwrap();

            callbacks.transfer_progress(move |stats| {
                if transfer_update_index == 100 {
                    let total_objects = stats.total_objects();
                    let received_objects = stats.received_objects();
                    let received_bytes = stats.received_bytes();
                    transfer_update_index = progress_helper.update_receiving(
                        received_objects,
                        total_objects,
                        received_bytes,
                        &sender,
                        transfer_update_index,
                    );
                } else if transfer_update_index == 101 {
                    let indexed_deltas = stats.indexed_deltas();
                    let total_deltas = stats.total_deltas();
                    transfer_update_index = progress_helper.update_resolving(
                        indexed_deltas,
                        total_deltas,
                        &sender,
                        transfer_update_index,
                    );
                }
                true
            });
        }

        // +---------------+
        // | SETTING FLAGS |
        // +---------------+

        // branch
        if let Some(branch) = &config.flags.branch {
            def_branch = branch.to_string();
        }

        // depth
        if let Some(depth) = config.flags.depth {
            let depth: i32 = depth as i32;
            fetch_options.depth(depth);
            fetch_options.download_tags(AutotagOption::None);
            let branch = def_branch.clone();
            repo_builder.remote_create(move |repo, name, url| {
                let refspec = format!("+refs/heads/{0:}:refs/remotes/origin/{0:}", branch);
                repo.remote_with_fetch(name, url, &refspec)
            });
        }

        // single-branch
        if config.flags.single_branch {
            fetch_options.download_tags(AutotagOption::None);
            let branch = def_branch.clone();
            repo_builder.remote_create(move |repo, name, url| {
                let refspec = format!("+refs/heads/{0:}:refs/remotes/origin/{0:}", branch);
                repo.remote_with_fetch(name, url, &refspec)
            });
        }

        // bare
        let repo_builder = repo_builder.bare(config.flags.bare);

        // +--------------+
        // | CLONING REPO |
        // +--------------+

        let repo_builder = repo_builder.branch(&def_branch);

        fetch_options.remote_callbacks(callbacks);

        // setting fetch options and cloning
        repo_builder.fetch_options(fetch_options);
        let repository = repo_builder.clone(config.get_url(), &repo_path)?;

        if let Some(pathspecs) = config.flags.recursive {
            if pathspecs.is_empty() {
                let submodule = repository.submodules()?;
                for mut sub in submodule {
                    // git seems to ignore errors in cloning submodule
                    // TODO: investigate this
                    let _ = sub.clone(None);
                }
            }
            for pathspec in pathspecs {
                if let Ok(mut submodule) = repository.find_submodule(&pathspec) {
                    // git seems to ignore errors in cloning submodule, hence doing the same
                    // here...
                    // TODO: investigate this
                    let _ = submodule.clone(None);
                }
            }
        }
        self.repository = Some(repository);

        Ok(())
    }
}

struct ProgressCallbackHelper {
    last_update_time: SystemTime,
    last_throughput_update_time: SystemTime,
    last_transfered_bytes: usize,
    previous_throughut: u128,
}

impl Default for ProgressCallbackHelper {
    fn default() -> Self {
        return Self {
            last_update_time: SystemTime::now(),
            last_throughput_update_time: SystemTime::now(),
            last_transfered_bytes: 0,
            previous_throughut: 0,
        };
    }
}

impl ProgressCallbackHelper {
    fn update_remote(index: usize, msg: &[u8], sender: &ChannelHelper<(usize, String)>) -> usize {
        let mut index = index;
        let msg = String::from_utf8_lossy(msg);
        let msgs = msg.split('\n');
        for msg in msgs {
            let messages = msg.split('\r');
            for message in messages {
                if !message.is_empty() {
                    let message = format!("remote: {}", message);
                    let _ = sender.send((index, message));
                }
                if message.ends_with("done.") {
                    index += 1;
                }
            }
        }
        return index;
    }

    fn update_resolving(
        &mut self,
        indexed_deltas: usize,
        total_deltas: usize,
        sender: &ChannelHelper<(usize, String)>,
        index: usize,
    ) -> usize {
        let now = SystemTime::now();
        let time_since_last_update = now
            .duration_since(self.last_update_time)
            .unwrap()
            .as_millis();
        if total_deltas == 0 {
            return index;
        }
        if time_since_last_update >= 100 && indexed_deltas < total_deltas {
            self.last_update_time = now;
            let percent = indexed_deltas * 100 / total_deltas;
            let msg = format!("Resolving deltas: {percent}% ({indexed_deltas}/{total_deltas})");
            let _ = sender.send((index, msg));
        } else if indexed_deltas == total_deltas {
            // making sure the last msg is sent regardless of rate limiting.
            self.last_update_time = now;
            let msg = format!("Resolving deltas: 100% ({indexed_deltas}/{total_deltas}), done.");
            let _ = sender.send((index, msg));
            return index + 1;
        }
        return index;
    }

    fn update_receiving(
        &mut self,
        recieved_obj: usize,
        total_obj: usize,
        recieved_bytes: usize,
        sender: &ChannelHelper<(usize, String)>,
        index: usize,
    ) -> usize {
        // calculating throughput
        let now = SystemTime::now();
        let time_since_last_update = now
            .duration_since(self.last_update_time)
            .unwrap()
            .as_millis();
        let time_since_last_throughput_update = now
            .duration_since(self.last_throughput_update_time)
            .unwrap()
            .as_millis();

        if time_since_last_throughput_update >= 500 {
            let throughput = ((recieved_bytes - self.last_transfered_bytes) * 1_000) as u128
                / time_since_last_throughput_update;
            self.previous_throughut = throughput;
            self.last_throughput_update_time = now;
        }
        if time_since_last_update >= 100 && recieved_obj < total_obj {
            // making stuff pretty
            let (speed_num, speed_unit) = Self::give_speed(self.previous_throughut);
            let (transfer_num, transfer_unit) = Self::give_data_transfer(recieved_bytes);
            let complete_percent = 100 * recieved_obj / total_obj;

            self.last_update_time = now;
            self.last_transfered_bytes = recieved_bytes;

            let msg = format!(
                "Receiving objects: {complete_percent}% ({recieved_obj}/{total_obj}), {:.2} {transfer_unit} | {:.2} {speed_unit}",
                transfer_num, speed_num
            );
            let _ = sender.send((index, msg));
            return index;
        } else if recieved_obj == total_obj {
            // making sure the last msg is sent regardless of rate limiting.
            let (speed_num, speed_unit) = Self::give_speed(self.previous_throughut);
            let (transfer_num, transfer_unit) = Self::give_data_transfer(recieved_bytes);
            let complete_percent = 100 * recieved_obj / total_obj;
            let msg = format!(
                "Receiving objects: {complete_percent}% ({recieved_obj}/{total_obj}), {:.2} {transfer_unit} | {:.2} {speed_unit}, done.",
                transfer_num, speed_num
            );
            let _ = sender.send((index, msg));
            return index + 1;
        }
        return index;
    }

    fn give_speed(bytes_per_sec: u128) -> (f32, String) {
        // GiB/s
        if bytes_per_sec > 1_073_741_824 {
            let bytes_per_sec: f32 = bytes_per_sec as f32;
            let speed: f32 = bytes_per_sec / 1_073_741_824.0;
            return (speed, "GiB/s".to_string());
        // MiB/s
        } else if bytes_per_sec > 1_048_576 {
            let bytes_per_sec: f32 = bytes_per_sec as f32;
            let speed: f32 = bytes_per_sec / 1_048_576.0;
            return (speed, "MiB/s".to_string());
        // KiB/s
        } else if bytes_per_sec > 1_024 {
            let bytes_per_sec: f32 = bytes_per_sec as f32;
            let speed: f32 = bytes_per_sec / 1_024.0;
            return (speed, "KiB/s".to_string());
        }
        return (bytes_per_sec as f32, "B/s".to_string());
    }

    fn give_data_transfer(bytes_transfered: usize) -> (f32, String) {
        // gib/s
        if bytes_transfered > 1_073_741_824 {
            let bytes_transfered: f32 = bytes_transfered as f32;
            let data: f32 = bytes_transfered / 1_073_741_824.0;
            return (data, "Gib".to_string());
        // mib/s
        } else if bytes_transfered > 1_048_576 {
            let bytes_transfered: f32 = bytes_transfered as f32;
            let data: f32 = bytes_transfered / 1_048_576.0;
            return (data, "Mib".to_string());
        // kib/s
        } else if bytes_transfered > 1_024 {
            let bytes_transfered: f32 = bytes_transfered as f32;
            let data: f32 = bytes_transfered / 1_024.0;
            return (data, "Kib".to_string());
        }
        return (bytes_transfered as f32, "B".to_string());
    }
}

#[cfg(test)]
mod clone_test {
    use super::{CloneConfig, CloneFlags};
    use crate::GitRepository;
    use std::{io::BufRead, path::Path, process::Command};

    #[test]
    fn git_clone_depth_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_depth"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_depth"),
        );
        config.add_flag(CloneFlags::Depth(Some(1)));
        repo.git_clone(config).unwrap();

        // verify that a single commit is cloned in
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/clone_depth/git2-rs/",
                "rev-list",
                "--count",
                "--all",
            ])
            .output()
            .unwrap();

        Command::new("rm")
            .args(["-rf", "./temp_test/clone_depth/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "1\n");
    }

    #[test]
    fn git_clone_bare_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_bare"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_bare"),
        );
        config.add_flag(CloneFlags::Bare(true));
        repo.git_clone(config).unwrap();

        // verify that repository is bare
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/clone_bare/git2-rs.git/",
                "rev-parse",
                "--is-bare-repository",
            ])
            .output()
            .unwrap();

        // delete the repository
        Command::new("rm")
            .args(["-rf", "./temp_test/clone_bare/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "true\n");
    }

    #[test]
    fn git_clone_branch_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_branch"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_branch/"),
        );
        config.add_flag(CloneFlags::Branch(Some(String::from("curl"))));
        repo.git_clone(config).unwrap();

        // verify that a single commit is cloned in
        let out = Command::new("git")
            .args(["-C", "./temp_test/clone_branch/git2-rs/", "branch"])
            .output()
            .unwrap();

        Command::new("rm")
            .args(["-rf", "./temp_test/clone_branch/"])
            .output()
            .unwrap();

        assert_eq!(String::from_utf8_lossy(&out.stdout), "* curl\n");
    }

    #[test]
    fn git_clone_single_branch_test() {
        // create temp directories
        Command::new("mkdir")
            .args(["-p", "./temp_test/clone_single_branch"])
            .output()
            .unwrap();

        // clone git2 using gittwo
        let mut repo = GitRepository::new();
        let mut config = CloneConfig::new(
            "https://github.com/rust-lang/git2-rs.git".to_string(),
            Path::new("./temp_test/clone_single_branch/"),
        );
        config.add_flag(CloneFlags::SingleBranch(true));
        repo.git_clone(config).unwrap();

        // verify that a single commit is cloned in
        let out = Command::new("git")
            .args([
                "-C",
                "./temp_test/clone_single_branch/git2-rs/",
                "branch",
                "--remotes",
            ])
            .output()
            .unwrap();

        let out = out.stdout.lines().count();
        Command::new("rm")
            .args(["-rf", "./temp_test/clone_single_branch/"])
            .output()
            .unwrap();

        assert_eq!(out, 2);
    }
}
