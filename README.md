# Boilerplate-Rust

![coverage](https://raw.githubusercontent.com/naa0yama/boilerplate-rust/badges/coverage.svg)
![test execution time](https://raw.githubusercontent.com/naa0yama/boilerplate-rust/badges/time.svg)

Rust プロジェクトのための開発テンプレート

## 概要

このプロジェクトは、Rust 開発を始めるためのボイラープレートです。Dev Containers に対応しており、VS Code での開発環境が簡単に構築できます。

## 必要要件

- Docker
- Visual Studio Code
- VS Code Dev Containers 拡張機能

## セットアップ

1. リポジトリをクローン:

```bash
git clone <repository-url>
cd boilerplate-rust
```

2. VS Codeでプロジェクトを開く:

```bash
code .
```

3. VS Codeのコマンドパレット（`Ctrl+Shift+P` / `Cmd+Shift+P`）から「Dev Containers: Reopen in Container」を選択

## 使い方

すべてのタスクは `mise run <task>` で実行します。

### 基本操作

```bash
mise run build            # デバッグビルド
mise run build:release    # リリースビルド
mise run test             # テスト実行
mise run test:watch       # TDD ウォッチモード
mise run test:doc         # ドキュメントテスト
```

### コード品質

```bash
mise run fmt              # フォーマット (cargo fmt + dprint)
mise run fmt:check        # フォーマットチェック
mise run clippy           # Lint
mise run clippy:strict    # Lint (warnings をエラー扱い)
mise run ast-grep         # ast-grep カスタムルールチェック
```

### コミット前チェック

```bash
mise run pre-commit       # fmt:check + clippy:strict + ast-grep
```

## プロジェクト構造

```
.
├── .cargo/                     # Cargo設定
│   └── config.toml
├── .devcontainer/              # Dev Container設定
│   ├── devcontainer.json       # Dev Container設定ファイル
│   ├── initializeCommand.sh    # 初期化コマンド
│   └── postStartCommand.sh     # 起動後コマンド
├── .githooks/                  # Git hooks (mise run 連携)
│   ├── commit-msg              # Conventional Commits 検証
│   ├── pre-commit              # コミット前チェック
│   └── pre-push                # プッシュ前チェック
├── .github/                    # GitHub Actions & 設定
│   ├── actions/                # カスタムアクション
│   ├── rulesets/               # Protection rulesets
│   ├── workflows/              # CI/CD ワークフロー
│   ├── labeler.yml
│   └── release.yml
├── .vscode/                    # VS Code設定
│   ├── launch.json             # デバッグ設定
│   └── settings.json           # ワークスペース設定
├── ast-rules/                  # ast-grep プロジェクトルール
├── docs/                       # ドキュメント
│   └── project_rules.md        # プロジェクトルール
├── src/                        # ソースコード
│   ├── main.rs                 # アプリケーションのエントリーポイント
│   ├── libs.rs                 # モジュール定義
│   └── libs/
│       └── hello.rs            # Helloモジュール
├── tests/                      # 統合テスト
│   └── integration_test.rs
├── .editorconfig               # エディター設定
├── .gitignore                  # Git除外設定
├── .octocov.yml                # カバレッジレポート設定
├── .tagpr                      # タグ&リリース設定
├── build.rs                    # ビルドスクリプト
├── Cargo.lock                  # 依存関係のロックファイル
├── Cargo.toml                  # プロジェクト設定と依存関係
├── deny.toml                   # cargo-deny 設定
├── Dockerfile                  # Dockerイメージ定義
├── dprint.jsonc                # Dprint フォーマッター設定
├── LICENSE                     # ライセンスファイル
├── mise.toml                   # ツール管理 & タスクランナー
├── README.md                   # このファイル
├── renovate.json               # Renovate自動依存関係更新設定
├── rust-toolchain.toml         # Rust toolchain バージョン固定
└── sgconfig.yml                # ast-grep 設定ファイル
```

## VSCode拡張機能

このプロジェクトの Dev Containers には、Rust開発を効率化する以下の拡張機能が含まれています：

### Rust開発

- **[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)** - Rust言語サポート（コード補完、エラー検出、リファクタリング）
- **[CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)** - Rustプログラムのデバッグサポート
- **[Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)** - Cargo.tomlファイルのシンタックスハイライトとバリデーション

### コード品質・フォーマット

- **[Biome](https://marketplace.visualstudio.com/items?itemName=biomejs.biome)** - 高速なフォーマッターとリンター
- **[dprint](https://marketplace.visualstudio.com/items?itemName=dprint.dprint)** - 高速なコードフォーマッター（設定ファイル: `dprint.jsonc`）
- **[EditorConfig for VS Code](https://marketplace.visualstudio.com/items?itemName=EditorConfig.EditorConfig)** - エディター設定の統一
- **[Error Lens](https://marketplace.visualstudio.com/items?itemName=usernamehw.errorlens)** - エラーと警告をインラインで表示

### 開発支援

- **[Claude Code for VSCode](https://marketplace.visualstudio.com/items?itemName=Anthropic.claude-code)** - AIアシスタントによるコーディング支援
- **[Calculate](https://marketplace.visualstudio.com/items?itemName=acarreiro.calculate)** - 選択したテキストの計算式を評価
- **[indent-rainbow](https://marketplace.visualstudio.com/items?itemName=oderwat.indent-rainbow)** - インデントレベルを色分け表示
- **[Local History](https://marketplace.visualstudio.com/items?itemName=xyz.local-history)** - ファイルの変更履歴をローカルに保存

### テキスト編集

- **[lowercase](https://marketplace.visualstudio.com/items?itemName=ruiquelhas.vscode-lowercase)** - 選択テキストを小文字に変換
- **[uppercase](https://marketplace.visualstudio.com/items?itemName=ruiquelhas.vscode-uppercase)** - 選択テキストを大文字に変換
- **[Markdown All in One](https://marketplace.visualstudio.com/items?itemName=yzhang.markdown-all-in-one)** - Markdownファイルの編集支援

## ライセンス

このプロジェクトは [LICENSE](./LICENSE) ファイルに記載されているライセンスの下で公開されています。

## 参考資料

- [The Rust Programming Language 日本語版](https://doc.rust-jp.rs/book-ja/)
- [Developing inside a Container](https://code.visualstudio.com/docs/devcontainers/containers)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)

## Troubleshooting

### Rust debug

```bash
RUST_LOG=trace RUST_BACKTRACE=1 cargo run -- help
```
