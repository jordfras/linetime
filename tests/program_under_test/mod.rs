use super::paths::LINETIME_PATH;
use regex::Regex;
use std::io::{Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio};
use std::time::Duration;

/// A wrapper to run the linetime program with some arguments. It provides functions to expect
/// output on stdout and stderr. These functions are _blocking_, i.e., if the amount of
/// expected text cannot be read the function will _not_ return.
///
/// I might consider turning the entire integration test framework into async with tokio at a
/// later stage.
pub struct Linetime {
    process: Child,
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
    stderr: ChildStderr,
    timestamp_regex: Regex,
}

impl Linetime {
    /// Runs linetime with the provided arguments
    pub fn run(args: Vec<std::ffi::OsString>) -> Self {
        let mut process = std::process::Command::new(LINETIME_PATH.as_path())
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
        }
    }

    /// Writes to the program's stdin
    pub fn write_stdin(&mut self, text: &str) {
        let Some(stdin) = &mut self.stdin else {
            panic!("Linetime stdin has already been closed!");
        };
        stdin
            .write_all(text.as_bytes())
            .expect("Could not write to linetime stdin");
        stdin.flush().expect("Could not flush linetime stdin");
    }

    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// Reads a timestamp from the program's stdout and returns it as a Duration, or panics if not
    /// possible to parse as a timestamp
    pub fn expect_stdout_timestamp(&mut self) -> Duration {
        Self::expect_timestamp(&mut self.stdout, &self.timestamp_regex, "stdout")
    }

    /// Reads a timestamp from the program's stderr and returns it as a Duration, or panics if not
    /// possible to parse as a timestamp
    pub fn expect_stderr_timestamp(&mut self) -> Duration {
        Self::expect_timestamp(&mut self.stderr, &self.timestamp_regex, "stdout")
    }

    /// Reads some text from the program's stdout and checks that it matches the expected text,
    /// otherwise it panics
    pub fn expect_stdout(&mut self, expected_text: &str) {
        let read_text = Self::read(&mut self.stdout, expected_text.len(), "stdout");
        assert_eq!(
            expected_text, read_text,
            "Expected to read '{expected_text}' from linetime stdout"
        );
    }

    /// Reads some text from the program's stderr and checks that it matches the expected text,
    /// otherwise it panics
    pub fn expect_stderr(&mut self, expected_text: &str) {
        let read_text = Self::read(&mut self.stderr, expected_text.len(), "stderr");
        assert_eq!(
            expected_text, read_text,
            "Expected to read '{expected_text}' from linetime stderr"
        );
    }

    /// Waits for program to end and checks that nothing more can be read from its stdout and stderr
    pub fn wait(&mut self) -> std::process::ExitStatus {
        let mut stdout_rest = String::new();
        if self.stdout.read_to_string(&mut stdout_rest).expect("") != 0 {
            panic!("Nothing should be left on linetime stdout, but found '{stdout_rest}'");
        }

        let mut stderr_rest = String::new();
        if self.stdout.read_to_string(&mut stderr_rest).expect("") != 0 {
            panic!("Nothing should be left on linetime stderr, but found '{stderr_rest}'");
        }

        self.process
            .wait()
            .expect("Could not wait for linetime process to exit")
    }

    fn read(reader: &mut dyn Read, size: usize, reader_name: &str) -> String {
        let mut buffer = vec![0; size];
        reader
            .read_exact(&mut buffer)
            .unwrap_or_else(|error: std::io::Error| {
                panic!("Reading {size} bytes from linetime {reader_name} failed: {error}")
            });
        String::from_utf8(buffer).unwrap_or_else(|error| panic!("Read {size} bytes from linetime {reader_name} but could not convert to UTF-8: {error}"))
    }

    fn expect_timestamp(reader: &mut dyn Read, regex: &Regex, reader_name: &str) -> Duration {
        let read_text = Self::read(reader, 9, reader_name);
        let Some(captures) = regex.captures(read_text.as_str()) else {
            panic!("Could not find timestamp in linetime {reader_name} text '{read_text}'");
        };
        assert_eq!(4, captures.len());
        Duration::from_secs(60 * get_u64(&captures, 1) + get_u64(&captures, 2))
            + Duration::from_millis(get_u64(&captures, 3))
    }
}

fn get_u64(captures: &regex::Captures, i: usize) -> u64 {
    captures.get(i).unwrap().as_str().parse::<u64>().unwrap()
}
