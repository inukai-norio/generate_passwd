use generate_passwd::{Base10, Base26, Base36, Base62, PasswordBase, format_password_report};

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
