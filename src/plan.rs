use std::{error::Error, fmt, marker::PhantomData};

use crate::{OsRandom, PasswordBase, RandomSource, entropy_bits, generate_password_with};

/// ランダム部分の長さを決める方法。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthPolicy {
    /// ハイフンを除いて、正確に指定した文字数にします。0文字は指定できません。
    Exact(usize),
}

/// パスワードの生成条件。
///
/// 既定値はランダム部分20文字、区切り幅4、出力上限なしです。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PasswordOptions {
    /// ランダム部分の文字数を決める方法。
    pub length_policy: LengthPolicy,
    /// ハイフンを含む出力全体の上限。`None`なら上限を指定しません。
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
    /// 文字集合が[`PasswordBase::ALPHABET`]の契約を満たしていません。
    InvalidAlphabet,
    /// 出力長の計算がオーバーフローするか、`String`で扱える長さを超えます。
    OutputLengthOverflow,
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
            Self::InvalidAlphabet => write!(
                f,
                "文字集合は重複のない1〜128文字のASCIIである必要があります"
            ),
            Self::OutputLengthOverflow => {
                write!(f, "出力文字数が文字列として扱える範囲を超えています")
            }
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
/// 0文字、出力上限超過、文字集合の契約違反、出力長の範囲外を拒否します。
/// `String`で扱える長さでも、実際に確保できるメモリ量は実行環境に依存します。
pub fn plan_password<B: PasswordBase>(
    options: &PasswordOptions,
) -> Result<PasswordPlan<B>, ConstraintError> {
    validate_alphabet(B::ALPHABET)?;
    if options.max_output_length == Some(0) {
        return Err(ConstraintError::ZeroMaxOutputLength);
    }
    let LengthPolicy::Exact(random_length) = options.length_policy;
    if random_length == 0 {
        return Err(ConstraintError::ZeroLength);
    }
    let output_length = output_length(random_length, options.group_size)
        .filter(|&length| length <= isize::MAX as usize)
        .ok_or(ConstraintError::OutputLengthOverflow)?;
    if let Some(max_output_length) = options.max_output_length
        && output_length > max_output_length
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Base10, Base62};

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
