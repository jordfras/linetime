use crate::error::{Result, ResultExt};
use std::process::Stdio;

pub struct Runner {
    command: std::process::Command,
    child: Option<std::process::Child>,
}

impl Runner {
    pub fn new(command_and_args: &[String]) -> Self {
        let mut command = std::process::Command::new(command_and_args[0].as_str());
        command
            .args(&command_and_args[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Self {
            command,
            child: None,
        }
    }

    /// Spawns a child process executing the command
    pub fn spawn(&mut self) -> Result<()> {
        assert!(self.child.is_none());
        self.child = Some(
            self.command
                .spawn()
                .error_context("Failed to execute command")?,
        );
        Ok(())
    }

    pub fn stdout(&mut self) -> std::process::ChildStdout {
        assert!(self.child.is_some());
        self.child
            .as_mut()
            .unwrap()
            .stdout
            .take()
            .expect("You can only get stdout for command once")
    }

    pub fn stderr(&mut self) -> std::process::ChildStderr {
        assert!(self.child.is_some());
        self.child
            .as_mut()
            .unwrap()
            .stderr
            .take()
            .expect("You can only get stderr for command once")
    }

    /// Waits for the command to finish
    pub fn wait(&mut self) {
        assert!(self.child.is_some());
        self.child
            .as_mut()
            .unwrap()
            .wait()
            .expect("Command expected to run");
    }

    /// Exits this program with the same status code as the command unless it was successful
    pub fn exit_if_failed(&mut self) -> Result<()> {
        assert!(self.child.is_some());
        if let Some(status) = self
            .child
            .as_mut()
            .unwrap()
            .try_wait()
            .error_context("Failed to get exit code for command")?
        {
            if !status.success() {
                if let Some(code) = status.code() {
                    eprintln!("Command exited with {code}");
                    // Exit with same code as underlying program
                    std::process::exit(code);
                } else {
                    eprintln!("Command terminated by signal");
                }
            }
        } else {
            panic!("wait() should be called before exit_if_failed() on CommandRunner");
        }
        Ok(())
    }
}
