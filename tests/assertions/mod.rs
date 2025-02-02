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
pub(crate) use assert_ok;

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
pub(crate) use assert_timeout;

/// Expect program to print that end is reached of stdin
macro_rules! assert_input_end {
    ( $put:expr ) => {
        assert_ok!($put.read_stdout_timestamp());
        assert_ok!($put.read_stdout(": ␃\n"));
    };
}
pub(crate) use assert_input_end;

/// Expect program to print that end is reached of both stdout and stderr of the command executed
/// by the program
macro_rules! assert_command_output_end {
    ( $put:expr ) => {
        assert_ok!($put.read_stdout_timestamp());
        assert_ok!($put.read_stdout(" ------: ␃\n"));
    };
}
pub(crate) use assert_command_output_end;

macro_rules! assert_near {
    ( $expected:expr, $actual:expr, $delta:expr ) => {
        assert!($expected + $delta >= $actual);
        assert!($expected <= $actual + $delta);
    };
}
pub(crate) use assert_near;
