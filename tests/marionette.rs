use actix_web::http::header::ContentType;
use actix_web::{dev, web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::{env, io::Write, sync::Mutex};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() == 1 {
        println!("Run without args, exiting since probably cargo test");
        return;
    }
    let Ok(port) = arguments[1].parse::<u16>() else {
        panic!("First argument should be the port number to use. Arguments were {arguments:?}");
    };

    let stop_handle = web::Data::new(StopHandle::new());
    let server = HttpServer::new({
        let stop_handle = stop_handle.clone();
        move || {
            App::new()
                .app_data(stop_handle.clone())
                .route("/", web::get().to(page))
                .route("/args", web::get().to(args))
                .route("/env", web::get().to(env))
                .route("/exit", web::post().to(exit))
                .route("/ping", web::post().to(ping))
                .route("/stdout", web::post().to(stdout))
                .route("/stderr", web::post().to(stderr))
        }
    })
    .bind(("localhost", port))
    .expect("could not bind to port")
    .run();

    stop_handle.register_server(server.handle());

    server.await.expect("failure when running server");
    std::process::exit(stop_handle.get_exit_code());
}

#[derive(Serialize)]
struct ArgsResult {
    args: Vec<String>,
}

#[derive(Serialize)]
struct EnvResult {
    vars: Vec<(String, String)>,
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
            <form action="/env" method="get">
              <button type="submit">Environment variables</button>
            </form>
            <form action="/exit" method="post">
              <input type="number" name="exit_code"/>
              <button type="submit">Exit</button>
            </form>
            <form action="/ping" method="post">
              <button type="submit">Ping</button>
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

async fn env() -> HttpResponse {
    HttpResponse::Ok().json(EnvResult {
        vars: env::vars().collect(),
    })
}

async fn exit(
    parameters: web::Form<ExitParameters>,
    stop_handle: web::Data<StopHandle>,
) -> HttpResponse {
    stop_handle.stop(parameters.exit_code);
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("Bye, bye")
}

async fn ping() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("Pong")
}

async fn stdout(parameters: web::Form<PrintParameters>) -> HttpResponse {
    print!("{}", parameters.text);
    std::io::stdout().flush().ok();
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("Printed '{}' to stdout", parameters.text))
}

async fn stderr(parameters: web::Form<PrintParameters>) -> HttpResponse {
    eprint!("{}", parameters.text);
    std::io::stderr().flush().ok();
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(format!("Printed '{}' to stderr", parameters.text))
}

struct StopHandle {
    server_handle: Mutex<Option<dev::ServerHandle>>,
    exit_code: Mutex<i32>,
}

impl StopHandle {
    fn new() -> Self {
        Self {
            server_handle: Mutex::new(None),
            exit_code: Mutex::new(0),
        }
    }

    fn register_server(&self, handle: dev::ServerHandle) {
        *self.server_handle.lock().unwrap() = Some(handle);
    }

    fn stop(&self, exit_code: i32) {
        *self.exit_code.lock().unwrap() = exit_code;
        #[allow(clippy::let_underscore_future)]
        let _ = self
            .server_handle
            .lock()
            .unwrap()
            .as_ref()
            .expect("No server has been registered")
            .stop(true);
    }

    fn get_exit_code(&self) -> i32 {
        *self.exit_code.lock().unwrap()
    }
}
