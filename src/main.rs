// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 inukai-norio <inukay@proton.me>

mod cli;

use std::{
    fmt::Display,
    io::{self, Write},
    process::ExitCode,
};

use generate_passwd::OsRandom;

fn main() -> ExitCode {
    match cli::run(std::env::args_os().skip(1), &mut OsRandom) {
        Ok(report) => match write_report(&report) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                write_error(format_args!("出力に失敗しました: {error}"));
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            write_error(&error);
            error.exit_code()
        }
    }
}

fn write_report(report: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(report.as_bytes())?;
    stdout.flush()
}

fn write_error(error: impl Display) {
    // エラーの表示先も書けない場合、再度エラーを表示しようとはしない。
    let _ = writeln!(io::stderr().lock(), "{error}");
}
