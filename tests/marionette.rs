use actix_web::http::header::ContentType;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::env;

#[actix_web::main]
async fn main() {
    if let Some(arg_count) = env::args().size_hint().1 {
        if arg_count == 1 {
            println!("Run without args, exiting since probably cargo test");
            return;
        }
    }

    let server = HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(page))
            .route("/args", web::get().to(args))
            .route("/exit", web::post().to(exit))
            .route("/stdout", web::post().to(stdout))
            .route("/stderr", web::post().to(stderr))
    });
    server
        .bind("localhost:8080")
        .expect("could not bind")
        .run()
        .await
        .expect("failure when running server");
}

#[derive(Serialize)]
struct ArgsResult {
    args: Vec<String>,
}

#[derive(Deserialize)]
struct ExitParameters {
    exit_code: i32,
}

#[derive(Deserialize)]
struct PrintParameters {
    text: String,
}

async fn page() -> HttpResponse {
    HttpResponse::Ok().content_type(ContentType::html()).body(
        r#"
            <h1>Marionette</h1>
            <form action="/args" method="get">
              <button type="submit">Args</button>
            </form>
            <form action="/exit" method="post">
              <input type="number" name="exit_code"/>
              <button type="submit">Exit</button>
            </form>
            <form action="/stdout" method="post">
              <input type="text" name="text"/>
              <button type="submit">Stdout</button>
            </form>
            <form action="/stderr" method="post">
              <input type="text" name="text"/>
              <button type="submit">Stderr</button>
            </form>
        "#,
    )
}

async fn args() -> HttpResponse {
    HttpResponse::Ok().json(ArgsResult {
        args: env::args().collect(),
    })
}

async fn exit(parameters: web::Form<ExitParameters>) -> HttpResponse {
    tokio::task::spawn(async move {
        std::process::exit(parameters.exit_code);
    });
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("Bye, bye")
}

async fn stdout(parameters: web::Form<PrintParameters>) -> HttpResponse {
    print!("{}", parameters.text);
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("Printed '{}' to stdout", parameters.text))
}

async fn stderr(parameters: web::Form<PrintParameters>) -> HttpResponse {
    eprint!("{}", parameters.text);
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("Printed '{}' to stderr", parameters.text))
}
