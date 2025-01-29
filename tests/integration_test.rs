mod assertions;
mod marionette_control;
mod paths;
mod program_under_test;

use assertions::{
    assert_command_output_end, assert_input_end, assert_near, assert_ok, assert_timeout,
};
use program_under_test::Linetime;

use std::time::Duration;
use tokio::time::timeout;

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
    let mut control = marionette_control::Bar::new().await;

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
    let mut control = marionette_control::Bar::new().await;

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
    let mut control = marionette_control::Bar::new().await;

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
async fn delta_times_can_be_shown_after_timestamp() {
    let mut put = Linetime::run(to_os(vec!["--show-delta"]));

    put.write_stdin("hello\n").await;
    let t1 = assert_ok!(put.read_stdout_timestamp());
    // Blank space left for first line instead of delta time
    assert_ok!(put.read_stdout("            : hello\n"));

    put.write_stdin("world\n").await;
    let t2 = assert_ok!(put.read_stdout_timestamp());
    let d2 = assert_ok!(put.read_stdout_delta());
    assert_ok!(put.read_stdout(": world\n"));

    assert!(t2 >= t1);
    // Allow 1 ms rounding error
    assert_near!(t2 - t1, d2, Duration::from_millis(1));

    put.close_stdin();
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout_delta());
    assert_ok!(put.read_stdout(": ␄\n"));

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
async fn input_from_stdout_is_not_buffered_to_print_complete_lines_if_flushed() {
    let mut args = to_os(vec!["--flush-all"]);
    args.append(&mut marionette_control::app_path_and_args(vec![]));
    let mut put = Linetime::run(args);
    let mut control = marionette_control::Bar::new().await;

    control.stdout("hello").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: hello"));

    control.stdout("world!\n").await;
    assert_ok!(put.read_stdout("world!\n"));

    control.exit(0).await;
    assert_command_output_end!(put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn input_from_command_is_buffered_to_print_complete_lines_even_for_stderr() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new().await;

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
async fn input_from_command_is_not_buffered_to_print_complete_lines_without_line_buffering() {
    let mut args = to_os(vec!["--no-line-buffering"]);
    args.append(&mut marionette_control::app_path_and_args(vec![]));
    let mut put = Linetime::run(args);
    let mut control = marionette_control::Bar::new().await;

    control.stderr("hello").await;
    assert_ok!(put.read_stderr_timestamp());
    assert_ok!(put.read_stderr(" stderr: hello"));
    control.stdout("hola\n").await;
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(" stdout: hola\n"));
    control.stderr("world\n").await;
    assert_ok!(put.read_stderr("world\n"));

    control.exit(0).await;
    assert_command_output_end!(&mut put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn application_exits_with_same_exit_code_as_command() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new().await;

    control.exit(17).await;
    assert_command_output_end!(&mut put);
    assert_ok!(put.read_stderr("Command exited with 17\n"));

    let exit_status = put.wait().await;
    assert!(!exit_status.success());
    assert_eq!(Some(17), exit_status.code());
}

#[tokio::test]
async fn arguments_are_forwarded_to_command() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![
        "--option", "value",
    ]));
    let mut control = marionette_control::Bar::new().await;

    let args = control.args().await;
    // Ignore program name and port argument
    assert_eq!(vec!["--option", "value"], args[2..]);

    control.exit(0).await;
    assert_command_output_end!(&mut put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn environment_variables_are_forwarded_to_command() {
    let mut put = Linetime::run_with_env(
        marionette_control::app_path_and_args(vec![]),
        vec![("variable".into(), "value".into())],
    );
    let mut control = marionette_control::Bar::new().await;

    assert_eq!(
        vec![("variable".to_string(), "value".to_string())],
        control
            .env()
            .await
            .into_iter()
            .filter(|(var, _)| var == "variable")
            .collect::<Vec<_>>()
    );

    control.exit(0).await;
    assert_command_output_end!(&mut put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn stdin_is_ignored_when_command_is_executed() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let mut control = marionette_control::Bar::new().await;

    put.write_stdin("ignored line").await;
    assert_timeout!(put.read_stdout_timestamp());

    control.exit(0).await;
    assert_command_output_end!(&mut put);

    assert!(put.wait().await.success());
}

#[tokio::test]
async fn escape_sequence_to_move_cursor_is_swallowed_to_unfold_lines() {
    let mut put = Linetime::run(vec![]);

    put.write_stdin("hello").await;
    // ESC[2K = erase entire line
    put.write_stdin("\x1b[2K").await;
    // ESC[H = move cursor home
    put.write_stdin("\x1b[H").await;
    put.write_stdin("world\n").await;

    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(": hello\n"));
    // New line and timestamp, since unfolding instead of printing erase or cursor escape sequences
    assert_ok!(put.read_stdout_timestamp());
    assert_ok!(put.read_stdout(": world\n"));

    put.close_stdin();
    assert_input_end!(put);

    assert!(put.wait().await.success());
}
