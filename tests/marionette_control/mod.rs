use super::paths::MARIONETTE_PATH;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};

/// Picks a port number based on thread id to avoid using same port in tests running in parallel
fn port() -> u16 {
    // Hack since ThreadId::as_u64() is still experimental
    let id_string = format!("{:?}", std::thread::current().id());
    let id = id_string
        .strip_prefix("ThreadId(")
        .expect("Unexpected thread ID format")
        .strip_suffix(")")
        .expect("Unexpected thread ID format");
    8080 + id.parse::<u16>().unwrap()
}

/// Creates argument vector for the marionette. The first argument is always the port number to use.
pub fn app_path_and_args(additional_app_args: Vec<&str>) -> Vec<std::ffi::OsString> {
    let mut result = Vec::with_capacity(2 + additional_app_args.len());
    result.push(MARIONETTE_PATH.clone().into_os_string());
    result.push(port().to_string().into());
    for arg in additional_app_args {
        result.push(arg.into());
    }
    result
}

/// Control bar for the marionette, i.e., functions to tell the marionette program what to do
#[derive(Default)]
pub struct Bar {
    http_client: Option<reqwest::Client>,
    url: String,
}

#[derive(Deserialize, PartialEq)]
struct ArgsResult {
    args: Vec<String>,
}

#[derive(Deserialize, PartialEq)]
struct EnvResult {
    vars: Vec<(String, String)>,
}

impl Drop for Bar {
    fn drop(&mut self) {
        if self.http_client.is_some() {
            eprintln!("Marionette client is left by test. Posting exit!");
            // Use a blocking client since drop is not async
            reqwest::blocking::Client::new()
                .post(format!("{}/exit", self.url))
                .form(&HashMap::from([("exit_code", 0)]))
                .send()
                .expect("Could not post exit to marionette");
        }
    }
}

impl Bar {
    /// Creates a new control bar and waits for the marionette to open its HTTP server
    pub async fn new() -> Self {
        let result = Self {
            http_client: Some(reqwest::Client::new()),
            url: format!("http://localhost:{}", port()),
        };
        result.wait_for_marionette().await;
        result
    }

    async fn wait_for_marionette(&self) {
        for _ in 0..1000 {
            if self
                .http_client
                .as_ref()
                .expect("Marionette already shut down")
                .post(format!("{}/ping", self.url))
                .send()
                .await
                .is_ok()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("Marionette HTTP server doesn't seem to be started!");
    }

    pub async fn stdout(&self, text: &str) {
        self.post_form("stdout", ("text", text)).await;
    }

    pub async fn stderr(&self, text: &str) {
        self.post_form("stderr", ("text", text)).await;
    }

    pub async fn exit(&mut self, exit_code: i32) {
        self.post_form("exit", ("exit_code", exit_code)).await;
        self.http_client = None;
    }

    pub async fn args(&self) -> Vec<String> {
        let result: ArgsResult = serde_json::from_str(self.get_text("args").await.as_str())
            .expect("Could not deserialize args from marionette");
        result.args
    }

    pub async fn env(&self) -> Vec<(String, String)> {
        let result: EnvResult = serde_json::from_str(self.get_text("env").await.as_str())
            .expect("Could not deserialize env from marionette");
        result.vars
    }

    async fn post_form<T: serde::Serialize>(&self, command: &str, key_value: (&str, T)) {
        self.http_client
            .as_ref()
            .expect("Marionette already shut down")
            .post(format!("{}/{}", self.url, command))
            .form(&HashMap::from([key_value]))
            .send()
            .await
            .unwrap_or_else(|_| panic!("Could not post {command} to marionette"));
    }

    async fn get_text(&self, command: &str) -> String {
        self.http_client
            .as_ref()
            .expect("Marionette already shut down")
            .get(format!("{}/{}", self.url, command))
            .send()
            .await
            .unwrap_or_else(|_| panic!("Could not send get {command} to marionette"))
            .text()
            .await
            .unwrap_or_else(|_| panic!("Could not decode {command} from marionette"))
    }
}
