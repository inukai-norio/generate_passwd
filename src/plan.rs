use std::{error::Error, fmt, marker::PhantomData};

use crate::{OsRandom, PasswordBase, RandomSource, entropy_bits, generate_password_with};

/// ランダム部分の長さを決める方法。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthPolicy {
    /// ハイフンを除いて、正確に指定した文字数にします。0文字は指定できません。
    Exact(usize),
    /// 指定ビット数以上になる最少のランダム文字数を使います。
    ///
    /// ビット数には有限の正の値を指定します。小数も使用できます。
    ///
    /// ```
    /// use generate_passwd::{Base62, LengthPolicy, PasswordOptions, plan_password};
    /// let options = PasswordOptions {
    ///     length_policy: LengthPolicy::MinEntropy(128.0),
    ///     max_output_length: Some(32),
    ///     ..PasswordOptions::default()
    /// };
    /// let plan = plan_password::<Base62>(&options)?;
    /// assert_eq!(plan.random_length(), 22);
    /// assert_eq!(plan.output_length(), 27);
    /// assert!(plan.entropy_bits() >= 128.0);
    /// # Ok::<(), generate_passwd::ConstraintError>(())
    /// ```
    MinEntropy(f64),
    /// ハイフン込みの出力上限に収まる、最多のランダム文字を使います。
    ///
    /// [`PasswordOptions::max_output_length`]に正の上限を指定する必要があります。
    ///
    /// ```
    /// use generate_passwd::{Base62, LengthPolicy, PasswordOptions, plan_password};
    /// let options = PasswordOptions {
    ///     length_policy: LengthPolicy::FitToMaxOutput,
    ///     max_output_length: Some(5),
    ///     ..PasswordOptions::default()
    /// };
    /// let plan = plan_password::<Base62>(&options)?;
    /// assert_eq!(plan.random_length(), 4);
    /// assert_eq!(plan.output_length(), 4); // 末尾ハイフンで5文字にはしない
    /// # Ok::<(), generate_passwd::ConstraintError>(())
    /// ```
    FitToMaxOutput,
}

/// パスワードの生成条件。
///
/// 既定値はランダム部分20文字、区切り幅4、出力上限なしです。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PasswordOptions {
    /// ランダム部分の文字数を決める方法。
    pub length_policy: LengthPolicy,
    /// ハイフンを含む出力全体の上限。`None`なら上限を指定しません。
    /// [`LengthPolicy::FitToMaxOutput`]では`Some`による指定が必須です。
    pub max_output_length: Option<usize>,
    /// 0なら区切りなし。それ以外は指定文字数ごとに`-`を挿入します。
    pub group_size: usize,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length_policy: LengthPolicy::Exact(20),
            max_output_length: None,
            group_size: 4,
        }
    }
}

