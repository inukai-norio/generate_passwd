//! 乱数取得を差し替えられるパスワード生成ライブラリ。

/// パスワード生成に必要な乱数取得のインターフェース。
///
/// 本番用の実装は、成功時にバッファ全体を暗号学的に安全な乱数で埋めます。
/// テストでは固定データやエラーを返す実装へ差し替えられます。
pub trait RandomSource {
    fn fill(&mut self, buffer: &mut [u8]) -> Result<(), getrandom::Error>;
}

/// OSから暗号学的に安全な乱数を取得する、本番用の実装。
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn fill(&mut self, buffer: &mut [u8]) -> Result<(), getrandom::Error> {
        getrandom::fill(buffer)
    }
}

mod plan;

pub use plan::{ConstraintError, LengthPolicy, PasswordOptions, PasswordPlan, plan_password};

/// パスワードに使用する文字集合。
///
/// 生成には[`plan_password`]で検証した[`PasswordPlan`]を使います。
pub trait PasswordBase {
    /// 空でない、重複のないASCII文字集合（最大128文字）。
    const ALPHABET: &'static [u8];
}

fn generate_password_with<B: PasswordBase, R: RandomSource + ?Sized>(
    length: usize,
    group_size: usize,
    random: &mut R,
) -> Result<String, getrandom::Error> {
    let alphabet = B::ALPHABET;
    let limit = 256 - (256 % alphabet.len());
    let mut output = String::new();
    let mut buffer = [0u8; 64];
    let mut count = 0;

    while count < length {
        random.fill(&mut buffer)?;

        for &byte in &buffer {
            // 剰余による文字の選択確率の偏りを避けるため、端数の領域を棄却する。
            if usize::from(byte) >= limit {
                continue;
            }
            if group_size != 0 && count != 0 && count % group_size == 0 {
                output.push('-');
            }
            let index = usize::from(byte) % alphabet.len();
            output.push(alphabet[index] as char);
            count += 1;
            if count == length {
                break;
            }
        }
    }
    Ok(output)
}

// Base型を定義するマクロ
macro_rules! define_base {
    ($name:ident, $alphabet:expr) => {
        pub struct $name;

        impl PasswordBase for $name {
            const ALPHABET: &'static [u8] = $alphabet;
        }
    };
}

define_base!(Base10, b"0123456789");
define_base!(Base26, b"abcdefghijklmnopqrstuvwxyz");
define_base!(Base36, b"0123456789abcdefghijklmnopqrstuvwxyz");
define_base!(
    Base62,
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
);

