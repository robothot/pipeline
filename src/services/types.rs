use actix_web::{Responder, body::BoxBody, HttpRequest, HttpResponse, http::header::ContentType};
use serde::{ Serialize };
use serde_json::Value;

#[derive(Serialize)]
pub enum StateEnum {
    ERROR,
    SUCCESS
}

#[derive(Serialize)]
pub struct Reply {
    pub state: StateEnum,
    pub data: Option<Value>,
    pub hint: String

}

impl Responder for Reply {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body)
    }
}

