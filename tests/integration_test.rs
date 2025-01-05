mod marionette_control;
mod program_under_test;

use program_under_test::Linetime;

/// Expect program to print that end is reached of stdin
fn expect_input_end(put: &mut Linetime) {
    put.expect_stdout_timestamp();
    put.expect_stdout(": ␄\n");
}

/// Expect program to print that end is reached of both stdout and stderr of the command executed
/// by the program
fn expect_command_output_end(put: &mut Linetime) {
    put.expect_stdout_timestamp();
    put.expect_stdout(" stdout: ␄\n");
    put.expect_stderr_timestamp();
    put.expect_stderr(" stderr: ␄\n");
}

#[test]
fn stdin_is_read_if_no_command_is_executed_by_program() {
    let mut put = Linetime::run(vec![]);

    put.write_stdin("hello\n");
    put.expect_stdout_timestamp();
    put.expect_stdout(": hello\n");

    put.close_stdin();
    expect_input_end(&mut put);

    assert!(put.wait().success());
}

#[test]
fn stdout_from_command_is_read_when_command_is_executed() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let control = marionette_control::Bar::new();

    control.stdout("hello\n");
    put.expect_stdout_timestamp();
    put.expect_stdout(" stdout: hello\n");

    control.exit(0);
    expect_command_output_end(&mut put);

    assert!(put.wait().success());
}

#[test]
fn stderr_from_command_is_read_when_command_is_executed() {
    let mut put = Linetime::run(marionette_control::app_path_and_args(vec![]));
    let control = marionette_control::Bar::new();

    control.stderr("hello\n");
    put.expect_stderr_timestamp();
    put.expect_stderr(" stderr: hello\n");

    control.exit(0);
    expect_command_output_end(&mut put);

    assert!(put.wait().success());
}
