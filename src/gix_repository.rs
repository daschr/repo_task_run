use core::str;
use log::info;
use std::io::Write;
use std::{error::Error, process::Command};

use std::fs;
use std::path::Path;

use crate::common::{get_userprofile, REPO_URL, SSH_KEY};

fn update_known_hosts(host: &str, known_hosts_file: &Path) -> Result<(), Box<dyn Error>> {
    let cmd = Command::new("ssh-keyscan.exe").arg(host).output()?;

    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(known_hosts_file)?;

    f.write_all(&cmd.stdout)?;

    Ok(())
}

fn add_ssh_key() -> Result<(), Box<dyn Error>> {
    let mut config_path = get_userprofile()?;
    config_path.push(".ssh");

    info!("[add_ssh_key] config_path: {}", config_path.display());

    fs::create_dir_all(&config_path)?;

    let payload = format!(
        r#"# BEGIN RepoRunTask
Host github.com
        IdentityFile {}\repo_task_run_id
# END RepoTaskRun
"#,
        config_path.to_str().unwrap()
    );

    info!("[add_ssh_key] payload: {}", payload);

    config_path.push("config");

    if config_path.exists() {
        let mut content = fs::read_to_string(&config_path)?;

        content.push_str(&payload);

        fs::write(&config_path, content)?;
    } else {
        fs::write(&config_path, payload)?;
    }

    config_path.pop();
    config_path.push("repo_task_run_id");

    fs::write(&config_path, SSH_KEY)?;

    config_path.pop();
    config_path.push("known_hosts");

    info!("known_hosts_path: {}", config_path.display());
    update_known_hosts("github.com", &config_path)?;

    Ok(())
}

fn remove_ssh_key() -> Result<(), Box<dyn Error>> {
    let mut config_path = get_userprofile()?;
    config_path.push(".ssh");

    if !config_path.exists() {
        return Ok(());
    }

    config_path.push("repo_task_run_id");
    fs::remove_file(&config_path).ok();

    config_path.pop();

    config_path.push("config");

    if !config_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)?;

    let mut payload = String::new();

    let mut begin_found = false;

    for line in content.lines() {
        if line.contains("# BEGIN RepoRunTask") {
            begin_found = true;
            continue;
        }

        if line.contains("# END RepoRunTask") {
            begin_found = false;
            continue;
        }

        if begin_found {
            continue;
        }

        payload.push_str(line);
        payload.push_str(r"\r\ņ");
    }

    fs::write(&config_path, payload)?;

    Ok(())
}

pub fn update_repo<'a>(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    unsafe {
        gix::interrupt::init_handler(1, || {})?;
    }

    std::env::set_var("GIT_SSH_COMMAND", "ssh -T");

    info!("Exported GIT_SSH_COMMAND");

    if repo_path.exists() {
        info!("Repo exists, removing it.");

        fs::remove_dir_all(repo_path)?;
    }

    info!("Writing ssh key and config...");

    remove_ssh_key()?;
    add_ssh_key()?;

    info!("Cloning the repo...");
    let bn = repo_path.parent().unwrap();
    fs::create_dir_all(&bn)?;

    let url = gix::url::parse(REPO_URL.into())?;

    info!("Url: {:?}", url.to_bstring());
    let mut prepare_clone = gix::prepare_clone(url, &repo_path)?;
    info!("Cloning {REPO_URL:?} into {repo_path:?}...");

    let (mut prepare_checkout, _) = prepare_clone
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;

    info!(
        "Checking out into {:?} ...",
        prepare_checkout.repo().work_dir().expect("should be there")
    );

    let (repo, _) =
        prepare_checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    info!(
        "Repo cloned into {:?}",
        repo.work_dir().expect("directory pre-created")
    );

    let remote = repo
        .find_default_remote(gix::remote::Direction::Fetch)
        .expect("always present after clone")?;

    info!(
        "Default remote: {} -> {}",
        remote
            .name()
            .expect("default remote is always named")
            .as_bstr(),
        remote
            .url(gix::remote::Direction::Fetch)
            .expect("should be the remote URL")
            .to_bstring(),
    );

    info!("Successfully cloned repo!");

    let mut git_config = repo_path.to_path_buf();
    git_config.push(".git");
    if git_config.is_dir() {
        fs::remove_dir_all(git_config)?;
    }

    remove_ssh_key()?;

    Ok(true)
}
