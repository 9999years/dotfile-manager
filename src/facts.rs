use std::collections::HashMap;
use std::env;

use futures::executor::{block_on, block_on_stream};
use heim::host::{Arch, Platform, User};
use heim::net::{Address, Nic};

#[cfg(linux)]
use heim::host::os::linux::UserExt;

/// System facts to be used for deciding dotfile status.
pub struct Facts {
    /// Map from usernames to user info.
    users: HashMap<String, User>,
    /// Map from network interface names to interface info.
    networks: HashMap<String, Nic>,
    platform: Platform,
}

impl Facts {
    pub fn new() -> heim::Result<Self> {
        Ok(Self {
            users: block_on_stream(heim::host::users())
                .map(|user| user.map(|u| (u.username().to_string(), u)))
                .collect::<heim::Result<_>>()?,
            networks: block_on_stream(heim::net::nic())
                .map(|nic| nic.map(|n| (n.name().to_string(), n)))
                .collect::<heim::Result<_>>()?,
            platform: block_on(heim::host::platform())?,
        })
    }

    pub fn user(&self, username: &str) -> Option<&User> {
        self.users.get(username)
    }

    pub fn os(&self) -> OsType {
        self.platform.system().into()
    }

    pub fn os_release(&self) -> &str {
        self.platform.release()
    }

    pub fn os_version(&self) -> &str {
        self.platform.version()
    }

    pub fn arch(&self) -> Arch {
        self.platform.architecture()
    }

    pub fn hostname(&self) -> &str {
        self.platform.hostname()
    }

    pub fn network(&self, interface: &str) -> Option<&Nic> {
        self.networks.get(interface)
    }

    pub fn addresses(&self) -> Vec<Address> {
        self.networks.values().map(|nic| nic.address()).collect()
    }

    pub fn env(&self, var: &str) -> Option<String> {
        env::var(var).ok()
    }
}

pub enum OsType {
    Linux,
    MacOS,
    Windows,
    Other(String),
}

impl<S: AsRef<str>> From<S> for OsType {
    fn from(s: S) -> Self {
        match s.as_ref() {
            "Linux" => OsType::Linux,
            "Darwin" => OsType::MacOS,
            "Windows" => OsType::Windows,
            s => OsType::Other(s.to_string()),
        }
    }
}