/// 各文字が独立かつ一様に選ばれる場合のエントロピー。固定の区切り文字は含めません。
pub fn entropy_bits<B: PasswordBase>(length: usize) -> f64 {
    length as f64 * (B::ALPHABET.len() as f64).log2()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// 指定したバッファまたはエラーを順番に返すテスト用実装。
    struct ScriptedRandom {
        batches: VecDeque<Result<[u8; 64], getrandom::Error>>,
        calls: usize,
    }

    impl ScriptedRandom {
        fn new(batches: impl IntoIterator<Item = Result<[u8; 64], getrandom::Error>>) -> Self {
            Self {
                batches: batches.into_iter().collect(),
                calls: 0,
            }
        }
    }

    impl RandomSource for ScriptedRandom {
        fn fill(&mut self, buffer: &mut [u8]) -> Result<(), getrandom::Error> {
            self.calls += 1;
            let batch = self.batches.pop_front().expect("想定外の乱数取得")?;
            buffer.copy_from_slice(&batch);
            Ok(())
        }
    }

    fn generate<B: PasswordBase>(
        length: usize,
        group_size: usize,
        random: &mut impl RandomSource,
    ) -> Result<String, getrandom::Error> {
        let plan = plan_password::<B>(&PasswordOptions {
            length_policy: LengthPolicy::Exact(length),
            group_size,
            ..PasswordOptions::default()
        })
        .unwrap();
        let password = plan.generate_with(random)?;
        assert_eq!(password.len(), plan.output_length());
        Ok(password)
    }

    #[test]
    fn maps_bytes_to_each_alphabet_in_order() {
        fn check<B: PasswordBase>(expected: &str) {
            let batch = std::array::from_fn(|index| index as u8);
            let mut random = ScriptedRandom::new([Ok(batch)]);
            let password = generate::<B>(expected.len(), 0, &mut random).unwrap();
            assert_eq!(password, expected);
        }
        check::<Base10>("0123456789");
        check::<Base26>("abcdefghijklmnopqrstuvwxyz");
        check::<Base36>("0123456789abcdefghijklmnopqrstuvwxyz");
        check::<Base62>("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");
    }

    #[test]
    fn invalid_conditions_do_not_request_random_bytes() {
        let mut random = ScriptedRandom::new([]);
        for options in [
            PasswordOptions {
                length_policy: LengthPolicy::Exact(0),
                ..PasswordOptions::default()
            },
            PasswordOptions {
                max_output_length: Some(23),
                ..PasswordOptions::default()
            },
            PasswordOptions {
                max_output_length: Some(0),
                ..PasswordOptions::default()
            },
            PasswordOptions {
                length_policy: LengthPolicy::MinEntropy(128.0),
                max_output_length: Some(32),
                ..PasswordOptions::default()
            },
            PasswordOptions {
                length_policy: LengthPolicy::MinEntropy(f64::NAN),
                ..PasswordOptions::default()
            },
            PasswordOptions {
                length_policy: LengthPolicy::MinEntropy(f64::MAX),
                ..PasswordOptions::default()
            },
        ] {
            let result =
                plan_password::<Base10>(&options).map(|plan| plan.generate_with(&mut random));
            assert!(result.is_err());
        }
        assert_eq!(random.calls, 0);
    }

    #[test]
    fn generates_with_the_length_resolved_from_minimum_entropy() {
        let options = PasswordOptions {
            length_policy: LengthPolicy::MinEntropy(128.0),
            max_output_length: Some(32),
            ..PasswordOptions::default()
        };
        let plan = plan_password::<Base62>(&options).unwrap();
        let mut random = ScriptedRandom::new([Ok([0; 64])]);
        let password = plan.generate_with(&mut random).unwrap();
        assert_eq!(password, "0000-0000-0000-0000-0000-00");
        assert_eq!(password.len(), plan.output_length());
        assert_eq!(password.bytes().filter(|&byte| byte != b'-').count(), 22);
        assert!(plan.entropy_bits() >= 128.0);
        assert_eq!(random.calls, 1);
    }

    #[test]
    fn groups_without_leading_or_trailing_separator() {
        for (length, group_size, expected) in [
            (1, 4, "0"),
            (5, 0, "00000"),
            (3, 1, "0-0-0"),
            (8, 4, "0000-0000"),
            (9, 4, "0000-0000-0"),
            (3, 4, "000"),
        ] {
            let mut random = ScriptedRandom::new([Ok([0; 64])]);
            assert_eq!(
                generate::<Base10>(length, group_size, &mut random).unwrap(),
                expected,
                "length={length}, group_size={group_size}"
            );
        }
    }

    #[test]
    fn refills_buffer_only_when_needed() {
        for (length, expected_calls) in [(64, 1), (65, 2)] {
            let mut random = ScriptedRandom::new([Ok([0; 64]), Ok([1; 64])]);
            let password = generate::<Base10>(length, 0, &mut random).unwrap();
            let expected = if length == 64 {
                "0".repeat(64)
            } else {
                format!("{}1", "0".repeat(64))
            };
            assert_eq!(password, expected);
            assert_eq!(random.calls, expected_calls);
        }
    }

    #[test]
    fn rejects_bytes_at_or_above_each_limit() {
        fn check<B: PasswordBase>(limit: u8, expected: &str) {
            let mut batch = [0; 64];
            batch[..4].copy_from_slice(&[limit - 1, limit, 255, 0]);
            let mut random = ScriptedRandom::new([Ok(batch)]);
            // 棄却したバイトは文字数にもグループ位置にも数えない。
            assert_eq!(generate::<B>(2, 1, &mut random).unwrap(), expected);
        }
        check::<Base10>(250, "9-0");
        check::<Base26>(234, "z-a");
        check::<Base36>(252, "z-0");
        check::<Base62>(248, "z-0");
    }

    #[test]
    fn retries_after_entire_buffer_is_rejected() {
        let mut random = ScriptedRandom::new([Ok([255; 64]), Ok([0; 64])]);
        assert_eq!(generate::<Base62>(1, 0, &mut random).unwrap(), "0");
        assert_eq!(random.calls, 2);
    }

    #[test]
    fn propagates_initial_random_error() {
        let mut random = ScriptedRandom::new([Err(getrandom::Error::UNSUPPORTED)]);
        assert_eq!(
            generate::<Base10>(1, 0, &mut random),
            Err(getrandom::Error::UNSUPPORTED)
        );
        assert_eq!(random.calls, 1);
    }

    #[test]
    fn propagates_random_error_after_partial_generation() {
        let mut random = ScriptedRandom::new([Ok([0; 64]), Err(getrandom::Error::UNEXPECTED)]);
        assert_eq!(
            generate::<Base10>(65, 0, &mut random),
            Err(getrandom::Error::UNEXPECTED)
        );
        assert_eq!(random.calls, 2);
    }

    #[test]
    fn calculates_entropy_for_each_base() {
        assert!((entropy_bits::<Base10>(20) - 66.43856189774725).abs() < 1e-10);
        assert!((entropy_bits::<Base26>(20) - 94.00879436282184).abs() < 1e-10);
        assert!((entropy_bits::<Base36>(20) - 103.39850002884624).abs() < 1e-10);
        assert!((entropy_bits::<Base62>(20) - 119.0839262077375).abs() < 1e-10);
        assert_eq!(entropy_bits::<Base10>(0), 0.0);
    }
}
