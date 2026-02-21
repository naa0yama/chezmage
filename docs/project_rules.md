# Rustプロジェクトルール

## 1. 開発環境

devcontainer を利用した環境ですべて設定済み

- Rust toolchain: rust-toolchain.toml で固定
- Edition: `2024`

## 2. ワークフロー

- ワークを計画
- ブランチを作成
- 実装
- 該当ファイルのみを Git stage に登録
- `mise run pre-commit` でコミットチェックを実施
  - エラーの場合は修正して再度 Git ステージに追加し `mise run pre-commit` を実施
- ドキュメントを更新

## 3. コーディング規約

### 3.1 命名規則

#### 基本ルール(Rust標準に準拠)

- **モジュール・関数・変数**: `snake_case`
- **型・トレイト**: `PascalCase`
- **定数**: `SCREAMING_SNAKE_CASE`
- **ライフタイム**: `'lowercase`

#### クレート名

- `-rs`や`-rust`のサフィックス・プレフィックスは禁止
- アンダースコア使用: `hoge_fuga`(ハイフン`hoge-huga`は避ける)

#### 変換メソッド命名

```rust
// コスト無し: as_*
fn as_bytes(&self) -> &[u8]

// コストあり: to_*
fn to_string(&self) -> String

// 所有権移動: into_*
fn into_vec(self) -> Vec<T>
```

#### ゲッター命名

```rust
impl S {
    // get_は付けない
    pub fn first(&self) -> &First { &self.first }

    // mut版
    pub fn first_mut(&mut self) -> &mut First { &mut self.first }
}
```

### 3.2 import/use文のルール

#### use文の配置

すべての `use` 文はファイル先頭にまとめる（Rust コミュニティ標準に準拠）。

```rust
// ✅ ファイル先頭にグループ化して記述
// 1. std
use std::collections::HashMap;
use std::sync::Arc;

// 2. 外部クレート
use anyhow::Context;
use serde::Deserialize;

// 3. crate/super
use crate::libs::hello::sayhello;
```

#### 禁止事項

```rust
// ❌ ワイルドカードインポート
use hoge::prelude::*;  // 禁止
```

#### 許可事項

```rust
// ✅ エイリアス（名前衝突解消、re-export で使用可）
use std::fmt::Result as FmtResult;

// ✅ Result 型エイリアス（std::io::Result 等と同じ慣用パターン）
type Result<T> = std::result::Result<T, MyError>;
```

### 3.3 エラーハンドリング

#### エラー型の設計

```rust
// 推奨: thiserror でドメインエラーを定義
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("not found")]
    NotFound,
}

// ライブラリ: 具体的なエラー型を返す
pub fn process(input: &str) -> Result<Output, MyError> { /* ... */ }

// アプリケーション: anyhow で集約
pub fn run() -> anyhow::Result<()> { /* ... */ }
```

#### エラーコンテキストの追加

```rust
use anyhow::Context;

// ✅ 必ずコンテキストを追加
hoge.process(param)
    .context(format!("process failed with param: {param:?}"))?;

// ❌ 裸の?は避ける
hoge.process(param)?;  // どこで何が起きたか不明
```

#### 環境変数設定

```bash
RUST_BACKTRACE=1  # 本番環境でも必須
```

### 3.5 CLI設計ガイドライン(clapクレート使用)

#### CLIオプション設計

```rust
#[derive(Parser)]
#[command(about)]  // Cargo.tomlのdescriptionを使用
struct Args {
    /// 必須の引数(短縮形と長形式の両方)
    #[arg(short, long)]
    name: String,

    /// オプション引数(デフォルト値設定)
    #[arg(short, long, default_value = "default")]
    option: String,

    /// フラグ(存在チェック)
    #[arg(long, short = 'V', help = "Print version")]
    version: bool,
}
```

#### ヘルプメッセージの品質

- 各オプションに簡潔で明確な説明を追加
- 使用例を `#[command(after_help = "...")]` で提供
- バージョン情報は build.rs で Git ハッシュ含めて生成

### 3.4 非同期プログラミング

#### ライフタイム管理

```rust
// 複雑な参照は避け、Cloneを活用
Arc<Mutex<T>>  // Send + Sync + 'static

// String のcloneを恐れない(&strより実装が簡単)
let data = input.to_string();  // clone OK
```

#### async fn の制限

```rust
// async fnが使えない場合は明示的にFutureを返す
#[allow(clippy::manual_async_fn)]
fn process<'a>() -> impl Future<Output = Result<()>> + Send + 'a {
    async move {
        // 処理
    }
}
```

## 4. プロジェクト構造

### 4.1 基本構造(CLIプロジェクト)

