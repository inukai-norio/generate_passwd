use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_generate_passwd"))
        .args(args)
        .output()
        .unwrap()
}

fn password(output: &Output, label: &str) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let prefix = format!("{label}: ");
    std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .to_owned()
}

#[test]
fn no_arguments_preserve_all_bases_and_output_format() {
    let output = run(&[]);
    let text = std::str::from_utf8(&output.stdout).unwrap();
    let blocks: Vec<_> = text.trim().split("\n\n").collect();
    assert_eq!(blocks.len(), 4);
    for (index, label, alphabet, entropy) in [
        (0, "Base10", "0123456789", "66.44"),
        (1, "Base26", "abcdefghijklmnopqrstuvwxyz", "94.01"),
        (
            2,
            "Base36",
            "0123456789abcdefghijklmnopqrstuvwxyz",
            "103.40",
        ),
        (
            3,
            "Base62",
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            "119.08",
        ),
    ] {
        let pass = password(&output, label);
        assert!(blocks[index].starts_with(label));
        assert_eq!(blocks[index].lines().count(), 3);
        assert_eq!(pass.len(), 24);
        assert_eq!(
            pass.split('-').map(str::len).collect::<Vec<_>>(),
            vec![4; 5]
        );
        assert!(
            pass.bytes()
                .all(|byte| byte == b'-' || alphabet.as_bytes().contains(&byte))
        );
        assert!(blocks[index].contains(&format!("エントロピー: {entropy} ビット")));
        assert!(blocks[index].contains("出力文字数: 24（区切り文字込み、ランダム部分: 20 文字）"));
    }
}

#[test]
fn minimum_entropy_uses_the_shortest_length_within_the_limit() {
    let output = run(&["--base", "62", "--min-entropy", "128", "--max-length", "32"]);
    assert_eq!(password(&output, "Base62").len(), 27);
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert!(text.contains("ランダム部分: 22 文字"));
    assert!(text.contains("エントロピー: 130.99 ビット"));
    assert!(!text.contains("Base10:"));

    let output = run(&[
        "--base=62",
        "--min-entropy=128.5",
        "--max-length=22",
        "--group-size=0",
    ]);
    let pass = password(&output, "Base62");
    assert_eq!(pass.len(), 22);
    assert!(!pass.contains('-'));
}

#[test]
fn all_bases_choose_their_own_minimum_entropy_lengths() {
    let output = run(&["--min-entropy", "128"]);
    let text = std::str::from_utf8(&output.stdout).unwrap();
    let blocks: Vec<_> = text.trim().split("\n\n").collect();
    assert_eq!(blocks.len(), 4);
    for (index, label, random_length, output_length) in [
        (0, "Base10", 39, 48),
        (1, "Base26", 28, 34),
        (2, "Base36", 25, 31),
        (3, "Base62", 22, 27),
    ] {
        assert_eq!(password(&output, label).len(), output_length);
        assert!(blocks[index].contains(&format!("ランダム部分: {random_length} 文字")));
    }
}

#[test]
fn maximum_only_and_exact_lengths_have_distinct_behaviors() {
    for (args, output_length) in [
        (vec!["--base", "10", "--max-length", "5"], 4),
        (vec!["--base", "10", "--max-length", "24"], 24),
        (
            vec!["--base", "10", "--length", "3", "--max-length", "5"],
            3,
        ),
        (
            vec!["--base", "10", "--max-length", "5", "--group-size", "0"],
            5,
        ),
        (
            vec!["--base", "10", "--length", "3", "--group-size", "1"],
            5,
        ),
    ] {
        assert_eq!(password(&run(&args), "Base10").len(), output_length);
    }
}

