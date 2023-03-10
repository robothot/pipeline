use actix::Addr;
use actix_web::{post, web::{ self, Data }, Result};
use log::{info, error};
use serde_json::Value;

use crate::{services::types::{Reply, StateEnum}, actors::{pipeline::{PipelineActor, RunPipeline}}, gitlab::{self, mr::get_gitlab_config}};

#[post("/api/gitlab/webhook/events")]
pub async fn webhook_events(
    params: web::Json<Value>,
    pipeline: Data<Addr<PipelineActor>>
) -> Result<Reply> {

    let event_type = params.get("object_kind");
    let object_attributes = params.get("object_attributes");
    let project = params.get("project");
    if event_type.is_some() && object_attributes.is_some() && project.is_some() {
        let project_id = project.unwrap().get("id").unwrap().as_u64().unwrap();
        let iid = object_attributes.unwrap().get("iid").unwrap().as_u64().unwrap();
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap();
        let action = object_attributes
            .unwrap()
            .get("action")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let title =  object_attributes
            .unwrap()
            .get("title")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let event_type = event_type.unwrap().as_str().unwrap();
        // see https://docs.gitlab.com/ee/user/project/integrations/webhook_events.html#merge-request-events
        if event_type.eq("merge_request") && action.eq("merge") {
            info!("[merge] merge_request -> received task, {}", object_attributes.unwrap().get("title").unwrap().as_str().unwrap());
            pipeline.do_send(RunPipeline {
                param: params.clone()
            });
            return Ok(Reply {
                state: StateEnum::SUCCESS,
                data: None,
                hint: "SUCCESS: order has been received".to_string(),
            });
        } else if event_type.eq("merge_request") && action.eq("open") {

            // 如果是自动发版创建的 MR 则直接合并即可
            if title.contains("[RUST-ROBOT]:") {
                match gitlab::mr::accept(project_id, iid).await {
                    Ok(resp) => {
                        let _ = resp.text().await;
                    },
                    Err(err) => {
                        error!("ERROR: {}", err.to_string());
                    },
                }
            } else {
                let gitlab = get_gitlab_config();
                if let Some((_, project)) = gitlab.project.iter().find(|(project_id, item)| {
                    let project = project_id.to_string();
                    item.branch.eq(&target_branch.to_string()) && project.eq(*project_id)
                }) {
                    if let Some(_) = project.merge_request.iter().find(|data| {
                        "quick://merge".eq(data.clone())
                    }) {
                        match gitlab::mr::accept(project_id, iid).await {
                            Ok(resp) => {
                                let _ = resp.text().await;
                            },
                            Err(err) => {
                                error!("ERROR: {}", err.to_string());
                            },
                        }
                    }
                }
            }

            return Ok(Reply {
                state: StateEnum::SUCCESS,
                data: None,
                hint: "SUCCESS: merge_request open ".to_string(),
            });
        }
        return Ok(Reply {
            state: StateEnum::ERROR,
            data: None,
            hint: "ERROR: did not match the specified command".to_string(),
        });
    }
    let reply = Reply {
        state: StateEnum::ERROR,
        data: None,
        hint: "ERROR: the parameter information is incorrect".to_string(),
    };
    return Ok(reply)
}