/// 乱数を取得する前に検出された、生成条件のエラー。
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintError {
    /// ランダム部分に0文字を指定しています。
    ZeroLength,
    /// 出力上限に0文字を指定しています。
    ZeroMaxOutputLength,
    /// 出力上限に合わせるモードで、上限が指定されていません。
    MissingMaxOutputLength,
    /// 文字集合が[`PasswordBase::ALPHABET`]の契約を満たしていません。
    InvalidAlphabet,
    /// 出力長の計算がオーバーフローするか、`String`で扱える長さを超えます。
    OutputLengthOverflow,
    /// 最小エントロピーが非有限値、または0以下です。
    InvalidMinEntropy,
    /// 文字集合と扱えるランダム文字数の範囲では、要求ビット数を満たせません。
    EntropyUnreachable { min_entropy_bits: f64 },
    /// 要求ビット数を満たす最短の出力でも、出力上限を超えます。
    EntropyLimitExceeded {
        min_entropy_bits: f64,
        random_length: usize,
        output_length: usize,
        max_output_length: usize,
        max_entropy_bits: f64,
    },
    /// 区切り文字を加えた長さが出力上限を超えます。
    OutputLimitExceeded {
        random_length: usize,
        output_length: usize,
        max_output_length: usize,
    },
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLength => write!(f, "ランダム部分の文字数には1以上を指定してください"),
            Self::ZeroMaxOutputLength => write!(f, "最大出力文字数には1以上を指定してください"),
            Self::MissingMaxOutputLength => {
                write!(f, "出力上限に合わせるモードでは最大出力文字数が必要です")
            }
            Self::InvalidAlphabet => write!(
                f,
                "文字集合は重複のない1〜128文字のASCIIである必要があります"
            ),
            Self::OutputLengthOverflow => {
                write!(f, "出力文字数が文字列として扱える範囲を超えています")
            }
            Self::InvalidMinEntropy => {
                write!(f, "最小エントロピーには有限の正の値を指定してください")
            }
            Self::EntropyUnreachable { min_entropy_bits } => write!(
                f,
                "この文字集合と扱えるランダム文字数の範囲では{min_entropy_bits}ビット以上を実現できません"
            ),
            Self::EntropyLimitExceeded {
                min_entropy_bits,
                random_length,
                output_length,
                max_output_length,
                max_entropy_bits,
            } => write!(
                f,
                "{min_entropy_bits}ビット以上にはランダム部分{random_length}文字、区切り込み{output_length}文字が必要です（出力上限: {max_output_length}文字、上限内で達成できるエントロピー: 約{max_entropy_bits:.2}ビット）"
            ),
            Self::OutputLimitExceeded {
                random_length,
                output_length,
                max_output_length,
            } => write!(
                f,
                "ランダム部分{random_length}文字には区切り込み{output_length}文字が必要です（出力上限: {max_output_length}文字）"
            ),
        }
    }
}

impl Error for ConstraintError {}

/// 検証済みの生成計画。長さ・区切り幅・文字集合は生成時に変更できません。
///
/// 旧APIの`Base62::generate_password(20, 4)`は、次の呼び出しに移行します。
///
/// ```
/// use generate_passwd::{Base62, LengthPolicy, PasswordOptions, plan_password};
///
/// let options = PasswordOptions {
///     length_policy: LengthPolicy::Exact(20),
///     max_output_length: Some(24),
///     group_size: 4,
/// };
/// let plan = plan_password::<Base62>(&options)?;
/// let password = plan.generate()?;
/// assert_eq!(password.len(), plan.output_length());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// 異なる文字集合の計画として扱うことはできません。
///
/// ```compile_fail
/// use generate_passwd::{Base10, Base62, PasswordOptions, PasswordPlan, plan_password};
/// let plan = plan_password::<Base62>(&PasswordOptions::default()).unwrap();
/// let wrong_base: PasswordPlan<Base10> = plan;
/// ```
pub struct PasswordPlan<B: PasswordBase> {
    random_length: usize,
    output_length: usize,
    group_size: usize,
    // 文字集合の型を計画に結び付けるための印。
    base: PhantomData<B>,
}

impl<B: PasswordBase> PasswordPlan<B> {
    /// ハイフンを除く、ランダム部分の文字数。
    pub fn random_length(&self) -> usize {
        self.random_length
    }

    /// ハイフンを含む出力全体の文字数。
    pub fn output_length(&self) -> usize {
        self.output_length
    }

    /// 区切り幅。0は区切りなし。
    pub fn group_size(&self) -> usize {
        self.group_size
    }

    /// 独立・一様な文字選択によるエントロピー。固定のハイフンは含みません。
    pub fn entropy_bits(&self) -> f64 {
        entropy_bits::<B>(self.random_length)
    }

    /// OSの暗号学的に安全な乱数で生成します。
    pub fn generate(&self) -> Result<String, getrandom::Error> {
        self.generate_with(&mut OsRandom)
    }

    /// 指定した乱数源で生成します。生成途中の乱数エラーもそのまま返します。
    ///
    /// 旧APIの`generate_password_with`は、このメソッドに移行します。
    ///
    /// ```
    /// use generate_passwd::{Base10, LengthPolicy, PasswordOptions, RandomSource, plan_password};
    /// struct FixedRandom;
    /// impl RandomSource for FixedRandom {
    ///     fn fill(&mut self, buffer: &mut [u8]) -> Result<(), getrandom::Error> {
    ///         buffer.fill(0);
    ///         Ok(())
    ///     }
    /// }
    /// let options = PasswordOptions {
    ///     length_policy: LengthPolicy::Exact(8),
    ///     ..PasswordOptions::default()
    /// };
    /// let plan = plan_password::<Base10>(&options)?;
    /// assert_eq!(plan.generate_with(&mut FixedRandom)?, "0000-0000");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn generate_with<R: RandomSource + ?Sized>(
        &self,
        random: &mut R,
    ) -> Result<String, getrandom::Error> {
        generate_password_with::<B, R>(self.random_length, self.group_size, random)
    }
}

