
use log::info;
use serde::{Serialize, Deserialize};
use serde_json::{json};

use super::comm::get_gitlab_config;

pub async fn accept(project: u64, merge_request: u64) -> Result<reqwest::Response, reqwest::Error> {
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/projects/{}/merge_requests/{}/merge", gitlab.protocol, gitlab.host, project, merge_request);
    info!("request -> {}", url);
    reqwest::Client::new()
    .put(url)
    .header("Private-Token", gitlab.token.as_str())
    .send()
    .await
}

pub async fn create(
    id: &str,
    source_branch: &str,
    target_branch: &str,
    title: &str
) -> Result<reqwest::Response, reqwest::Error> {
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/projects/{}/merge_requests", gitlab.protocol, gitlab.host, id);

    
    let param = json!({
        title: title,
        source_branch: source_branch,
        target_branch: target_branch
    });
    info!("request -> {}", url);
    reqwest::Client::new()
        .post(url)
        .header("Private-Token", gitlab.token.as_str())
        .header("Accept", "application/json; charset=utf-8")
        .json(&param)
        .send()
        .await
}

#[derive(Serialize, Deserialize)]
pub struct MRChangeInfo {
    pub new_path: String
}

#[derive(Serialize, Deserialize)]
pub struct MRChange {
    pub changes: Vec<MRChangeInfo>
}

pub async fn changes (id: String, merge_request_iid: String) -> Result<MRChange, reqwest::Error>{
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/projects/{}/merge_requests/{}/changes", gitlab.protocol, gitlab.host, id, merge_request_iid);
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