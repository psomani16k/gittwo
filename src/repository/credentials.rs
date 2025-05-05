use git2::{Cred, Error};

#[derive(Clone)]
pub(crate) enum GitCredentials {
    Https(GitHttpsCredentials),
    // Ssh(GitSshCredentials),
    Default,
}

impl GitCredentials {
    pub(crate) fn get_cred(&self) -> Result<Cred, Error> {
        match self {
            GitCredentials::Https(git_https_credentials) => git_https_credentials.get_cred(),
            // GitCredentials::Ssh(git_ssh_credentials) => git_ssh_credentials,
            GitCredentials::Default => Cred::default(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct GitHttpsCredentials {
    user: Option<String>,
    pass: Option<String>,
}

impl GitHttpsCredentials {
    pub fn new(user: Option<String>, pass: Option<String>) -> Self {
        return GitHttpsCredentials { user, pass };
    }

    pub(crate) fn get_cred(&self) -> Result<Cred, Error> {
        if let Some(user) = &self.user {
            if let Some(pass) = &self.pass {
                return Cred::userpass_plaintext(&user, &pass);
            } else {
                return Cred::username(&user);
            }
        }
        return Cred::default();
    }

    pub fn get_cred_type(&self) -> Result<CredType, Error> {
        let cred = self.get_cred()?;
        match cred.credtype() {
            1 => Ok(CredType::UserPassPlainText),
            2 => Ok(CredType::SshKey),
            4 => Ok(CredType::SshCustom),
            8 => Ok(CredType::Default),
            16 => Ok(CredType::SshInteractive),
            32 => Ok(CredType::Username),
            64 => Ok(CredType::SshMemory),
            _ => Ok(CredType::Unknown),
        }
    }
}

#[derive(Clone)]
pub(crate) struct GitSshCredentials {}

pub enum CredType {
    UserPassPlainText,
    SshKey,
    SshCustom,
    Default,
    SshInteractive,
    Username,
    SshMemory,
    Unknown,
}