#[test]
fn selecting_a_base_or_group_size_alone_preserves_the_default_random_length() {
    for base in ["10", "26", "36", "62"] {
        let output = run(&["--base", base, "--group-size", "0"]);
        let pass = password(&output, &format!("Base{base}"));
        assert_eq!(pass.len(), 20);
        assert!(!pass.contains('-'));
        let text = std::str::from_utf8(&output.stdout).unwrap();
        assert_eq!(text.trim().split("\n\n").count(), 1);
    }
    let output = run(&["--group-size", "0"]);
    for label in ["Base10", "Base26", "Base36", "Base62"] {
        assert_eq!(password(&output, label).len(), 20);
    }
}

#[test]
fn incompatible_all_base_request_reports_every_failure_and_no_passwords() {
    let output = run(&[
        "--base",
        "all",
        "--min-entropy",
        "128",
        "--max-length",
        "32",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error = std::str::from_utf8(&output.stderr).unwrap();
    assert!(error.contains("Base10:") && error.contains("48文字"));
    assert!(error.contains("Base26:") && error.contains("34文字"));
    assert!(error.contains("86.37ビット") && error.contains("122.21ビット"));
    assert!(!error.contains("Base36:") && !error.contains("Base62:"));
}

#[test]
fn invalid_arguments_fail_without_standard_output() {
    let overflow = format!("{}0", usize::MAX);
    let largest_integer = usize::MAX.to_string();
    for args in [
        vec!["--length", "20", "--min-entropy", "128"],
        vec!["--unknown"],
        vec!["-x"],
        vec!["20"],
        vec!["--help=1"],
        vec!["--length", "0"],
        vec!["--length", "-1"],
        vec!["--length", "1.5"],
        vec!["--length", &overflow],
        vec!["--max-length", &largest_integer],
        vec!["--length", "20", "--max-length", "23"],
        vec!["--max-length", "0"],
        vec!["--group-size", "-1"],
        vec!["--min-entropy", "NaN"],
        vec!["--min-entropy", "inf"],
        vec!["--min-entropy", "-inf"],
        vec!["--min-entropy", "1e309"],
        vec!["--min-entropy", "1e308"],
        vec!["--min-entropy", "0"],
        vec!["--min-entropy", "-1"],
        vec!["--min-entropy", "abc"],
        vec!["--base", "64"],
        vec!["--max-length="],
        vec!["--length", "20", "21"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn missing_values_and_duplicate_options_are_input_errors() {
    for (name, value) in [
        ("--length", "20"),
        ("--min-entropy", "128"),
        ("--max-length", "32"),
        ("--group-size", "4"),
        ("--base", "62"),
    ] {
        for args in [vec![name], vec![name, "--help"]] {
            let output = run(&args);
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            assert!(output.stdout.is_empty(), "{args:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(name), "{args:?}: {error}");
            assert!(error.contains("a value is required"), "{args:?}: {error}");
        }
        let repeated = format!("{name}={value}");
        let output = run(&[name, value, &repeated]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(name), "{name}: {error}");
        assert!(
            error.contains("cannot be used multiple times"),
            "{name}: {error}"
        );
    }
}

#[test]
fn help_is_successful_and_describes_length_semantics() {
    for arg in ["--help", "-h"] {
        let output = run(&[arg]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let text = std::str::from_utf8(&output.stdout).unwrap();
        for option in [
            "--length",
            "--min-entropy",
            "--max-length",
            "--group-size",
            "--base",
        ] {
            assert!(text.contains(option));
        }
        assert!(text.contains("ハイフン込み"));
        assert!(text.contains("同時指定できません"));
    }
}

#[cfg(unix)]
#[test]
fn invalid_utf8_is_an_input_error_instead_of_a_panic() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let output = Command::new(env!("CARGO_BIN_EXE_generate_passwd"))
        .arg(OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    // clapは引数の位置によって診断が変わるため、拒否とパニックしないことを確認する。
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!error.is_empty());
    assert!(!error.contains("panicked"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn output_failure_returns_exit_code_one_instead_of_a_panic() {
    // Linuxの/dev/fullは書き込みに失敗するため、出力エラーを再現できる。
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_generate_passwd"))
        .arg("--help")
        .stdout(full)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("出力に失敗しました"));
    assert!(!error.contains("panicked"));
}
