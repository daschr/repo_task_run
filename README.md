<div align="center">
  <h1>RepoTaskRun</h1>
  <em>A standalone executable which clones a git repository and executes all powershell scripts in it.</em><br><br>
  <em>Born out of the frustration that Intune deployment is such a pain.</em>
</div>

## Capabilities
- detects whether it runs in SYSTEM-context or user-context (like apps in Intune)
- installs itself to autostart on system boot or user login
- clones a repository and executes all powershell scripts in it based on rules defined by the directory structure


## Directory rules
|syntax|description|example|
|----|-----------|-------|
|`group-<groupname>`|executes all scripts in the folder only if the user is a member of the Entra group <groupname>|`group-sales`|
|`depends-<script name without extension>`|executes all scripts in the folder not before the script name `<script name without extension>` has run|`depends-install-openvpn`|
|`context-<system\|user>`|executes all scripts in the folder only if RepoTaskRun is executed either system or user context|`context-system` or `context-user`|
|`reboot-<enabled\|disabled>`|on `reboot-enabled`: after a script in this folder ran, reboot the machine|`reboot-enabled`|
|`type-<oneshot\|onboot>`|on `type-oneshot`: only execute the scripts a single time, but re-execute them if they have changed; on `type-onboot` execute the scripts at every boot|`reboot-oneshot`|

Your deployment repository will then maybe look like this:

<p align="center"> 
  <img src="https://github.com/user-attachments/assets/014d674e-2929-4824-97c7-24ebd73dca9d" />
</p>

## Requirements
- you need Office365 with Intune (or any other subscription where Intune is included)
- you need to create a Entra daemon application:
  -  see https://learn.microsoft.com/en-us/entra/identity-platform/quickstart-register-app
    -  specify "Accounts in this organization directory only"
    -  do not set a redirect URI
    -  register
  - go to "Certificates & Secrets" and create a new "Client Secret"; save the secret, it is only shown once, see the image
    ![image](https://github.com/user-attachments/assets/aa213fd9-9884-41b3-8153-00eef217845c)

  - go to "API permissions" and add the ones in the image below; **IMPORTANT**: set "Application" as the permission type
    ![image](https://github.com/user-attachments/assets/14edcea1-30be-4daf-86ca-5dd6c00b1901)

  - click on the "grant admin consent for <your company name>" above the permission table

## Building
1. you need to specify the following environment variables:

|envvar name|description|example|
|----|-----------|-------|
|`REPO_HOST`|the repo host and port to connect to, this is used to wait for until the repository is reachable|`github.com:22`| 
|`REPO_URL`|the ssh repository url containing the scripts|`git@github.com:yourcompany/company-intune-scripts.git`|
|`ENTRA_TENANT_ID`|the Entra tenant id of your organization|`01949404-f2d7-709d-b77f-48e99edbfeea`|
|`ENTRA_CLIENT_ID`|the Entra client id of your application (RepoRunTask)|`01949404-f2d7-709d-b77f-5d6c897d04c4`|
|`ENTRA_CLIENT_SECRET`|the Entra client secret|`oiahjns~~aioiNAS9d70a9dnpsasodipaf0wwi2`|

2. you need to create a new ssh key, f. e.  using `ssh-keygen -b 4096 -f ssh_key`, and store the private key to `ssh-key`, it gets imported at build-time
3. run `cargo b --release`

## Deployment
- you need to add the public ssh key as a deployment key in your repository, RepoTaskRun uses it for authentication

### Intune Application
- you can build a *.intunewin* package wich only contains the executable (`./target/x86_64-pc-windows-gnu/release/repo_task_run.exe`)
- the app configuration for running the it in system-context is the following:
  ![image](https://github.com/user-attachments/assets/2a240950-dece-4342-8d44-99c6297e251b)
- the app configuration for running the it per-user in user context is the following:
  ![image](https://github.com/user-attachments/assets/59c3c00a-ca6a-42e6-bd3c-fa906aa5884e)

## Debugging
- the location of the logfiles in *system* context is `C:\Programdata\repo_task_run.*`
- the location of the logfiles in *per-user* context is `%LOCALAPPDATA%\repo_task_run.*`