```
project-name/
├── .cargo/
│   └── config.toml                     # Cargo設定
├── .devcontainer/                      # 開発環境設定
├── .github/                            # GitHub Actions & 設定
│   ├── actions/
│   │   ├── act-setup-rust/             # Rust のセットアップ
│   │   └── create-release/             # Release 作成 action
│   ├── workflows/                      # CI/CD ワークフロー
│   │   ├── audit.yaml                  # cargo audit の定期実行
│   │   ├── ci.yaml                     # CI pipeline
│   │   ├── cleanup.yaml                # Cache/ untag container
│   │   ├── pr-labeler.yaml
│   │   ├── prebuild-container.yaml
│   │   ├── release-build.yaml          # Release の Build workflow
│   │   ├── release.yaml                # Release oneshot 用 workflow
│   │   └── tagpr.yaml
│   ├── labeler.yml
│   └── release.yml
├── .githooks/                          # Git hooks (mise run 連携)
│   ├── commit-msg                      # Conventional Commits 検証
│   ├── pre-commit                      # コミット前チェック
│   └── pre-push                        # プッシュ前チェック
├── .vscode/                            # VS Code設定
│   ├── launch.json                     # デバッグ設定
│   └── settings.json                   # ワークスペース設定
├── docs/                               # ドキュメント
│   └── project_rules.md                # プロジェクトルール
├── src/                                # ソースコード
│   ├── main.rs                         # CLIエントリーポイント
│   ├── libs.rs                         # ライブラリモジュール定義
│   └── libs/                           # ビジネスロジック
│       └── hello.rs                    # 個別機能モジュール
├── tests/                              # 統合テスト
│   └── integration_test.rs
├── .editorconfig                       # エディター設定
├── .gitignore                          # Git除外設定
├── .octocov.yml                        # カバレッジレポート設定
├── .tagpr                              # タグ&リリース設定
├── ast-rules/                          # ast-grep プロジェクトルール
├── build.rs                            # ビルドスクリプト
├── Cargo.lock                          # 依存関係ロックファイル
├── Cargo.toml                          # プロジェクト設定と依存関係
├── Dockerfile                          # devcontainer 環境ファイル
├── dprint.jsonc                        # フォーマッター設定
├── mise.toml                           # ツール管理 & タスクランナー
├── LICENSE                             # ライセンスファイル
├── README.md                           # プロジェクト説明
├── renovate.json                       # 依存関係自動更新
├── rust-toolchain.toml                 # Rust toolchain バージョン固定
└── sgconfig.yml                        # ast-grep 設定ファイル
```

### 4.2 モジュール設計

#### 可視性の原則

- デフォルトはprivate
- 必要最小限のみpublic
- `pub(crate)`を活用して内部実装を隠蔽

#### ファイル分割

```rust
// 一つの責務 = 一つのモジュール
// 500行を超えたら分割を検討
```

## 5. テスト戦略

### 4.1 テストの種類

#### 単体テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // arrange
        let input = prepare_input();

        // act
        let result = process(input);

        // assert
        assert_eq!(result, expected);
    }
}
```

#### 統合テスト

- 全ての CLI オプション・フラグをテスト
- 正常系・異常系の両方をカバー
- `assert_cmd` でプロセス実行をテスト
- `predicates` で出力内容を検証

```rust
// tests/integration_test.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_with_name_argument() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("--name").arg("test");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Hi, test"));
}

#[test]
fn cli_version_flag() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("version"));
}
```

### 4.2 テストユーティリティ

```rust
// テストでのtracing出力モック
#[test]
fn test_with_tracing_mock() {
    use tracing::subscriber::with_default;
    use tracing_mock::{expect, subscriber};

    let subscriber = subscriber::mock()
        .event(expect::event().with_target(env!("CARGO_PKG_NAME")))
        .run();

    with_default(subscriber, || {
        // テスト実行
        crate::run("test".to_string());
    });
}

// 単体テストでのtracing初期化
#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt;

    #[test]
    fn test_sayhello() {
        // テスト用のtracing初期化
        fmt().with_test_writer().init();

        let result = sayhello("Alice".to_string());
        assert_eq!(result, "Hi, Alice");
    }
}
```

## 6. CI/CD

### 5.1 必須チェック(`mise run` 経由)

```bash
# フォーマットチェック
mise run fmt:check       # cargo fmt --check + dprint check

# 静的解析
mise run clippy          # clippy (warnings only, TDD向け)
mise run clippy:strict   # clippy (warnings as errors, CI/pre-commit向け)
mise run ast-grep        # ast-grep project rules check

# テスト実行
mise run test            # unit & integration tests