/// 条件を検証し、乱数を取得せずに生成計画を作ります。
///
/// 0文字、不正な最小エントロピー、出力上限超過、文字集合の契約違反、
/// 出力長の範囲外を拒否します。エントロピーの判定は[`entropy_bits`]と同じ
/// `f64`精度で行い、表示用に丸めた値や許容誤差は使いません。
/// `String`で扱える長さでも、実際に確保できるメモリ量は実行環境に依存します。
pub fn plan_password<B: PasswordBase>(
    options: &PasswordOptions,
) -> Result<PasswordPlan<B>, ConstraintError> {
    validate_alphabet(B::ALPHABET)?;
    if options.max_output_length == Some(0) {
        return Err(ConstraintError::ZeroMaxOutputLength);
    }
    let random_length = match options.length_policy {
        LengthPolicy::Exact(0) => return Err(ConstraintError::ZeroLength),
        LengthPolicy::Exact(length) => length,
        LengthPolicy::MinEntropy(bits) => minimum_random_length::<B>(bits)?,
        LengthPolicy::FitToMaxOutput => max_random_length(
            options
                .max_output_length
                .ok_or(ConstraintError::MissingMaxOutputLength)?,
            options.group_size,
        ),
    };
    let output_length = output_length(random_length, options.group_size)
        .filter(|&length| length <= isize::MAX as usize)
        .ok_or(ConstraintError::OutputLengthOverflow)?;
    if let Some(max_output_length) = options.max_output_length
        && output_length > max_output_length
    {
        if let LengthPolicy::MinEntropy(min_entropy_bits) = options.length_policy {
            return Err(ConstraintError::EntropyLimitExceeded {
                min_entropy_bits,
                random_length,
                output_length,
                max_output_length,
                max_entropy_bits: entropy_bits::<B>(max_random_length(
                    max_output_length,
                    options.group_size,
                )),
            });
        }
        return Err(ConstraintError::OutputLimitExceeded {
            random_length,
            output_length,
            max_output_length,
        });
    }
    Ok(PasswordPlan {
        random_length,
        output_length,
        group_size: options.group_size,
        base: PhantomData,
    })
}

fn validate_alphabet(alphabet: &[u8]) -> Result<(), ConstraintError> {
    if alphabet.is_empty() || alphabet.len() > 128 {
        return Err(ConstraintError::InvalidAlphabet);
    }
    let mut seen = [false; 128];
    for &byte in alphabet {
        if !byte.is_ascii() || seen[usize::from(byte)] {
            return Err(ConstraintError::InvalidAlphabet);
        }
        seen[usize::from(byte)] = true;
    }
    Ok(())
}

fn output_length(random_length: usize, group_size: usize) -> Option<usize> {
    let separators = if random_length == 0 || group_size == 0 {
        0
    } else {
        (random_length - 1) / group_size
    };
    random_length.checked_add(separators)
}

