use log::info;
use serde_json::{ json };

use super::comm::get_gitlab_config;

pub async fn create(
    project_id: &str,
    merge_request_iid: &str,
    body: &str
) -> Result<reqwest::Response, reqwest::Error> {
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/projects/{}/merge_requests/{}/notes", gitlab.protocol, gitlab.host, project_id, merge_request_iid);
    info!("request -> {}", url);
    let param = json!({
        "body": body
    });
    reqwest::Client::new()
        .post(url)
        .header("Private-Token", gitlab.token.as_str())
        .header("Accept", "application/json; charset=utf-8")
        .json(&param)
        .send()
        .await
}