use actix::Actor;
use actix_web::{ App, HttpServer, web::Data };

mod services;
mod actors;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();


    let addr = actors::pipeline::PipelineActor::new().start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(addr.clone()))
            .service(services::web_hooks::webhook_events)
    })
    .workers(5)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}