# ビルドチェック
mise run build           # debug build
mise run build:release   # release build

# カバレッジ
mise run coverage        # code coverage report

# クロスコンパイル
mise run zigbuild:all    # Tier 1 targets
```

### 5.2 品質基準

- **Warning一切禁止**
- **フォーマット違反禁止**
- **カバレッジ目標**: 80%以上

### 5.3 クロスコンパイル対応

```bash
# Tier 1 targets（全て対応）
mise run zigbuild:all
# - aarch64-apple-darwin    (Apple Silicon macOS)
# - aarch64-unknown-linux-gnu (ARM64 Linux)
# - x86_64-pc-windows-gnu   (Windows)
# - x86_64-unknown-linux-gnu (Intel/AMD Linux)

# 個別ターゲット
TARGET=x86_64-pc-windows-gnu mise run zigbuild
```

### 5.4 Git フック(`.githooks/` + `mise`)

#### 事前チェック(pre-commit)

`.githooks/` と `mise` によりコミット時に自動的に品質チェックを実行:

```bash
# mise により自動実行される項目
mise run pre-commit
```

- **コミット時**: 上記チェックが失敗すると自動的にコミット拒否
- **品質保証**: 全てのチェックに合格したコードのみリポジトリに取り込み

## 7. ドキュメント

### 6.1 コメント規約

#### ドキュメントコメント

````rust
/// 関数の簡潔な説明
///
/// # Arguments
/// * `param` - パラメータの説明
///
/// # Returns
/// 戻り値の説明
///
/// # Errors
/// エラー条件の説明
///
/// # Panics
/// パニック条件の説明
///
/// # Examples
/// ```
/// let result = function(param);
/// assert_eq!(result, expected);
/// ```
pub fn function(param: Type) -> Result<ReturnType> {
    // 実装
}
````

### 6.2 README必須項目(CLIプロジェクト)

- プロジェクト概要(CLIツールの目的)
- Dev Container を使ったセットアップ手順
- `mise run` を使ったビルド・テスト方法
- CLI の使用方法とオプション
- クロスコンパイル手順
- VSCode拡張機能一覧
- GitHub Actions による CI/CD 説明

## 8. 依存関係管理

### 8.1 依存関係の選定基準

- メジャーバージョン 1.0 以上を優先
- ダウンロード数・スター数を確認
- 最終更新日を確認(6ヶ月以内が理想)
- ライセンスの確認
- すでに利用しているパッケージの類似の場合、 3rd party より本家を優先する

## 9. デバッグとロギング

### 9.1 ロギング設定(tracingクレート使用)

```rust
// main.rsの最初に
use tracing_subscriber::{filter::EnvFilter, fmt};

fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    tracing::info!("Application started");
}

// printlnではなくtracing使用
tracing::debug!("Debug information: {:?}", data);
tracing::error!("Error occurred: {}", err);
tracing::info!("Process completed successfully");
```

#### OpenTelemetry 対応（`otel` feature 有効時）

コンテナ環境等で OpenTelemetry (OTLP) によるトレースエクスポートが必要な場合、
`--features otel` でビルドし、環境変数 `OTEL_EXPORTER_OTLP_ENDPOINT` を設定する。

```rust
// otel feature 有効時の初期化（main.rs）
// OTEL_EXPORTER_OTLP_ENDPOINT が設定されていれば OTel レイヤーが追加される
// 未設定の場合は fmt のみ（従来と同じ動作）
tracing_subscriber::registry()
    .with(env_filter)
    .with(fmt_layer)
    .with(otel_layer) // Option<Layer>: None なら無視
    .init();
```

**ビルド方法:**

```bash
# ターミナル用（OTel なし）
cargo build --release

# コンテナ用（OTel 対応）
cargo build --release --features otel
```

**コンテナ実行時の環境変数:**

| 環境変数                      | 必須 | 説明                                                        |
| ----------------------------- | ---- | ----------------------------------------------------------- |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Yes  | OTel Collector エンドポイント (例: `http://localhost:4318`) |
| `OTEL_SERVICE_NAME`           | No   | サービス名 (デフォルト: パッケージ名)                       |
| `RUST_LOG`                    | No   | ログレベル (デフォルト: `info`)                             |

**注意:**

- アプリケーションコードの `tracing::info!` 等は変更不要
- `otel` feature 無効時は OTel 依存が一切含まれず、従来のバイナリと同一

### 9.2 デバッグ手法

- `println!` デバッグは開発時のみ
- 本番コードは `tracing` クレート使用を必須
- 複雑なバグは二分探索でprint debug
- ast-grepにより自動的に `tracing` 以外の出力を検知

### 9.3 ログ出力の自動検知(ast-grep)

