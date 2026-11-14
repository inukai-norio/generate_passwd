use std::{collections::HashSet, ffi::OsString, fmt, process::ExitCode, str::FromStr};

use generate_passwd::{
    Base10, Base26, Base36, Base62, ConstraintError, LengthPolicy, PasswordBase, PasswordOptions,
    PasswordPlan, RandomSource, plan_password,
};

const HELP: &str = "\
使い方: generate_passwd [オプション]

  --length N          ランダム部分を正確にN文字にする
  --min-entropy BITS  指定ビット数以上になる最少のランダム文字数を使う
  --max-length M      ハイフン込みの出力全体をM文字以下にする
  --group-size G      G文字ごとにハイフンを入れる（既定: 4、0: 区切りなし）
  --base BASE         10 / 26 / 36 / 62 / all（既定: all）
  -h, --help          この説明を表示する

--lengthと--min-entropyは同時指定できません。
--max-lengthだけの長さ指定では、上限内で最多のランダム文字を使います。
長さ関係の指定がなければ、ランダム部分20文字で生成します。
文字数・上限は正の整数、最小エントロピーは有限の正の数を指定します。
ハイフンはエントロピーに寄与しません。値を取るオプションは重複指定できません。

例: generate_passwd --base 62 --min-entropy 128 --max-length 32

終了コード: 0 成功・ヘルプ、1 乱数取得・出力エラー、2 引数・生成条件エラー
";

#[derive(Debug)]
pub(crate) enum CliError {
    Input(String),
    Random(getrandom::Error),
}

impl CliError {
    pub(crate) fn exit_code(&self) -> ExitCode {
        match self {
            Self::Input(_) => ExitCode::from(2),
            Self::Random(_) => ExitCode::FAILURE,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(message) => write!(f, "{message}"),
            Self::Random(error) => write!(f, "乱数の取得に失敗しました: {error}"),
        }
    }
}

#[derive(Clone, Copy)]
enum Base {
    Base10,
    Base26,
    Base36,
    Base62,
}

impl Base {
    fn label(self) -> &'static str {
        match self {
            Self::Base10 => "Base10",
            Self::Base26 => "Base26",
            Self::Base36 => "Base36",
            Self::Base62 => "Base62",
        }
    }
}

enum Command {
    Help,
    Generate {
        options: PasswordOptions,
        bases: Vec<Base>,
    },
}

// 文字集合の異なる型付き計画を、型の対応を保ったまままとめて保持する。
enum PlannedPassword {
    Base10(PasswordPlan<Base10>),
    Base26(PasswordPlan<Base26>),
    Base36(PasswordPlan<Base36>),
    Base62(PasswordPlan<Base62>),
}

impl PlannedPassword {
    fn prepare(base: Base, options: &PasswordOptions) -> Result<Self, ConstraintError> {
        match base {
            Base::Base10 => plan_password::<Base10>(options).map(Self::Base10),
            Base::Base26 => plan_password::<Base26>(options).map(Self::Base26),
            Base::Base36 => plan_password::<Base36>(options).map(Self::Base36),
            Base::Base62 => plan_password::<Base62>(options).map(Self::Base62),
        }
    }

    fn generate_report(&self, random: &mut impl RandomSource) -> Result<String, getrandom::Error> {
        match self {
            Self::Base10(plan) => generate_report("Base10", plan, random),
            Self::Base26(plan) => generate_report("Base26", plan, random),
            Self::Base36(plan) => generate_report("Base36", plan, random),
            Self::Base62(plan) => generate_report("Base62", plan, random),
        }
    }
}

fn generate_report<B: PasswordBase>(
    label: &str,
    plan: &PasswordPlan<B>,
    random: &mut impl RandomSource,
) -> Result<String, getrandom::Error> {
    let password = plan.generate_with(random)?;
    let entropy_bits = plan.entropy_bits();
    let output_length = password.chars().count();
    let length = plan.random_length();
    Ok(format!(
        "{label}: {password}\n\
         エントロピー: {entropy_bits:.2} ビット\n\
         出力文字数: {output_length}（区切り文字込み、ランダム部分: {length} 文字）\n\n"
    ))
}

