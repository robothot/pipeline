use actix::{Actor, Context, Handler, Message};
use gitlab::{Gitlab, api::{projects::merge_requests::{ MergeMergeRequest, RebaseMergeRequest }, Query}};
use tokio::task::spawn_blocking;

use crate::ConfigToml;


#[derive(Message)]
#[rtype(result = "()")]
pub struct MergeRequest {
    pub project: u64,
    pub merge_request: u64,
    pub target_branch: String
}

pub struct GitLabActor {
    config: ConfigToml
}

impl GitLabActor {
    pub fn new(setting: ConfigToml) -> GitLabActor {
        GitLabActor {
            config: setting
        }
    }
    fn get_gitlab (setting: ConfigToml) -> Gitlab {
        let gitlab = match setting.gitlab.protocol.as_str() {
            "http" => {
                Gitlab::new(
                    setting.gitlab.host.as_str(),
                    setting.gitlab.token.as_str()
                ).unwrap()
            },
            _ => {
                Gitlab::new_insecure(
                    setting.gitlab.host.as_str(),
                    setting.gitlab.token.as_str()
                ).unwrap()
            }
        };
        return gitlab;
    }
} 

impl Actor for GitLabActor {
    type Context = Context<Self>;
}

impl Handler<MergeRequest> for GitLabActor{ 
    type Result = ();

    fn handle(&mut self, msg: MergeRequest, _: &mut Self::Context) {
        let setting = self.config.clone();
        if let Some((_, project)) = self.config.gitlab.project.iter().find(|(project_id, item)| {
            let project = msg.project.to_string();
            item.branch.eq(&msg.target_branch) && project.eq(*project_id)
        }) {
            if let Some(element) = project.merge_request.iter().find(|data| {
                let data: Vec<&str> = data.split("://").collect();
                data[0].eq("quick")
            }) {
                let data: Vec<&str> = element.split("://").collect();
                if "merge".eq(data[1]) {
                    let endpoint = MergeMergeRequest::builder()
                    .project(msg.project)
                    .merge_request(msg.merge_request)
                    .build()
                    .unwrap();

                    spawn_blocking(move || {
                        let gitlab = Self::get_gitlab(setting);
                        gitlab::api::ignore(endpoint).query(&gitlab).unwrap();
                    });
                } else if "rebase".eq(data[1]) {
                    let endpoint = RebaseMergeRequest::builder()
                    .project(msg.project)
                    .merge_request(msg.merge_request)
                    .build()
                    .unwrap();

                    spawn_blocking(move || {
                        let gitlab = Self::get_gitlab(setting);
                        gitlab::api::ignore(endpoint).query(&gitlab).unwrap();
                    });
                }
            }
            
        }
    }
}