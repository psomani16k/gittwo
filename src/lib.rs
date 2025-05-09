pub extern crate git2;

mod configs;
mod repository;

pub use self::configs::clone_config::CloneConfig;
pub use self::configs::clone_config::CloneFlags;
pub use self::repository::repository::GitRepository;
