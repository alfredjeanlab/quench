// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use yare::parameterized;

#[test]
fn config_error_display() {
    let err = Error::Config {
        message: "invalid version".into(),
        path: Some(PathBuf::from("quench.toml")),
    };
    assert!(err.to_string().contains("invalid version"));
}

#[test]
fn exit_code_from_config_error() {
    let err = Error::Config { message: "test".into(), path: None };
    assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);
}

#[test]
fn exit_code_from_argument_error() {
    let err = Error::Argument("unknown flag".into());
    assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);
}

#[test]
fn exit_code_from_internal_error() {
    let err = Error::Internal("bug".into());
    assert_eq!(ExitCode::from(&err), ExitCode::InternalError);
}

#[parameterized(
    config = { Error::Config { message: "x".into(), path: None }, ExitCode::ConfigError },
    argument = { Error::Argument("x".into()), ExitCode::ConfigError },
    internal = { Error::Internal("x".into()), ExitCode::InternalError },
)]
fn exit_code_mapping(err: Error, expected: ExitCode) {
    assert_eq!(ExitCode::from(&err), expected);
}
