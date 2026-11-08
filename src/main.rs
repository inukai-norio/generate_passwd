use generate_passwd::{
    Base10, Base26, Base36, Base62, PasswordBase, PasswordOptions, PasswordPlan, plan_password,
};

/// 生成済みパスワードと、その生成計画から表示文字列を作ります。
pub fn format_password_report<B: PasswordBase>(
    label: &str,
    password: &str,
    plan: &PasswordPlan<B>,
) -> String {
    let entropy_bits = plan.entropy_bits();
    let output_length = password.chars().count();
    let length = plan.random_length();
    format!(
        "{label}: {password}\n\
         エントロピー: {entropy_bits:.2} ビット\n\
         出力文字数: {output_length}（区切り文字込み、ランダム部分: {length} 文字）\n\n"
    )
}

fn print_password<B: PasswordBase>(
    label: &str,
    options: &PasswordOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let plan = plan_password::<B>(options)?;
    let password = plan.generate()?;
    print!("{}", format_password_report(label, &password, &plan));

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = PasswordOptions::default();
    print_password::<Base10>("Base10", &options)?;
    print_password::<Base26>("Base26", &options)?;
    print_password::<Base36>("Base36", &options)?;
    print_password::<Base62>("Base62", &options)?;

    Ok(())
}
