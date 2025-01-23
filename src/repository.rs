use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use std::error::Error;

use std::path::Path;
use std::{env, fs, str};

use crate::common::{APP_NAME, GIT_USERNAME, REPO_URL, SSH_KEY};

fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = git2::RemoteCallbacks::default();

    cb.credentials(|_url, username_from_url, _allowed_types| {
        println!(
            "credentials2 called! {:?} {:?} {:?}",
            _url, username_from_url, _allowed_types
        );

        Cred::ssh_key_from_memory(
            username_from_url.unwrap_or(GIT_USERNAME),
            None,
            SSH_KEY,
            None,
        )
    });

    cb.certificate_check(|_, cn| {
        println!("certificate_check called! {}", cn);
        Ok(git2::CertificateCheckStatus::CertificateOk)
    });

    let mut fo = git2::FetchOptions::default();
    fo.remote_callbacks(cb);
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    println!("Fetching {} {:?} for repo", remote.name().unwrap(), refs);
    remote.fetch(refs, Some(&mut fo), None)?;

    println!("Fetched.");

    println!("getting fetch head");
    let fetch_head = repo.find_reference("FETCH_HEAD")?;

    Ok(repo.reference_to_annotated_commit(&fetch_head)?)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    println!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
}

pub fn update_repo<'a>(repo_path: &Path, branch: &str) -> Result<bool, Box<dyn Error>> {
    let mut remote_cb = RemoteCallbacks::default();

    remote_cb.credentials(|_url, username_from_url, _allowed_types| {
        println!(
            "credentials1 called! {:?} {:?} {:?}",
            _url, username_from_url, _allowed_types
        );

        Cred::ssh_key_from_memory(
            username_from_url.unwrap_or(GIT_USERNAME),
            None,
            SSH_KEY,
            None,
        )
    });

    remote_cb.certificate_check(|_, cn| {
        println!("certificate_check called!  {}", cn);
        Ok(git2::CertificateCheckStatus::CertificateOk)
    });

    let mut fo = FetchOptions::default();
    fo.remote_callbacks(remote_cb);

    let repo = {
        if repo_path.exists() {
            println!("Repo exists, trying to open it.");
            Repository::open(repo_path)?
        } else {
            println!("Repo does not exist, cloning it.");
            let bn = repo_path.parent().unwrap();
            fs::create_dir_all(&bn)?;

            RepoBuilder::new()
                .fetch_options(fo)
                .clone(REPO_URL, &repo_path)?;

            println!("Successfully cloned repo!");
            return Ok(true);
        }
    };

    println!("Updating timeouts...");
    unsafe {
        git2::opts::set_server_connect_timeout_in_milliseconds(3000)?;
        git2::opts::set_server_timeout_in_milliseconds(3000)?;
    }

    git2::trace_set(git2::TraceLevel::Info, |level, buf| {
        println!(
            "{:?}: {:?}",
            level,
            buf // str::from_utf8(buf).expect("Failed to decode as utf-8")
        );
    })?;
    println!("Finding remote origin");
    let mut remote = repo.find_remote("origin")?;
    println!("Fetching...");
    let fetch_commit = do_fetch(&repo, &[branch], &mut remote)?;

    println!("Doing merge analysis");
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.0.is_up_to_date() {
        println!("Repo ist up-to-date!");
        return Ok(false);
    }

    if analysis.0.is_fast_forward() {
        println!("Doing a fast forward");

        let refname = format!("refs/heads/{}", branch);
        let mut r = repo.find_reference(&refname)?;

        fast_forward(&repo, &mut r, &fetch_commit)?;
    } else {
        println!("Need to remove repo and do fresh clone...");
        let bn = repo_path.parent().unwrap();
        fs::create_dir_all(&bn)?;

        fs::remove_dir(repo_path)?;

        println!("Cloning...");
        RepoBuilder::new()
            .fetch_options(fo)
            .clone(REPO_URL, &repo_path)?;
    }

    println!("Sucessfully updated repo!");
    Ok(true)
}
