use generate_passwd::{Base10, Base26, Base36, Base62, PasswordBase, entropy_bits};

/// 生成済みパスワードの表示文字列を作ります。lengthは区切り文字を除く長さです。
pub fn format_password_report<B: PasswordBase>(
    label: &str,
    password: &str,
    length: usize,
) -> String {
    let entropy_bits = entropy_bits::<B>(length);
    let output_length = password.chars().count();
    format!(
        "{label}: {password}\n\
         エントロピー: {entropy_bits:.2} ビット\n\
         出力文字数: {output_length}（区切り文字込み、ランダム部分: {length} 文字）\n\n"
    )
}

fn print_password<B: PasswordBase>(
    label: &str,
    length: usize,
    group_size: usize,
) -> Result<(), getrandom::Error> {
    let password = B::generate_password(length, group_size)?;
    print!("{}", format_password_report::<B>(label, &password, length));

    Ok(())
}

fn main() -> Result<(), getrandom::Error> {
    print_password::<Base10>("Base10", 20, 4)?;
    print_password::<Base26>("Base26", 20, 4)?;
    print_password::<Base36>("Base36", 20, 4)?;
    print_password::<Base62>("Base62", 20, 4)?;

    Ok(())
}