/// 全条件の検証と全件の生成が成功してから、表示用の文字列を返す。
pub(crate) fn run(
    args: impl IntoIterator<Item = OsString>,
    random: &mut impl RandomSource,
) -> Result<String, CliError> {
    let Command::Generate { options, bases } = parse(args)? else {
        return Ok(HELP.to_owned());
    };
    let mut plans = Vec::new();
    let mut errors = Vec::new();
    for base in bases {
        match PlannedPassword::prepare(base, &options) {
            Ok(plan) => plans.push(plan),
            Err(error) => errors.push(format!("{}: {error}", base.label())),
        }
    }
    if !errors.is_empty() {
        return Err(CliError::Input(errors.join("\n")));
    }
    let mut report = String::new();
    for plan in plans {
        report.push_str(&plan.generate_report(random).map_err(CliError::Random)?);
    }
    Ok(report)
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, CliError> {
    let mut args = args.into_iter();
    let defaults = PasswordOptions::default();
    let mut seen = HashSet::new();
    let mut length = None;
    let mut min_entropy = None;
    let mut max_output_length = None;
    let mut group_size = defaults.group_size;
    let mut bases = all_bases();

    while let Some(arg) = args.next() {
        let arg = argument_string(arg)?;
        if arg == "--help" || arg == "-h" {
            return Ok(Command::Help);
        }
        let (name, inline_value) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (arg.as_str(), None),
        };
        if !matches!(
            name,
            "--length" | "--min-entropy" | "--max-length" | "--group-size" | "--base"
        ) {
            return Err(unknown_argument(name));
        }
        if !seen.insert(name.to_owned()) {
            return Err(CliError::Input(format!("{name}は一度だけ指定してください")));
        }
        let value = match inline_value {
            Some(value) => value.to_owned(),
            None => {
                let value = args.next().ok_or_else(|| missing_value(name))?;
                let value = argument_string(value)?;
                if value.starts_with("--") || value == "-h" {
                    return Err(missing_value(name));
                }
                value
            }
        };
        match name {
            "--length" => length = Some(positive_integer(name, &value)?),
            "--max-length" => max_output_length = Some(positive_integer(name, &value)?),
            "--group-size" => group_size = number(name, &value)?,
            "--min-entropy" => {
                let bits: f64 = number(name, &value)?;
                if !bits.is_finite() || bits <= 0.0 {
                    return Err(CliError::Input(
                        "--min-entropyには有限の正の値を指定してください".to_owned(),
                    ));
                }
                min_entropy = Some(bits);
            }
            "--base" => bases = parse_bases(&value)?,
            _ => return Err(unknown_argument(name)),
        }
    }

    let length_policy = match (length, min_entropy, max_output_length) {
        (Some(_), Some(_), _) => {
            return Err(CliError::Input(
                "--lengthと--min-entropyは同時指定できません".to_owned(),
            ));
        }
        (Some(length), None, _) => LengthPolicy::Exact(length),
        (None, Some(bits), _) => LengthPolicy::MinEntropy(bits),
        (None, None, Some(_)) => LengthPolicy::FitToMaxOutput,
        (None, None, None) => defaults.length_policy,
    };
    Ok(Command::Generate {
        options: PasswordOptions {
            length_policy,
            max_output_length,
            group_size,
        },
        bases,
    })
}

fn all_bases() -> Vec<Base> {
    vec![Base::Base10, Base::Base26, Base::Base36, Base::Base62]
}

fn parse_bases(value: &str) -> Result<Vec<Base>, CliError> {
    match value {
        "10" => Ok(vec![Base::Base10]),
        "26" => Ok(vec![Base::Base26]),
        "36" => Ok(vec![Base::Base36]),
        "62" => Ok(vec![Base::Base62]),
        "all" => Ok(all_bases()),
        _ => Err(CliError::Input(
            "--baseには10 / 26 / 36 / 62 / allを指定してください".to_owned(),
        )),
    }
}

fn argument_string(arg: OsString) -> Result<String, CliError> {
    arg.into_string()
        .map_err(|_| CliError::Input("引数にはUTF-8の文字列を指定してください".to_owned()))
}

fn number<T: FromStr>(name: &str, value: &str) -> Result<T, CliError> {
    value
        .parse()
        .map_err(|_| CliError::Input(format!("{name}の値が不正です: {value}")))
}

fn positive_integer(name: &str, value: &str) -> Result<usize, CliError> {
    let value = number(name, value)?;
    if value == 0 {
        return Err(CliError::Input(format!(
            "{name}には1以上を指定してください"
        )));
    }
    Ok(value)
}

fn unknown_argument(name: &str) -> CliError {
    CliError::Input(format!(
        "不明な引数です: {name}（--helpで使い方を確認できます）"
    ))
}

fn missing_value(name: &str) -> CliError {
    CliError::Input(format!("{name}の値が必要です"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingRandom {
        calls: usize,
        fail_on: Option<usize>,
    }

    impl RandomSource for CountingRandom {
        fn fill(&mut self, buffer: &mut [u8]) -> Result<(), getrandom::Error> {
            self.calls += 1;
            if self.fail_on == Some(self.calls) {
                return Err(getrandom::Error::UNEXPECTED);
            }
            buffer.fill(0);
            Ok(())
        }
    }

    #[test]
    fn all_invalid_base_plans_are_reported_before_random_is_requested() {
        let mut random = CountingRandom {
            calls: 0,
            fail_on: None,
        };
        let result = run(
            ["--min-entropy", "128", "--max-length", "32"].map(OsString::from),
            &mut random,
        );
        let Err(CliError::Input(message)) = result else {
            panic!("条件エラーを期待")
        };
        assert!(message.contains("Base10:") && message.contains("48文字"));
        assert!(message.contains("Base26:") && message.contains("34文字"));
        assert!(!message.contains("Base36:") && !message.contains("Base62:"));
        assert_eq!(random.calls, 0);
    }

    #[test]
    fn random_error_after_one_base_does_not_return_a_partial_report() {
        let mut random = CountingRandom {
            calls: 0,
            fail_on: Some(2),
        };
        assert!(matches!(
            run([], &mut random),
            Err(CliError::Random(getrandom::Error::UNEXPECTED))
        ));
        assert_eq!(random.calls, 2);
    }

    #[test]
    fn invalid_arguments_and_help_do_not_request_random_bytes() {
        for args in [
            vec!["--length", "20", "--min-entropy", "128"],
            vec!["--length"],
            vec!["--length", "0"],
            vec!["--base", "64"],
            vec!["--unknown"],
        ] {
            let mut random = CountingRandom {
                calls: 0,
                fail_on: Some(1),
            };
            assert!(matches!(
                run(args.into_iter().map(OsString::from), &mut random),
                Err(CliError::Input(_))
            ));
            assert_eq!(random.calls, 0);
        }
        let mut random = CountingRandom {
            calls: 0,
            fail_on: Some(1),
        };
        assert_eq!(run([OsString::from("--help")], &mut random).unwrap(), HELP);
        assert_eq!(random.calls, 0);
    }
}
