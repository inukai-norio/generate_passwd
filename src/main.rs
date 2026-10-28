
trait PasswordBase {
    const ALPHABET: &'static [u8];

    fn generate_password(
        length: usize,
        group_size: usize,
    ) -> Result<String, getrandom::Error> {
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

                if group_size != 0
                    && count != 0
                    && count % group_size == 0
                {
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
define_base!(Base62, b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");

fn main() -> Result<(), getrandom::Error> {
    println!("{}", Base10::generate_password(20, 4)?);
    println!("{}", Base26::generate_password(20, 4)?);
    println!("{}", Base36::generate_password(20, 4)?);
    println!("{}", Base62::generate_password(20, 4)?);

    Ok(())
}
