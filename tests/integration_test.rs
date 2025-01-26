mod marionette_control;
mod paths;
mod program_under_test;

use program_under_test::Linetime;

use std::time::Duration;
use tokio::time::timeout;

/// Macro that wraps a future returning a result in a timeout and asserts that the future is
/// succesful and does not timeout
macro_rules! assert_ok {
    ( $future:expr ) => {
        match timeout(Duration::from_secs(3), $future).await {
            Ok(timeout_result) => match timeout_result {
                Ok(result) => result,
                Err(error) => {
                    panic!(
                        "Operation '{}' should be successful but it failed with: {}",
                        stringify!($future),
                        error
                    );
                }
            },
            Err(_) => {
                panic!(
                    "Operation '{}' should be successful but it timeout out",
                    stringify!($future),
                );
            }
        }
    };
}

/// Macro that wraps a future in a timeout and asserts that the future likely would block
/// indefinetely, by testing it with a short timeout
macro_rules! assert_timeout {
    ( $future:expr ) => {
        match timeout(Duration::from_millis(500), $future).await {
            Ok(timeout_result) => panic!(
                "Operation '{}' should block but returned with: {:?}",
                stringify!($future),
                timeout_result
            ),
            Err(_) => {
                // This is what we expect!
            }
        }
    };
}

/// Expect program to print that end is reached of stdin
macro_rules! assert_input_end {
    ( $put:expr ) => {
        assert_ok!($put.read_stdout_timestamp());
        assert_ok!($put.read_stdout(": ␄\n"));
    };
}

/// Expect program to print that end is reached of both stdout and stderr of the command executed
/// by the program
macro_rules! assert_command_output_end {
    ( $put:expr ) => {
        assert_ok!($put.read_stdout_timestamp());
        assert_ok!($put.read_stdout(" stdout: ␄\n"));
        assert_ok!($put.read_stderr_timestamp());
        assert_ok!($put.read_stderr(" stderr: ␄\n"));
    };
}

fn to_os(strings: Vec<&str>) -> Vec<std::ffi::OsString> {
    strings.into_iter().map(|s| s.to_string().into()).collect()
}

#[tokio::test]
async fn stdin_is_read_when_no_command_is_executed() {
    let mut put = Linetime::run(vec![]);

    put.write_stdin("hello\n").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(": hello\n"));

    put.close_stdin();
    assert_input_end!(put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn stdout_from_command_is_read_when_command_is_executed() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new();

    control.stdout("hello\n").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: hello\n"));

    control.exit(0).await;
    assert_command_output_end!(put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn stderr_from_command_is_read_when_command_is_executed() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new();

    control.stderr("hello\n").await;
    assert_ok!(put.read_stderr_timestamp());
    assert_ok!(put.read_stderr(" stderr: hello\n"));

    control.exit(0).await;
    assert_command_output_end!(put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn output_lines_get_ordererd_timestamps() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new();

    control.stdout("hello\n").await;
    let t1 = assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: hello\n"));

    control.stdout("world\n").await;
    let t2 = assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: world\n"));
    assert!(t2 >= t1);

    control.exit(0).await;
    assert_command_output_end!(&mut put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn input_from_stdin_is_not_buffered_to_print_complete_lines_if_flushed() {
    let mut put = Linetime::run(to_os(vec!["--flush-all"]));

    put.write_stdin("hello").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(": hello"));

    put.write_stdin("world!\n").await;
    assert_ok!(put.read_stdout("world!\n"));

    put.close_stdin();
    assert_input_end!(put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn input_from_command_is_buffered_to_print_complete_lines_even_for_stderr() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new();

    control.stdout("aaa").await;
    assert_timeout!(put.read_stdout_timestamp());
    control.stderr("bbb\n").await;
    assert_ok!(put.read_stderr_timestamp());
    assert_ok!(put.read_stderr(" stderr: bbb\n"));
    control.stdout("ccc\n").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: aaaccc\n"));

    control.stderr("aaa").await;
    assert_timeout!(put.read_stderr_timestamp());
    control.stdout("bbb\n").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: bbb\n"));
    control.stderr("ccc\n").await;
    assert_ok!(put.read_stderr_timestamp());
    assert_ok!(put.read_stderr(" stderr: aaaccc\n"));

    control.exit(0).await;
    assert_command_output_end!(&mut put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn application_exits_with_same_exit_code_as_command() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new();

    control.exit(17).await;
    assert_command_output_end!(&mut put);
    assert_ok!(put.read_stderr("Command exited with 17\n"));

    let exit_status = put.wait().await;
    assert!(!exit_status.success());
    assert_eq!(Some(17), exit_status.code());
}
