trait PasswordBase {
    const ALPHABET: &'static [u8];

    fn generate_password(length: usize, group_size: usize) -> Result<String, getrandom::Error> {
        let alphabet = Self::ALPHABET;
        let limit = 256 - (256 % alphabet.len());

        let mut output = String::new();
        let mut buffer = [0u8; 64];
        let mut count = 0;

        while count < length {
            getrandom::fill(&mut buffer)?;

            for &byte in &buffer {
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
}

// Base型を定義するマクロ
macro_rules! define_base {
    ($name:ident, $alphabet:expr) => {
        struct $name;

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

fn print_password<B: PasswordBase>(
    label: &str,
    length: usize,
    group_size: usize,
) -> Result<(), getrandom::Error> {
    let password = B::generate_password(length, group_size)?;
    // 各文字は独立かつ一様に選ばれる。固定の区切り文字はエントロピーに含めない。
    let entropy_bits = length as f64 * (B::ALPHABET.len() as f64).log2();

    println!("{label}: {password}");
    println!("エントロピー: {entropy_bits:.2} ビット");
    println!(
        "出力文字数: {}（区切り文字込み、ランダム部分: {length} 文字）\n",
        password.chars().count()
    );

    Ok(())
}

fn main() -> Result<(), getrandom::Error> {
    print_password::<Base10>("Base10", 20, 4)?;
    print_password::<Base26>("Base26", 20, 4)?;
    print_password::<Base36>("Base36", 20, 4)?;
    print_password::<Base62>("Base62", 20, 4)?;

    Ok(())
}
