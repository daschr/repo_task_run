use core::str;
use std::{
    env::{self, VarError},
    net::TcpStream,
    path::PathBuf,
    process::Command,
};

pub const APP_NAME: &str = "RepoTaskRun";
pub const RUN_REGKEY_NAME: &str = "RepoTaskRun";
pub const REPO_HOST: &str = env!("REPO_HOST");
pub const REPO_URL: &str = env!("REPO_URL");
pub const SSH_KEY: &str = include_str!("../ssh_key");

pub const ENTRA_TENANT_ID: &str = env!("ENTRA_TENANT_ID");
pub const ENTRA_CLIENT_ID: &str = env!("ENTRA_CLIENT_ID");
pub const ENTRA_CLIENT_SECRET: &str = env!("ENTRA_CLIENT_SECRET");

#[allow(unused)]
pub fn get_upn() -> Option<String> {
    let c = Command::new("whoami.exe").arg("/upn").output();

    if let Ok(c) = c {
        if !c.stdout.is_empty() {
            if let Ok(s) = str::from_utf8(&c.stdout) {
                return Some(s.split("\n").next().unwrap().to_string());
            }
        }
    }

    None
}

#[allow(unused)]
pub fn get_user_install_path() -> Result<PathBuf, VarError> {
    let mut path = get_appdata_local()?;

    path.push(APP_NAME);

    Ok(path)
}

#[allow(unused)]
pub fn get_system_install_path() -> Result<PathBuf, VarError> {
    let mut path = get_programdata()?;

    path.push(APP_NAME);

    Ok(path)
}

#[allow(unused)]
pub fn get_system_repository_path() -> Result<PathBuf, VarError> {
    let mut path = get_programdata()?;

    path.push(APP_NAME);
    path.push("repo");

    Ok(path)
}

#[allow(unused)]
pub fn get_user_repository_path() -> Result<PathBuf, VarError> {
    let mut path = get_appdata_local()?;

    path.push(APP_NAME);
    path.push("repo");

    Ok(path)
}

macro_rules! new_envar_pathgetter {
    ($name:ident, $var:literal) => {
        #[allow(unused)]
        pub fn $name() -> Result<PathBuf, VarError> {
            let p = match env::var($var) {
                Ok(h) => PathBuf::from(h.trim()),
                Err(e) => return Err(e),
            };

            Ok(p)
        }
    };
}

new_envar_pathgetter!(get_appdata_roaming, "APPDATA");
new_envar_pathgetter!(get_appdata_local, "LOCALAPPDATA");
new_envar_pathgetter!(get_tempdir, "TEMP");
new_envar_pathgetter!(get_homepath, "HOMEPATH");
new_envar_pathgetter!(get_programdata, "PROGRAMDATA");
new_envar_pathgetter!(get_userprofile, "USERPROFILE");

pub fn is_host_reachable(addr: &str) -> bool {
    TcpStream::connect(addr).is_ok()
}
