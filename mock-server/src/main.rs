use std::time::Duration;

use actix_web::{
    get,
    http::StatusCode,
    web::{Json, Path},
    App, HttpServer, Responder,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

/*
   Some sample datastructure that we want to serve over the http interface
*/
#[derive(Serialize, Deserialize, Clone)]
pub struct Person {
    pub id: u32,
    pub first_name: &'static str,
    pub last_name: &'static str,
    pub age: u32,
}

/*
   Define a constant person we want to serve
*/
const PERSON: Person = Person {
    id: 7,
    last_name: "John",
    first_name: "Doe",
    age: 30,
};

/*
   Simply return the person we defined earlier, serialized as json
*/
#[get("/person")]
async fn get_person() -> Json<Person> {
    Json(PERSON)
}

/*
   Wait a bit **for this connection only**, then return the person we defined earlier.
   As before, serialized as json.
*/
#[get("/slow")]
async fn get_person_slow() -> Json<Person> {
    tokio::time::sleep(Duration::from_millis(200)).await;
    Json(PERSON)
}

/*
   Wait a bit **for this connection only**, then return the person we defined earlier.
   As before, serialized as json.
   Note that this endpoint prints a line to stdout so you can see when a request **started**, not when is was completed.
*/
#[get("/slow_log")]
async fn get_person_slow_log() -> Json<Person> {
    println!("slow_log called");
    tokio::time::sleep(Duration::from_millis(3000)).await;
    Json(PERSON)
}

/*
  Provide an endpoint to generate a custom http status code
*/
#[get("/code/{code}")]
async fn get_custom_code(path_code: Path<u16>) -> impl Responder {
    let code = path_code.into_inner();
    (
        format!("Your code: {} \n", code),
        StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_REQUEST),
    )
}

/*
  Provide an endpoint to generate a custom http status code
*/
#[get("/random_code")]
async fn get_random_code() -> impl Responder {
    let vs = vec![
        StatusCode::OK,
        StatusCode::BAD_REQUEST,
        StatusCode::INTERNAL_SERVER_ERROR,
    ];

    let code = vs
        .choose(&mut rand::thread_rng())
        .unwrap_or(&StatusCode::BAD_REQUEST);
    (
        format!("Your code: {} \n", code),
        StatusCode::from_u16(code.as_u16()).unwrap_or(StatusCode::BAD_REQUEST),
    )
}

/*
    Boilerplate to set up actix web
*/
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .service(get_person)
            .service(get_person_slow)
            .service(get_person_slow_log)
            .service(get_custom_code)
            .service(get_random_code)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