fn minimum_random_length<B: PasswordBase>(bits: f64) -> Result<usize, ConstraintError> {
    if !bits.is_finite() || bits <= 0.0 {
        return Err(ConstraintError::InvalidMinEntropy);
    }
    let mut low = 1;
    let mut high = isize::MAX as usize;
    if entropy_bits::<B>(high) < bits {
        // 1文字だけの文字集合も、エントロピーが常に0なのでここで拒否する。
        return Err(ConstraintError::EntropyUnreachable {
            min_entropy_bits: bits,
        });
    }
    // 要求を満たす最初の文字数を二分探索する。
    // 浮動小数から整数への変換や、その後の丸め誤差の補正は不要。
    while low < high {
        let middle = low + (high - low) / 2;
        if entropy_bits::<B>(middle) >= bits {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(low)
}

// 出力上限に収まる最多のランダム文字数。上限指定モードとエラー説明で共用する。
fn max_random_length(max_output_length: usize, group_size: usize) -> usize {
    if group_size == 0 || group_size >= max_output_length {
        return max_output_length;
    }
    // (g+1)文字ごとのハイフン用の枠を、全体の上限から除く。
    // 上の分岐によりg < max_output_lengthなので、g+1はオーバーフローしない。
    max_output_length - max_output_length / (group_size + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Base10, Base26, Base36, Base62};

    #[test]
    fn counts_separators_at_group_boundaries() {
        for (length, group, expected) in [
            (0, 0, 0),
            (0, 4, 0),
            (1, 4, 1),
            (3, 4, 3),
            (4, 4, 4),
            (5, 4, 6),
            (8, 4, 9),
            (9, 4, 11),
            (5, 0, 5),
            (3, 1, 5),
            (3, usize::MAX, 3),
        ] {
            assert_eq!(output_length(length, group), Some(expected));
        }
        assert_eq!(output_length(usize::MAX, 1), None);
        assert_eq!(output_length(usize::MAX, 0), Some(usize::MAX));
    }

    #[test]
    fn accepts_exact_output_limit_and_rejects_one_less() {
        let mut options = PasswordOptions {
            max_output_length: Some(24),
            ..PasswordOptions::default()
        };
        let plan = plan_password::<Base62>(&options).unwrap();
        assert_eq!(plan.random_length(), 20);
        assert_eq!(plan.output_length(), 24);
        assert_eq!(plan.group_size(), 4);
        assert_eq!(plan.entropy_bits(), entropy_bits::<Base62>(20));
        options.max_output_length = Some(23);
        assert!(matches!(
            plan_password::<Base62>(&options),
            Err(ConstraintError::OutputLimitExceeded {
                random_length: 20,
                output_length: 24,
                max_output_length: 23,
            })
        ));
    }

    #[test]
    fn rejects_zero_and_unrepresentable_output_lengths() {
        for (length, max, group, expected) in [
            (0, None, 4, ConstraintError::ZeroLength),
            (20, Some(0), 4, ConstraintError::ZeroMaxOutputLength),
            (usize::MAX, None, 1, ConstraintError::OutputLengthOverflow),
            (usize::MAX, None, 0, ConstraintError::OutputLengthOverflow),
        ] {
            let options = PasswordOptions {
                length_policy: LengthPolicy::Exact(length),
                max_output_length: max,
                group_size: group,
            };
            assert!(matches!(plan_password::<Base10>(&options), Err(error) if error == expected));
        }
    }

    #[test]
    fn planning_does_not_allocate_the_requested_output() {
        let options = PasswordOptions {
            length_policy: LengthPolicy::Exact(isize::MAX as usize),
            group_size: 0,
            ..PasswordOptions::default()
        };
        let plan = plan_password::<Base10>(&options).unwrap();
        assert_eq!(plan.output_length(), isize::MAX as usize);
    }

    #[test]
    fn resolves_minimum_entropy_for_each_base() {
        fn check<B: PasswordBase>(expected_random: usize, expected_output: usize) {
            let options = PasswordOptions {
                length_policy: LengthPolicy::MinEntropy(128.0),
                ..PasswordOptions::default()
            };
            let plan = plan_password::<B>(&options).unwrap();
            assert_eq!(plan.random_length(), expected_random);
            assert_eq!(plan.output_length(), expected_output);
            assert!(plan.entropy_bits() >= 128.0);
            assert!(entropy_bits::<B>(plan.random_length() - 1) < 128.0);
        }
        check::<Base10>(39, 48);
        check::<Base26>(28, 34);
        check::<Base36>(25, 31);
        check::<Base62>(22, 27);
    }

    #[test]
    fn compares_unrounded_entropy_at_length_boundaries() {
        fn check<B: PasswordBase>() {
            for length in [1, 4, 20, 21, 22, 64, 128] {
                let boundary = entropy_bits::<B>(length);
                // 境界の直前・境界そのもの・直後で、最少文字数の条件を確認する。
                for bits in [boundary.next_down(), boundary, boundary.next_up()] {
                    let options = PasswordOptions {
                        length_policy: LengthPolicy::MinEntropy(bits),
                        ..PasswordOptions::default()
                    };
                    let plan = plan_password::<B>(&options).unwrap();
                    assert!(plan.entropy_bits() >= bits);
                    assert!(entropy_bits::<B>(plan.random_length() - 1) < bits);
                }
                assert_eq!(minimum_random_length::<B>(boundary).unwrap(), length);
                assert_eq!(
                    minimum_random_length::<B>(boundary.next_up()).unwrap(),
                    length + 1
                );
            }
        }
        check::<Base10>();
        check::<Base26>();
        check::<Base36>();
        check::<Base62>();
        let boundary = entropy_bits::<Base62>(22);
        assert_eq!(
            format!("{boundary:.2}"),
            format!("{:.2}", boundary.next_up())
        );
    }

    #[test]
    fn accepts_fractional_and_very_small_positive_entropy() {
        for (bits, expected_length) in [(128.5, 22), (f64::from_bits(1), 1)] {
            let options = PasswordOptions {
                length_policy: LengthPolicy::MinEntropy(bits),
                ..PasswordOptions::default()
            };
            let plan = plan_password::<Base62>(&options).unwrap();
            assert_eq!(plan.random_length(), expected_length);
            assert!(plan.entropy_bits() >= bits);
            assert!(entropy_bits::<Base62>(plan.random_length() - 1) < bits);
        }
    }

    #[test]
    fn rejects_non_positive_or_non_finite_entropy() {
        for bits in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let options = PasswordOptions {
                length_policy: LengthPolicy::MinEntropy(bits),
                ..PasswordOptions::default()
            };
            assert!(matches!(
                plan_password::<Base62>(&options),
                Err(ConstraintError::InvalidMinEntropy)
            ));
        }
    }

    #[test]
    fn rejects_entropy_outside_the_representable_length_range() {
        let options = PasswordOptions {
            length_policy: LengthPolicy::MinEntropy(f64::MAX),
            ..PasswordOptions::default()
        };
        assert!(matches!(
            plan_password::<Base62>(&options),
            Err(ConstraintError::EntropyUnreachable { min_entropy_bits })
                if min_entropy_bits == f64::MAX
        ));
        let options = PasswordOptions {
            length_policy: LengthPolicy::MinEntropy(entropy_bits::<Base62>(isize::MAX as usize)),
            group_size: 1,
            ..PasswordOptions::default()
        };
        assert!(matches!(
            plan_password::<Base62>(&options),
            Err(ConstraintError::OutputLengthOverflow)
        ));
    }

    #[test]
    fn a_single_character_alphabet_cannot_provide_positive_entropy() {
        struct Single;
        impl PasswordBase for Single {
            const ALPHABET: &'static [u8] = b"x";
        }
        assert!(plan_password::<Single>(&PasswordOptions::default()).is_ok());
        let options = PasswordOptions {
            length_policy: LengthPolicy::MinEntropy(1.0),
            ..PasswordOptions::default()
        };
        assert!(matches!(
            plan_password::<Single>(&options),
            Err(ConstraintError::EntropyUnreachable {
                min_entropy_bits: 1.0
            })
        ));
    }

    #[test]
    fn reports_required_length_and_achievable_entropy_when_limit_is_too_short() {
        let options = PasswordOptions {
            length_policy: LengthPolicy::MinEntropy(128.0),
            max_output_length: Some(32),
            ..PasswordOptions::default()
        };
        assert!(plan_password::<Base62>(&options).is_ok());
        let Err(error) = plan_password::<Base10>(&options) else {
            panic!("Base10の128ビットは出力上限32文字に収まらない");
        };
        assert!(matches!(
            error,
            ConstraintError::EntropyLimitExceeded {
                min_entropy_bits: 128.0,
                random_length: 39,
                output_length: 48,
                max_output_length: 32,
                max_entropy_bits,
            } if max_entropy_bits == entropy_bits::<Base10>(26)
        ));
        let message = error.to_string();
        assert!(message.contains("39文字") && message.contains("48文字"));
        assert!(message.contains("32文字") && message.contains("86.37ビット"));
    }

    #[test]
    fn entropy_limit_includes_separators_without_counting_them_as_entropy() {
        let mut options = PasswordOptions {
            length_policy: LengthPolicy::MinEntropy(128.0),
            max_output_length: Some(27),
            ..PasswordOptions::default()
        };
        let plan = plan_password::<Base62>(&options).unwrap();
        assert_eq!(plan.output_length(), 27);
        options.max_output_length = Some(26);
        assert!(matches!(
            plan_password::<Base62>(&options),
            Err(ConstraintError::EntropyLimitExceeded {
                output_length: 27,
                ..
            })
        ));
        options.group_size = 0;
        options.max_output_length = Some(22);
        let ungrouped = plan_password::<Base62>(&options).unwrap();
        assert_eq!(ungrouped.output_length(), 22);
        assert_eq!(ungrouped.entropy_bits(), plan.entropy_bits());
    }

    #[test]
    fn maximum_random_length_agrees_with_enumerated_outputs() {
        for limit in 0..=32 {
            for group in 0..=8 {
                let expected = (0..=limit)
                    .filter(|&n| output_length(n, group).unwrap() <= limit)
                    .max()
                    .unwrap();
                assert_eq!(max_random_length(limit, group), expected);
            }
        }
        assert_eq!(max_random_length(usize::MAX, usize::MAX), usize::MAX);
        assert_eq!(max_random_length(usize::MAX, 0), usize::MAX);
        assert_eq!(max_random_length(usize::MAX, 1), usize::MAX / 2 + 1);
    }

    #[test]
    fn fits_the_most_random_characters_within_the_output_limit() {
        for (limit, group, expected_random, expected_output) in [
            (24, 4, 20, 24),
            (5, 4, 4, 4),
            (6, 4, 5, 6),
            (9, 4, 8, 9),
            (10, 4, 8, 9),
            (32, 0, 32, 32),
            (6, 1, 3, 5),
            (1, 4, 1, 1),
            (5, usize::MAX, 5, 5),
        ] {
            let options = PasswordOptions {
                length_policy: LengthPolicy::FitToMaxOutput,
                max_output_length: Some(limit),
                group_size: group,
            };
            let plan = plan_password::<Base62>(&options).unwrap();
            assert_eq!(plan.random_length(), expected_random);
            assert_eq!(plan.output_length(), expected_output);
            assert!(plan.output_length() <= limit);
            // 1文字増やすと上限を超えることで、最多の文字数であることを確認する。
            assert!(output_length(plan.random_length() + 1, group).unwrap() > limit);
        }
    }

    #[test]
    fn fitting_requires_a_positive_representable_output_limit() {
        for (limit, expected) in [
            (None, ConstraintError::MissingMaxOutputLength),
            (Some(0), ConstraintError::ZeroMaxOutputLength),
            (Some(usize::MAX), ConstraintError::OutputLengthOverflow),
        ] {
            let options = PasswordOptions {
                length_policy: LengthPolicy::FitToMaxOutput,
                max_output_length: limit,
                ..PasswordOptions::default()
            };
            assert!(matches!(plan_password::<Base62>(&options), Err(error) if error == expected));
        }
        // 境界値の計画だけを作り、巨大な文字列は生成しない。
        for group in [0, 1, 4, usize::MAX] {
            let limit = isize::MAX as usize;
            let options = PasswordOptions {
                length_policy: LengthPolicy::FitToMaxOutput,
                max_output_length: Some(limit),
                group_size: group,
            };
            let plan = plan_password::<Base62>(&options).unwrap();
            assert!(plan.output_length() <= limit);
            assert!(output_length(plan.random_length() + 1, group).unwrap() > limit);
        }
    }

    #[test]
    fn rejects_invalid_custom_alphabets() {
        for alphabet in [&b""[..], &b"00"[..], &b"\xff"[..], &[0; 129][..]] {
            assert_eq!(
                validate_alphabet(alphabet),
                Err(ConstraintError::InvalidAlphabet)
            );
        }
        struct Duplicate;
        impl PasswordBase for Duplicate {
            const ALPHABET: &'static [u8] = b"00";
        }
        assert!(matches!(
            plan_password::<Duplicate>(&PasswordOptions::default()),
            Err(ConstraintError::InvalidAlphabet)
        ));
    }
}
