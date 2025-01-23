use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashSet;

use crate::common::{AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_TENANT_ID};

pub fn get_azure_groups_of_user(upn: &str) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let token_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        AZURE_TENANT_ID
    );
    let client = Client::new();
    let token_response: Value = client
        .post(&token_url)
        .form(&[
            ("client_id", AZURE_CLIENT_ID),
            ("scope", "https://graph.microsoft.com/.default"),
            ("client_secret", AZURE_CLIENT_SECRET),
            ("grant_type", "client_credentials"),
        ])
        .send()?
        .json()?;

    let access_token = token_response["access_token"]
        .as_str()
        .ok_or("Failed to get access token")?;

    let graph_url = format!("https://graph.microsoft.com/v1.0/users/{}/memberOf", upn);
    let response: Value = client
        .get(&graph_url)
        .bearer_auth(access_token)
        .send()?
        .json()?;

    let groups = match response["value"].as_array() {
        Some(groups) => groups
            .iter()
            .filter(|g| g["displayName"].is_string())
            .map(|g| g["displayName"].as_str().unwrap().to_string())
            .collect(),
        None => HashSet::new(),
    };

    Ok(groups)
}
