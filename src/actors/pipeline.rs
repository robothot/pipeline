use std::{error::Error, env, fs::{self}, process::{Command, Stdio}, io::BufRead, collections::HashMap, fmt, sync::Mutex, cell::{RefCell, RefMut}};

use actix::{Actor, Context, Handler, Message };
use actix_web::web::{self, Data};
use log::{info};
use serde::Deserialize;
use serde_json::Value;


#[derive(Deserialize)]
struct PipelineTomlBuild {
    command: String,
    output_directory: String
}

#[derive(Deserialize)]
struct PipelineTomlPublishGit {

    // 设置环境变量指定 key GIT_SSH_COMMAND='ssh -i  ~/.ssh'
    ssh_key: Option<String>,

    // 分支信息
    branch: String,

    // 仓库地址
    repository: String,
}

#[derive(Deserialize)]
struct PipelineTomlPublish {
    git: HashMap<String, PipelineTomlPublishGit>
}

#[derive(Deserialize)]
struct PipelineToml {
    build: PipelineTomlBuild,
    publish: PipelineTomlPublish
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RunPipeline {
    pub param: Value
}

#[derive(Debug)]
struct PipelineError(String);


impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for PipelineError {}

pub struct PipelineActor {
    pids: Data<Mutex<HashMap<String, String>>>
}

impl PipelineActor {
    pub fn new() -> PipelineActor {
        PipelineActor {
            pids: Data::new(Mutex::new(HashMap::new()))
        }
    }


    fn run_build_script(param: Value, pids: Data<Mutex<HashMap<String, String>>>)-> Result<(), Box<dyn Error>>  {
        Self::run_pipeline_clone(&param)?;
        let path = env::current_dir()?.join(".cache");
        let project = param.get("project");
        let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
        let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let object_attributes = param.get("object_attributes");
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap();
        let project_id = project.unwrap().get("id").unwrap().as_u64().unwrap();
        let target_path = &path.join(&project_namespace).join(&project_name).join(&target_branch);
        let content = fs::read_to_string(target_path.join(".pipeline.toml"))?;
        let setting: PipelineToml  = toml::from_str(&content)?;
        let command = setting.build.command;

        let task_id = format!("{}/{}", project_id, target_branch);


        info!("run command -> {} ", &command);
        let mut child = Command::new("sh")
            .current_dir(&target_path)
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .spawn()?;

        {
            let mut pids = pids.lock().unwrap();
            pids.insert(task_id.clone(), child.id().to_string());

        }
        let out = child.stdout.take().unwrap();
        
        let mut out = std::io::BufReader::new(out);
        let mut buffer = String::new();
        while let Ok(_) = out.read_line(&mut buffer) {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
            if buffer.is_empty() == false {
                info!("[{}] -> {}", &child.id(), buffer);
                buffer.clear();
            }
        }
        {
            let mut pids = pids.lock().unwrap();
            pids.remove(&task_id);
        }
        Ok(())
    }

    fn run_pipeline_clone(param: &Value) -> Result<(), Box<dyn Error>> {
        let path =env::current_dir()?.join(".cache");
        let project = param.get("project");
        let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
        let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let object_attributes = param.get("object_attributes");
    
        let target = object_attributes.unwrap().get("target").unwrap();
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap();
        let target_http_url = target.get("git_http_url").clone().unwrap().as_str().unwrap().to_string();
        let dir = path.join(&project_namespace).join(&project_name);
        if dir.join(target_branch).exists() == false {
            fs::create_dir_all(&dir)?;
            let child = Command::new("sh")
            .current_dir(&dir)
            .arg("-c")
            .arg(format!("git clone -b {} --depth 1 {} {}", &target_branch, &target_http_url, &target_branch))
            .stdout(Stdio::piped())
            .spawn()?;
            child.wait_with_output()?;
        } else {
            let child = Command::new("sh")
            .current_dir(&dir.join(target_branch))
            .arg("-c")
            .arg(format!("git pull "))
            .stdout(Stdio::piped())
            .spawn()?;
            child.wait_with_output()?;
        }
        Ok(())
    }
}

impl Actor for PipelineActor {
    type Context = Context<Self>;
}

impl Handler<RunPipeline> for PipelineActor{
    type Result = ();
    fn handle(&mut self, msg: RunPipeline, _: &mut Self::Context) {
        let param = msg.param.clone();
        let object_attributes = param.get("object_attributes");
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap().to_string();
        let project = param.get("project");
        let project_id = project.unwrap().get("id").unwrap().as_u64().unwrap();
        let task_id = format!("{}/{}", project_id, target_branch);
        let pids = self.pids.clone();

        {
            let mut pids = pids.lock().unwrap();
            if pids.contains_key(&task_id) {
                let id = pids.get(&task_id).unwrap().clone();
                let child = Command::new("sh")
                .arg("-c")
                .arg(format!("kill -9 {}", id))
                .stdout(Stdio::piped())
                .spawn().unwrap();
                child.wait_with_output().unwrap();
                pids.remove(&task_id);
            }
        }

        let _ = web::block(move || {
            let path = env::current_dir().unwrap().join(".cache");
            let project = param.get("project");
            let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
            let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
            let target_path = &path.join(&project_namespace).join(&project_name).join(&target_branch);

            match Self::run_build_script(msg.param.clone(), pids) {
                Ok(()) => {
                    let content = fs::read_to_string(target_path.join(".pipeline.toml")).unwrap();
                    let setting: PipelineToml  = toml::from_str(&content).unwrap();
                    for (name, item) in setting.publish.git.iter() {

                        // let output_directory = target_path.join(&setting.build.output_directory).join(".git");
                        // if  output_directory.exists() {
                        //     std::fs::remove_dir_all(output_directory).unwrap();
                        // }
                        let mut new_branch_name = item.branch.clone();

                        if new_branch_name.eq("$auto_branch") {
                            new_branch_name = format!("RCD_{}", target_branch);
                        }

                        let command = match &item.ssh_key  {
                            Some(git_ssh_command) => {
                                format!(
                                    "GIT_SSH_COMMAND='ssh -i {}' && git init && git add . && git commit -am 'Rust Publish Static Files' && git checkout -b {} && git remote add {} {} && git push --force {} {} ",
                                    git_ssh_command,
                                    &new_branch_name,
                                    &name,
                                    &item.repository,
                                    &name,
                                    &new_branch_name,
                                )
                            },
                            None => {
                                format!(
                                    "git init && git add . && git commit -am 'Rust Publish Static Files' && git checkout -b {} && git remote add {} {} && git push --force {} {} ",
                                    &new_branch_name,
                                    &name,
                                    &item.repository,
                                    &name,
                                    &new_branch_name,
                                )
                            }
                        };

                        info!("run command -> {} ", &command);
                        let child = Command::new("sh")
                        .current_dir(&target_path.join(&setting.build.output_directory))
                        .arg("-c")
                        .arg(command)
                        .stdout(Stdio::piped())
                        .spawn().expect("run task error");
                        child.wait_with_output().unwrap();
                    }
                },
                Err(_) => {
                }
            }
        });
    }
}