出力方法を自動検知し、 `tracing` 使用を促します

#### 適切な例外の指定

正当な理由がある場合は、該当行に無視コメントを追加:

```rust
// Cargo ビルドスクリプトでの正当用途
// ast-grep-ignore: no-println-debug
println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
```

#### 無視コメントの種類

- `// ast-grep-ignore` - 次の行のすべて診断を無視
- `// ast-grep-ignore: rule-id` - 特定のルールのみ無視
- `// ast-grep-ignore: rule-1, rule-2` - 複数ルールを無視

## 10. パフォーマンス最適化

### 9.1 最適化の原則

- **計測なき最適化は悪**
- まず動くものを作る
- ボトルネックを`cargo build --timings`で特定
- 必要な箇所のみ最適化

### 9.2 メモリ管理

```rust
// 開発速度優先
// cloneを恐れない、まず動かす
let data = original.clone();

// 後から最適化
Arc<String>  // 共有が必要な場合
Cow<'a, str>  // 条件付きクローン
```

## 11. セキュリティ

### 11.1 基本原則

- `unsafe`使用は原則禁止
- 使用する場合は必ず`// SAFETY:`コメント
- 外部入力は必ず検証

### 11.2 依存関係の監査

```bash
cargo audit  # 定期実行
cargo outdated  # アップデート確認
```

## 12. トレイト実装

### 12.1 必須トレイト実装

標準的なデータ型には以下を実装：

- `Debug`(必須)
- `Clone`(可能な限り)
- `PartialEq`, `Eq`
- `PartialOrd`, `Ord`(順序がある場合)
- `Hash`(ハッシュマップのキーになる場合)
- `Default`(妥当なデフォルト値がある場合)
- `Display`(ユーザー向け出力がある場合)

### 12.2 Serdeサポート

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }

[features]
default = []
serde = ["dep:serde"] # オプショナル機能として
```

## 13. マクロ使用ガイドライン

### 13.1 原則

- **マクロは最終手段**
- 関数やジェネリクスで解決できないか検討
- 使用する場合は十分なドキュメントを記載

### 13.2 許可されるマクロ

- `println!`, `format!`等の標準マクロ
- `derive`マクロ
- テスト用のヘルパーマクロ

## 14. feature フラグ

### 14.1 命名規則

```toml
[features]
default = ["std"]
std = [] # no-std対応の場合

# ❌ use-std, with-stdなどは使わない
```

#### プロジェクト定義の feature フラグ

```toml
[features]
default = []
otel = [...]  # OpenTelemetry 対応（コンテナ環境向け）
```

- `otel`: OpenTelemetry トレースエクスポート機能を有効化。コンテナビルド時に `--features otel` で指定

## 15. コードレビュー基準

### 15.1 必須確認項目

- [ ] `mise run pre-commit` 実行済み
- [ ] `mise run clippy:strict` 警告なし
- [ ] `mise run ast-grep` エラーなし
- [ ] テスト追加・更新
- [ ] ドキュメント更新
- [ ] エラーハンドリング適切
- [ ] `unwrap()` の正当性確認

### 15.2 推奨確認項目

- [ ] パフォーマンス影響の検討
- [ ] 後方互換性の維持
- [ ] 依存関係の妥当性

## 参照

本ルールは以下の資料を参考に策定：

- Rust公式ドキュメント
  - [The Rust Programming Language(公式ドキュメント)](https://doc.rust-lang.org/book/)(1次ソース)
  - [Rust API Guidelines(公式)](https://rust-lang.github.io/api-guidelines/)(1次ソース)
  - [Rust Style Guide(公式)](https://doc.rust-lang.org/1.0.0/style/)(1次ソース)

- CLIプロジェクト特有の参考資料
  - [clap Documentation](https://docs.rs/clap/) - CLI引数解析
  - [tracing Documentation](https://docs.rs/tracing/) - 構造化ログ
  - [reqwest Documentation](https://docs.rs/reqwest/) - HTTP クライアント
  - [assert_cmd Documentation](https://docs.rs/assert_cmd/) - CLI テスト
  - [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) - クロスコンパイル
  - [OpenTelemetry Rust](https://docs.rs/opentelemetry/) - 分散トレーシング
  - [tracing-opentelemetry](https://docs.rs/tracing-opentelemetry/) - tracing → OTel ブリッジ

- 開発環境・ツール
  - [mise Documentation](https://mise.jdx.dev/) - ツール管理 & タスクランナー
  - [dprint Documentation](https://dprint.dev/) - コードフォーマッター
  - [Dev Containers](https://code.visualstudio.com/docs/devcontainers/containers) - 開発環境
