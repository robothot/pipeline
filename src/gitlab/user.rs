


use log::info;
use serde::{Serialize, Deserialize};

use super::comm::get_gitlab_config;

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: u64,
    pub name: String,
    pub username: String,
    pub state: String,
    pub avatar_url: String,
    pub web_url: String,
    pub created_at: String,
    pub bio: String,
    pub location: String,
    pub skype: String,
    pub linkedin: String,
    pub twitter: String,
    pub website_url: String,
    pub organization: String
}

pub async fn query_by_id (id: u64) -> Result<UserInfo, reqwest::Error>{
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/users/{}", gitlab.protocol, gitlab.host, id);
    info!("request -> {}", url);
    let resp = reqwest::Client::new()
        .get(url)
        .header("Private-Token", gitlab.token.as_str())
        .header("Accept", "application/json; charset=utf-8")
        .send()
        .await?;
    let resp = resp.json().await?;
    Ok(resp)
}


#[derive(Serialize, Deserialize)]
struct User {
    pub id: String,
    pub name: String,
    pub username: String,
    pub state: String,
    pub avatar_url: String,
    pub web_url: String
}

pub async fn query_by_name (username: String) -> Result<UserInfo, reqwest::Error>{
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/users?username={}", gitlab.protocol, gitlab.host, username);
    info!("request -> {}", url);
    let resp = reqwest::Client::new()
        .get(url)
        .header("Private-Token", gitlab.token.as_str())
        .header("Accept", "application/json; charset=utf-8")
        .send()
        .await?;
    let resp: Vec<UserInfo> = resp.json().await?;
    let id = &resp[0].id;
    query_by_id(id.clone()).await
}
