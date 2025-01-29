use super::paths::LINETIME_PATH;
use regex::Regex;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};

/// A wrapper to run the linetime program with some arguments. It provides functions to expect
/// output on stdout and stderr.
pub struct Linetime {
    process: Child,
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
    stderr: ChildStderr,
    timestamp_regex: Regex,
    delta_regex: Regex,
}

impl Drop for Linetime {
    fn drop(&mut self) {
        if self.process.try_wait().unwrap().is_none() {
            eprintln!("Linetime process left by test. Attempting to kill!");
            // Use synchronous start_kill()/try_wait() since drop is not async
            self.process.start_kill().unwrap();
            for _ in 0..100 {
                if self.process.try_wait().unwrap().is_some() {
                    eprintln!("Linetime process killed successfully!");
                    return;
                }
                // Synchronous sleep with std, locks async runtime
                std::thread::sleep(Duration::from_millis(100));
            }
            eprintln!("Failed to kill linetime process");
        }
    }
}

impl Linetime {
    /// Runs linetime with the provided arguments
    pub fn run(args: Vec<std::ffi::OsString>) -> Self {
        Self::run_with_env(args, vec![])
    }

    /// Runs linetime with the provided arguments and environment variables
    pub fn run_with_env(
        args: Vec<std::ffi::OsString>,
        env_vars: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    ) -> Self {
        let mut process = tokio::process::Command::new(LINETIME_PATH.as_path())
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(env_vars)
            .spawn()
            .expect("Should be able to run linetime");
        let stdin = Some(process.stdin.take().unwrap());
        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();
        Self {
            process,
            stdin,
            stdout,
            stderr,
            timestamp_regex: Regex::new("([0-9]+):([0-9]+).([0-9]+)").unwrap(),
            delta_regex: Regex::new(" \\(([0-9]+):([0-9]+).([0-9]+)\\)").unwrap(),
        }
    }

    /// Writes to the program's stdin
    pub async fn write_stdin(&mut self, text: &str) {
        let Some(stdin) = &mut self.stdin else {
            panic!("Linetime stdin has already been closed!");
        };
        stdin
            .write_all(text.as_bytes())
            .await
            .expect("Could not write to linetime stdin");
        stdin.flush().await.expect("Could not flush linetime stdin");
    }

    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// Reads a timestamp from the program's stdout and returns it as a Duration
    pub async fn read_stdout_timestamp(&mut self) -> Result<Duration, std::io::Error> {
        Self::read_timestamp(&mut self.stdout, &self.timestamp_regex, "stdout").await
    }

    /// Reads a timestamp from the program's stdout and returns it as a Duration
    pub async fn read_stderr_timestamp(&mut self) -> Result<Duration, std::io::Error> {
        Self::read_timestamp(&mut self.stderr, &self.timestamp_regex, "stderr").await
    }

    /// Reads a delta time from the program's stdout and returns it as a Duration
    pub async fn read_stdout_delta(&mut self) -> Result<Duration, std::io::Error> {
        Self::read_delta(&mut self.stdout, &self.delta_regex, "stdout").await
    }

    /// Reads a delta time from the program's stderr and returns it as a Duration
    pub async fn read_stderr_delta(&mut self) -> Result<Duration, std::io::Error> {
        Self::read_delta(&mut self.stderr, &self.delta_regex, "stderr").await
    }

    /// Reads some text from the program's stdout and checks that it matches the expected text,
    /// otherwise it returns an error
    pub async fn read_stdout(&mut self, expected_text: &str) -> Result<(), std::io::Error> {
        let read_text = Self::read(&mut self.stdout, expected_text.len(), "stdout").await?;
        if read_text == expected_text {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "Expected to read '{expected_text}' from stdout but read '{read_text}'"
            )))
        }
    }

    /// Reads some text from the program's stderr and checks that it matches the expected text,
    /// otherwise it returns an error
    pub async fn read_stderr(&mut self, expected_text: &str) -> Result<(), std::io::Error> {
        let read_text = Self::read(&mut self.stderr, expected_text.len(), "stderr").await?;
        if read_text == expected_text {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "Expected to read '{expected_text}' from stderr but read '{read_text}'"
            )))
        }
    }

    /// Waits for program to end and checks that nothing more can be read from its stdout and stderr
    pub async fn wait(&mut self) -> std::process::ExitStatus {
        let mut stdout_rest = String::new();
        if self
            .stdout
            .read_to_string(&mut stdout_rest)
            .await
            .expect("Could not convert left-overs on linetime stdout to UTF-8")
            != 0
        {
            panic!("Nothing should be left on linetime stdout, but found '{stdout_rest}'");
        }

        let mut stderr_rest = String::new();
        if self
            .stderr
            .read_to_string(&mut stderr_rest)
            .await
            .expect("Could not convert left-overs on linetime stderr to UTF-8")
            != 0
        {
            panic!("Nothing should be left on linetime stderr, but found '{stderr_rest}'");
        }

        self.process
            .wait()
            .await
            .expect("Could not wait for linetime process to exit")
    }

    async fn read<R>(
        reader: &mut R,
        size: usize,
        reader_name: &str,
    ) -> Result<String, std::io::Error>
    where
        R: AsyncReadExt,
        R: Unpin,
    {
        let mut buffer = vec![0; size];
        reader.read_exact(&mut buffer).await?;
        Ok(String::from_utf8(buffer).unwrap_or_else(|error| panic!("Read {size} bytes from linetime {reader_name} but could not convert to UTF-8: {error}")))
    }

    async fn read_timestamp<R>(
        reader: &mut R,
        regex: &Regex,
        reader_name: &str,
    ) -> Result<Duration, std::io::Error>
    where
        R: AsyncReadExt,
        R: Unpin,
    {
        let read_text = Self::read(reader, 9, reader_name).await?;
        let Some(captures) = regex.captures(read_text.as_str()) else {
            return Err(std::io::Error::other(format!(
                "Could not find timestamp in linetime {reader_name} text '{read_text}'"
            )));
        };
        assert_eq!(4, captures.len());
        Ok(
            Duration::from_secs(60 * get_u64(&captures, 1) + get_u64(&captures, 2))
                + Duration::from_millis(get_u64(&captures, 3)),
        )
    }

    async fn read_delta<R>(
        reader: &mut R,
        regex: &Regex,
        reader_name: &str,
    ) -> Result<Duration, std::io::Error>
    where
        R: AsyncReadExt,
        R: Unpin,
    {
        let read_text = Self::read(reader, 12, reader_name).await?;
        let Some(captures) = regex.captures(read_text.as_str()) else {
            return Err(std::io::Error::other(format!(
                "Could not find delta time in linetime {reader_name} text '{read_text}'"
            )));
        };
        assert_eq!(4, captures.len());
        Ok(
            Duration::from_secs(60 * get_u64(&captures, 1) + get_u64(&captures, 2))
                + Duration::from_millis(get_u64(&captures, 3)),
        )
    }
}

fn get_u64(captures: &regex::Captures, i: usize) -> u64 {
    captures.get(i).unwrap().as_str().parse::<u64>().unwrap()
}
