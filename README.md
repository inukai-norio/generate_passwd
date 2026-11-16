<!-- SPDX-License-Identifier: ISC -->
<!-- Copyright (c) 2026 inukai-norio <inukay@proton.me> -->

# generate_passwd

OSの暗号学的に安全な乱数を使う、パスワード生成CLIとRustライブラリです。ランダム部分の固定文字数、最小エントロピー、ハイフン込みの最大出力文字数を指定できます。

## 配布版の導入

[GitHub Releases](https://github.com/inukai-norio/generate_passwd/releases) から、OSとCPUに合うZIPをダウンロードしてください。配布版のCLIを実行するためにRustやCargoをインストールする必要はありません。

| OS | CPU | ZIPファイル |
| --- | --- | --- |
| Linux | x86_64 / AMD64（64ビット） | `generate_passwd-x86_64-unknown-linux-musl.zip` |
| Linux | ARM64 / AArch64 | `generate_passwd-aarch64-unknown-linux-musl.zip` |
| Linux | x86（32ビット） | `generate_passwd-i686-unknown-linux-musl.zip` |
| macOS | Intel | `generate_passwd-x86_64-apple-darwin.zip` |
| macOS | Apple Silicon | `generate_passwd-aarch64-apple-darwin.zip` |
| Windows | x64（64ビット） | `generate_passwd-x86_64-pc-windows-msvc.zip` |
| Windows | x86（32ビット） | `generate_passwd-i686-pc-windows-msvc.zip` |

ZIPを展開し、展開先のディレクトリでターミナルを開きます。Linux・macOSでは、実行権限を付けて起動します。

```sh
chmod +x ./generate_passwd
./generate_passwd --help
```

Windowsでは、展開先のディレクトリでPowerShellを開いて起動します。

```powershell
.\generate_passwd.exe --help
.\generate_passwd.exe --base 62 --min-entropy 128 --max-length 32
```

## CLIの使い方

以下はLinux・macOSでの例です。WindowsのPowerShellでは、先頭の `./generate_passwd` を `.\generate_passwd.exe` に置き換えてください。

```sh
# 引数なし: 全Base、ランダム部分20文字、4文字ごとにハイフン
./generate_passwd

# Base62で128ビット以上、ハイフン込みで32文字以下
./generate_passwd --base 62 --min-entropy 128 --max-length 32

# 区切りなしなら、Base62の128ビット以上を22文字で生成できる
./generate_passwd --base 62 --min-entropy 128 --max-length 22 --group-size 0

# 最大24文字に収まる最多のランダム文字を使う
./generate_passwd --base 62 --max-length 24

# ランダム部分を正確に20文字とし、出力上限も検証する
./generate_passwd --base 62 --length 20 --max-length 24

# 上限5文字・区切り幅4では、末尾ハイフンを付けず4文字で出力する
./generate_passwd --base 62 --max-length 5

./generate_passwd --help
```

Base62・128ビット以上・区切り幅4の出力例です。パスワード部分は省略しています。

```text
Base62: （生成されたパスワード）
エントロピー: 130.99 ビット
出力文字数: 27（区切り文字込み、ランダム部分: 22 文字）
```

### オプション

| オプション | 意味 | 既定値 |
| --- | --- | --- |
| `--length N` | ランダム部分を正確にN文字にする | 長さ関係の指定がなければ20文字 |
| `--min-entropy BITS` | 指定ビット数以上になる最少のランダム文字数を使う。小数も指定可能 | 指定なし |
| `--max-length M` | ハイフン込みの出力全体をM文字以下にする | 上限指定なし |
| `--group-size G` | G文字ごとにハイフンを入れる。0なら区切りなし | 4 |
| `--base BASE` | `10` / `26` / `36` / `62` / `all` | `all` |
| `-h`, `--help` | 使い方を表示する | — |

`--base 62`と`--base=62`の両形式を使用できます。値を取るオプションは、それぞれ一度だけ指定できます。

文字数と最大文字数は正の整数、最小エントロピーは有限の正の数を指定します。0文字・上限0・負数・`NaN`・無限大は拒否します。区切り幅には0も指定できます。未知の引数、値の欠落、数値の範囲外もエラーになります。

### 長さ指定の組み合わせ

| 長さ関係の指定 | ランダム部分の長さの決め方 |
| --- | --- |
| 指定なし | 20文字 |
| `--length N` | 正確にN文字 |
| `--min-entropy BITS` | 要求ビット数を満たす最少文字数 |
| `--max-length M`だけ | 出力上限に収まる最多のランダム文字数 |
| `--length N --max-length M` | N文字で、ハイフン込みの出力がM文字以下なら成功 |
| `--min-entropy BITS --max-length M` | 要求を満たす最短の出力がM文字以下なら成功 |
| `--length N --min-entropy BITS` | 同時指定エラー |

`--base`や`--group-size`だけを変更した場合、ランダム部分の長さは20文字です。

最大文字数は、ちょうどその長さまで埋める指定ではありません。区切り幅4・上限5なら、ランダム部分4文字の出力になります。ランダム部分を5文字にするとハイフン込みで6文字となり、上限を超えるためです。先頭・末尾のハイフンの追加や、生成後の切り詰めは行いません。

### 文字集合とエントロピー

| Base | 使用する文字 | 128ビット以上に必要なランダム文字数 | 区切り幅4の出力文字数 |
| --- | --- | ---: | ---: |
| Base10 | 数字 | 39 | 48 |
| Base26 | 英小文字 | 28 | 34 |
| Base36 | 数字・英小文字 | 25 | 31 |
| Base62 | 数字・英大文字・英小文字 | 22 | 27 |

各文字が独立・一様に選ばれるとき、ランダム文字数をn、文字集合の大きさをAとして、エントロピーは`n × log2(A)`です。位置が固定されたハイフンはエントロピーに加算しません。

判定は`f64`精度で計算した丸め前の値を使います。小数2桁に丸めるのは表示時だけです。

### 条件が両立しない場合

次の指定は、Base10では区切り込み48文字が必要になるため失敗します。

```sh
# 終了コード2。パスワードは標準出力に出さない
./generate_passwd --base 10 --min-entropy 128 --max-length 32
```

エラーには必要なランダム文字数、ハイフン込みの出力文字数、指定上限と、その上限内で達成できるエントロピーが表示されます。

文字集合・区切り幅・上限を明示的に変更してください。条件を成立させるために、文字集合や区切り方を自動変更することはありません。

`--base all`では、すべてのBaseについて先に条件を検証します。一つでも不成立なら、そのBaseの理由をまとめて報告し、生成には進みません。たとえば128ビット以上・上限32文字・区切り幅4では、Base10とBase26が不成立です。

生成に進んだ場合も、全件の生成に成功してから表示します。途中で乱数取得に失敗した場合は、一部のパスワードだけを表示しません。標準出力への書き込み自体が途中で失敗した場合は、出力エラーとして扱います。

| 終了コード | 意味 |
| --- | --- |
| 0 | 生成成功、またはヘルプ表示 |
| 1 | 乱数取得エラー、または出力エラー |
| 2 | 引数または生成条件のエラー |

正常結果とヘルプは標準出力、エラーは標準エラー出力へ書き込みます。

## ソースからのビルド

RustのstableツールチェーンとCargoを用意し、このリポジトリを取得したディレクトリで実行します。CIでもstableを使用しています。

```sh
cargo build --locked --release
cargo run --locked --release -- --help
```

生成される実行ファイルは、Linux・macOSでは `target/release/generate_passwd`、Windowsでは `target\release\generate_passwd.exe` です。配布版と同じCLIオプションを使用できます。

## ライブラリAPI

この節は、Rustプロジェクトから本ライブラリを利用する場合の説明です。

生成条件を`PasswordOptions`で指定し、`plan_password`で検証済みの計画を作ります。計画作成では乱数を取得せず、`generate()`を呼んだ時点で生成します。

```rust
use generate_passwd::{Base62, LengthPolicy, PasswordOptions, plan_password};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = PasswordOptions {
        length_policy: LengthPolicy::MinEntropy(128.0),
        max_output_length: Some(32),
        group_size: 4,
    };
    let plan = plan_password::<Base62>(&options)?;
    assert_eq!(plan.random_length(), 22);
    assert_eq!(plan.output_length(), 27);
    assert!(plan.entropy_bits() >= 128.0);

    let password = plan.generate()?;
    assert_eq!(password.len(), plan.output_length());
    Ok(())
}
```

### 長さのモードと既定値

| `LengthPolicy` | 意味 |
| --- | --- |
| `Exact(n)` | ランダム部分を正確にn文字にする |
| `MinEntropy(bits)` | 必要ビット数を満たす最少のランダム文字数を求める |
| `FitToMaxOutput` | 出力上限内で最多のランダム文字数を使う。正の`max_output_length`が必須 |

`PasswordOptions::default()`は固定長20文字、区切り幅4、出力上限なしです。

ライブラリでは、`max_output_length`だけを書き換えてもモードは固定長のままです。上限に合わせて文字数を決めたい場合は、`length_policy`に`LengthPolicy::FitToMaxOutput`も指定してください。CLIで最大文字数だけを指定した場合は、CLI側でこのモードを選びます。

### 計画・生成・エラー

- `plan_password::<B>(&options)`は、条件エラーを`ConstraintError`として返します。
- `plan.generate()`はOS乱数を使い、乱数取得エラーを`getrandom::Error`として返します。
- `plan.generate_with(&mut random)`には、`RandomSource`を実装した乱数源を渡せます。戻るエラー型は`generate()`と同じです。

計画からは、`random_length()`、`output_length()`、`group_size()`、`entropy_bits()`で決定済みの値を参照できます。フィールドは非公開で、生成時に別の長さや文字集合へ差し替えることはできません。

独自の`PasswordBase`も使用できます。`ALPHABET`は重複のない1〜128文字のASCIIである必要があり、計画作成時に検証します。1文字だけの文字集合でも固定長生成はできますが、正のエントロピーは実現できません。

## 開発時の確認

Rustのstableツールチェーンを使用します。初回は必要なコンポーネントを追加してください。

```sh
rustup component add --toolchain stable rustfmt clippy rust-analyzer rust-src
```

CIでは整形、Clippy、rust-analyzerの診断、テストを順に実行します。ローカルでも次のコマンドで確認できます。

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
rustup run stable rust-analyzer --version
rustup run stable rust-analyzer diagnostics . --severity warning
cargo test --locked
cargo doc --locked --no-deps
```

rust-analyzerはstableに導入した本体を明示的に使い、CLIの`diagnostics`で警告以上の診断をファイル・位置・メッセージ付きで表示します。警告だけなら成功、エラーや起動失敗は非ゼロ終了となり、CIも失敗します。

このCLIでは診断ルールを設定ファイルで指定できず、VSCodeの設定も読み込みません。rust-analyzer 1.100.0での確認では、`#[cfg(test)]`内や`#[test]`関数内の一部の型エラーが未検出でした。既存のClippyと`cargo test`も実行して検証します。なお、CIのClippyは`-D warnings`を指定しているため、Clippyの警告はCI失敗として扱います。

CLIサブコマンドには安定性保証がないため、Rust更新時はバージョンと`rustup run stable rust-analyzer --help`を確認してください。

標準ライブラリやderiveマクロの読み込みに失敗する場合は、`rustup show active-toolchain`と`rustup component list --toolchain stable --installed`でツールチェーンとコンポーネントを確認してください。`rust-src`が同じstableに存在することを確認し、次のコマンドでsysroot・build script・proc macroの読み込みログを取得できます。診断を消すためだけにコードを書き換える前に、読み込み状況を確認してください。

```sh
RA_LOG=project_model=info,proc_macro_api=info rustup run stable rust-analyzer diagnostics . --severity warning
```

上の環境変数指定はUnix系シェル向けです。PowerShellでは先に`$env:RA_LOG = 'project_model=info,proc_macro_api=info'`を設定してから診断コマンドを実行してください。

生成されたAPIドキュメントは`target/doc/generate_passwd/index.html`から参照できます。このREADMEのRustコード例も、ドキュメントテストで検証します。

単体テストでは固定乱数やエラーを返す乱数源を使い、文字の選択・棄却・区切り位置・エラーの伝播を再現します。最小エントロピーでは、求めた文字数が要求を満たし、1文字減らすと不足することを確認します。最大文字数では、上限内に収まり、1文字増やすと上限を超えることを確認します。

CLI統合テストでは実行ファイルを起動し、文字集合、長さ、標準出力・標準エラー出力、終了コードを確認します。生成されるパスワードそのものを固定値とは比較しません。一部のテストはUnixまたはLinux限定です。

## ライセンス

本パッケージは `AGPL-3.0-or-later` で公開しています。README全体、テストコード、API文書内のコード例にはISCライセンスを適用します。

| 対象 | ライセンス | 本文 |
| --- | --- | --- |
| CLI・Rustライブラリの本体、ソース内のAPI文書の説明文 | GNU Affero General Public License v3.0 or later（`AGPL-3.0-or-later`） | [LICENSE](https://github.com/inukai-norio/generate_passwd/blob/main/LICENSE) |
| 単体テストと補助コード（`src/lib.rs`、`src/cli.rs`、`src/plan.rs` 内の `#[cfg(test)] mod tests`）、統合テスト（`tests/`） | ISC（`ISC`） | [TEST_CODE_LICENSE](https://github.com/inukai-norio/generate_passwd/blob/main/TEST_CODE_LICENSE) |
| README全体（API文書への取り込み部分を含む）、API文書内のコード例 | ISC（`ISC`） | [TEST_CODE_LICENSE](https://github.com/inukai-norio/generate_passwd/blob/main/TEST_CODE_LICENSE) |

依存ライブラリには、それぞれのライセンスが適用されます。

配布ZIPには、本体とRust依存クレートのライセンス本文・著作権表示をまとめた `LICENSES.html` と、READMEなどのISC許諾を示す `TEST_CODE_LICENSE` を同梱しています。`LICENSES.html` はブラウザーで開いて確認できます。

Linux musl版には、muslの著作権表示・許諾文をまとめた `MUSL_LICENSE` も同梱しています。取得元と更新方法は、ソースリポジトリの [muslのライセンス表示の取得元](https://github.com/inukai-norio/generate_passwd/blob/main/third-party/musl/README.md) に記載しています。
