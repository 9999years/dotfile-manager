use std::collections::HashMap;
use std::env;

use futures::executor::{block_on, block_on_stream};
use heim::host::{Arch, Platform as HeimPlatform, User as HeimUser};
use heim::net::{Address, Nic};
use lazy_static::lazy_static;

#[derive(Debug)]
pub struct User(HeimUser);

impl From<HeimUser> for User {
    fn from(u: HeimUser) -> Self {
        User(u)
    }
}

/// System facts to be used for deciding dotfile status.
pub struct Facts {
    /// Map from usernames to user info.
    users: HashMap<String, HeimUser>,
    /// Map from network interface names to interface info.
    networks: HashMap<String, Nic>,
    platform: HeimPlatform,
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

    // pub fn load<'vm>(&'vm self) -> impl Fn(&'vm Thread) -> vm::Result<vm::ExternModule> {
    //     |vm| {
    //         vm::ExternModule::new(
    //             vm,
    //             record! {
    //                 __user_facts => UserFacts(self.users.clone()),
    //                 __user => primitive!(2, UserFacts::user),
    //             },
    //         )
    //     }
    // }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Platform {
    Linux,
    FreeBsd,
    Windows,
    MacOS,
    Ios,
    Android,
    Nintendo,
    Xbox,
    PlayStation,
    Dive,
    Fuchsia,
    Redox,
    Other(String),
}

impl From<whoami::Platform> for Platform {
    fn from(p: whoami::Platform) -> Self {
        match p {
            whoami::Platform::Linux => Platform::Linux,
            whoami::Platform::FreeBsd => Platform::FreeBsd,
            whoami::Platform::Windows => Platform::Windows,
            whoami::Platform::MacOS => Platform::MacOS,
            whoami::Platform::Ios => Platform::Ios,
            whoami::Platform::Android => Platform::Android,
            whoami::Platform::Nintendo => Platform::Nintendo,
            whoami::Platform::Xbox => Platform::Xbox,
            whoami::Platform::PlayStation => Platform::PlayStation,
            whoami::Platform::Dive => Platform::Dive,
            whoami::Platform::Fuchsia => Platform::Fuchsia,
            whoami::Platform::Redox => Platform::Redox,
            whoami::Platform::Unknown(s) => Platform::Other(s),
            unknown => Platform::Other(format!("{}", unknown)),
        }
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
