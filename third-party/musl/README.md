<!-- SPDX-License-Identifier: ISC -->
<!-- Copyright 2026 inukai-norio <inukay@proton.me> -->

# muslのライセンス表示の取得元

リポジトリ直下の [MUSL_LICENSE](../../MUSL_LICENSE) は、Linux musl版に含まれるmuslのライセンス表示です。公式の `COPYRIGHT` 全文と、個別ソースの著作権表示・許諾文をまとめています。muslの原文には、それぞれの権利者のライセンスが適用されます。このREADME自体は [ISC](../../TEST_CODE_LICENSE) です。

## 対象と確認した構成

対象は次の3ターゲットです。

- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `i686-unknown-linux-musl`

GitHub Actionsの調査用ワークフローが出力したリンカーマップを確認し、3ターゲットともRust同梱の `lib/rustlib/<target>/lib/self-contained/libc.a` を使用していることを確認しました。

| 項目 | 確認した値 |
| --- | --- |
| Rust | `1.100.0 (0ef1bc386 2026-11-09)` |
| Rustコミット | `0ef1bc386059596dada8fb65b50425aab2ea637a` |
| cross | `0.2.5 (a9301e9 2026-10-31)` |
| musl | `1.2.5` にRust配布用のバックポートを適用したもの |
| musl-cross-makeコミット | `3635262e4524c991552789af6f36211a335a77b3`（x86_64・AArch64用） |

版の根拠は、上記Rustコミットに含まれる以下のビルド定義です。

- x86_64: [dist-x86_64-musl/Dockerfile](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/host-x86_64/dist-x86_64-musl/Dockerfile) と [musl-toolchain.sh](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/scripts/musl-toolchain.sh)
- AArch64: [dist-arm-linux-musl/Dockerfile](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/host-x86_64/dist-arm-linux-musl/Dockerfile) と同じ `musl-toolchain.sh`
- i686: [dist-i586-gnu-i586-i686-musl/Dockerfile](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/host-x86_64/dist-i586-gnu-i586-i686-musl/Dockerfile) と [musl.sh](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/scripts/musl.sh)

## 原文の取得元

- 公式ソース: <https://musl.libc.org/releases/musl-1.2.5.tar.gz>
- 公式COPYRIGHT: <https://git.musl-libc.org/cgit/musl/plain/COPYRIGHT?h=v1.2.5>

配布アーカイブの `COPYRIGHT` と、公式Gitから取得した原文が一致することを確認しています。`MUSL_LICENSE` 内では全文をそのまま保持し、続けて以下の個別表示を掲載しています。

| 掲載元（musl 1.2.5内のパス） | 掲載した表示 | 対象 |
| --- | --- | --- |
| `src/stdlib/qsort.c` | Valentin Ochsの著作権表示・MIT許諾文・免責条項を含む冒頭コメント全文 | 全3ターゲット |
| `src/string/aarch64/memcpy.S` | Arm Limitedの著作権表示とMIT指定を含む冒頭コメント全文 | AArch64 |
| `src/string/aarch64/memset.S` | 同上 | AArch64 |
| `src/thread/x86_64/__set_thread_area.s` | Nicholas J. Kainの著作権表示とMIT指定 | x86_64 |
| `src/thread/x86_64/__unmapself.s` | 同上。直前のファイルと同一の表示なので、両パスを示して1回掲載 | x86_64 |

Arm LimitedとNicholas J. Kainの表示に対応するMITの許諾文・免責条項は、同じファイル内の公式 `COPYRIGHT` に掲載しています。

リンカーマップに現れるmuslアーカイブメンバーについて、muslのMakefileが選択する汎用ソース・アーキテクチャ別ソース、mallocng、i686のtime32互換ソースと照合しました。マップには破棄されるセクションも含まれるため、表示の採取対象は最終的に残る部分より広く取っています。内部ヘッダーと対象アーキテクチャのヘッダーも確認しました。

公式 `COPYRIGHT` が言及するTRE、crypt、他アーキテクチャのコード等について、この調査のマップに対応するオブジェクトが現れないものの個別ソース表示は追加していません。公式 `COPYRIGHT` 内のそれらの説明は、省略せず保持しています。

## バックポートの確認

以下の5つのパッチを公式v1.2.5ソースに適用できることと、適用後も `COPYRIGHT` および上記の個別表示が変わらないことを確認しました。パッチには著作権表示・ライセンス条項を変更する差分はありません。

| パッチ | 取得元 | 変更するファイル |
| --- | --- | --- |
| `0001-cve-2025-26519-p1.diff` | [musl-cross-make](https://github.com/richfelker/musl-cross-make/blob/3635262e4524c991552789af6f36211a335a77b3/patches/musl-1.2.5/0001-cve-2025-26519-p1.diff) | `src/locale/iconv.c` |
| `0002-cve-2025-26519-p2.diff` | [musl-cross-make](https://github.com/richfelker/musl-cross-make/blob/3635262e4524c991552789af6f36211a335a77b3/patches/musl-1.2.5/0002-cve-2025-26519-p2.diff) | `src/locale/iconv.c` |
| `musl-cve-2026-6042.diff` | [Rust](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/scripts/musl-cve-2026-6042.diff) | `src/locale/iconv.c`、`src/locale/gb18030utf.h` |
| `musl-cve-2026-40200.diff` | [Rust](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/scripts/musl-cve-2026-40200.diff) | `src/stdlib/qsort.c` |
| `musl-cve-2026-107891.diff` | [Rust](https://github.com/rust-lang/rust/blob/0ef1bc386059596dada8fb65b50425aab2ea637a/src/ci/docker/scripts/musl-cve-2026-107891.diff) | `src/regex/regcomp.c` |

i686用の `musl.sh` は、最初の2パッチに対応する修正をスクリプト内で適用します。x86_64・AArch64用の `musl-toolchain.sh` は、上記コミットのmusl-cross-makeに含まれる2パッチを利用します。残り3パッチはいずれもRustリポジトリから適用します。

パッチの作者表示にはRich Felker、Luca Kellermann、Szabolcs Nagyが含まれます。作者表示や原文のライセンスを本プロジェクトの著作権表示に置き換えていません。

## 照合用SHA-256

| 対象 | SHA-256 |
| --- | --- |
| 公式 `musl-1.2.5.tar.gz` | `a9a118bbe84d8764da0ea0d28b3ab3fae8477fc7e4085d90102b8596fc7c75e4` |
| 公式 `COPYRIGHT` | `f9bc4423732350eb0b3f7ed7e91d530298476f8fec0c6c427a1c04ade22655af` |
| リポジトリ直下の `MUSL_LICENSE` | `bc467106031631ef7a409416c91e1d5ee541875eb5f05ab3602e7854c24a5166` |

## 更新時の確認

1. `.github/workflows/musl-diagnostics.yml` を手動実行し、3ターゲットのリンカーマップ・Rustコミット・crossの取得元・イメージ情報を取得する。
2. 実際にリンクしたlibcの提供元と、そのビルド定義で指定されたmuslの版・パッチを確認する。
3. 対応する公式ソースの `COPYRIGHT` 全文と、アーカイブメンバーに対応する個別ソースの表示を照合する。新しいターゲットやライブラリの使用範囲が増えた場合も採取対象を見直す。
4. `MUSL_LICENSE` の原文・版・適用対象と、このREADMEの取得元・SHA-256を更新する。
5. 配布ZIPに更新後のファイルが含まれることを検証する。

Rustやcrossを更新した場合は、このファイルに記録した版と同じだと仮定せず、再調査します。
