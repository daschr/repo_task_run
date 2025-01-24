use std::{env, error::Error};

use common::{get_appdata_local, get_programdata};
use installation::{AutostartConfiguration, PerUserAutostart, SystemAutostart};
use log::{error, info};
use task::ExecutionContext;
use task_runner::TaskRunner;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

mod common;
mod entra_groups;
mod gix_repository;
mod installation;
mod task;
mod task_fetcher;
mod task_runner;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        winapi::um::wincon::FreeConsole();
    }

    let args: Vec<String> = env::args().collect();

    let own_username = env::var("USERNAME").expect("Failed to get username!");
    let computername = env::var("COMPUTERNAME").expect("Failed to get the Computername");

    info!("Username: {} Computername: {}", own_username, computername);

    let username_without_dollar = own_username.as_str().split("$").next().unwrap();

    let execution_context = if computername == username_without_dollar {
        ExecutionContext::System
    } else {
        ExecutionContext::User
    };

    let writer = {
        let p = match execution_context {
            ExecutionContext::System => get_programdata()?,
            ExecutionContext::User => get_appdata_local()?,
        };

        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .max_log_files(7)
            .filename_prefix("repo_task_run")
            .build(p)
            .expect("Failed to build LogFileAppender")
    };

    tracing_subscriber::fmt().with_writer(writer).init();

    info!("Username: {} Computername: {}", own_username, computername);

    if args.len() > 1 {
        match args[1].as_str() {
            "--install" => match execution_context {
                ExecutionContext::System => {
                    if let Err(e) = SystemAutostart::install() {
                        error!("Error installing system autostart: {:?}", e);
                        return Err(e);
                    }
                }
                ExecutionContext::User => {
                    if let Err(e) = PerUserAutostart::install() {
                        error!("Error installing per-user autostart: {:?}", e);
                        return Err(e);
                    }
                }
            },
            "--uninstall" => match execution_context {
                ExecutionContext::System => SystemAutostart::uninstall()?,
                ExecutionContext::User => PerUserAutostart::uninstall()?,
            },
            _ => (),
        }
    }

    info!("Running tasks...");

    let mut runner = TaskRunner::new(execution_context)?;

    runner.run();

    Ok(())
}
