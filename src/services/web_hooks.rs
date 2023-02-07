use actix::Addr;
use actix_web::{post, web::{ self, Data }, Result};
use serde_json::Value;

use crate::{services::types::{Reply, StateEnum}, actors::{pipeline::{PipelineActor, RunPipeline}, gitlab::{GitLabActor, MergeRequest}}};

#[post("/api/gitlab/webhook/events")]
pub async fn webhook_events(
    req: web::Json<Value>,
    pipeline: Data<Addr<PipelineActor>>,
    gitlab: Data<Addr<GitLabActor>>
) -> Result<Reply> {

    let event_type = req.get("event_type");
    let object_attributes = req.get("object_attributes");
    let project = req.get("project");
    let project_id = req.get("id").unwrap().as_u64().unwrap();
    let id = object_attributes.unwrap().get("id").unwrap().as_u64().unwrap();
    let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap();
    if event_type.is_some() && object_attributes.is_some() && project.is_some() {
        let action = object_attributes
            .unwrap()
            .get("action")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();

        let event_type = event_type.unwrap().as_str().unwrap();
        // see https://docs.gitlab.com/ee/user/project/integrations/webhook_events.html#merge-request-events
        if event_type.eq("merge_request") && action.eq("merge") {
            pipeline.do_send(RunPipeline {
                param: req.clone()
            });
            return Ok(Reply {
                state: StateEnum::SUCCESS,
                data: None,
                hint: "SUCCESS: order has been received".to_string(),
            });
        } else if event_type.eq("merge_request") && action.eq("open") {
            gitlab.do_send(MergeRequest {
                project: project_id,
                merge_request: id,
                target_branch: target_branch.to_string()
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
    return Ok(reply);
}
