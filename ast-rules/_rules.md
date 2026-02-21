# ast-grep ルール解説

プロジェクトで使用しているast-grepルールの詳細解説です。各ルールについてダメな例、良い例、理由を示します。

> **Note**: 以下のルールは clippy に移行済みのため、ast-grep からは削除されました:
>
> - `unsafe-needs-safety-comment` -> `clippy::undocumented_unsafe_blocks`
> - `no-unwrap-in-production` -> `clippy::unwrap_used` + `.clippy.toml` の `allow-unwrap-in-tests`
> - `no-println-debug` -> `clippy::print_stdout` + `print_stderr` + `dbg_macro`
> - `no-wildcard-import` -> `clippy::wildcard_imports`
> - `no-ignored-result` -> rustc `unused_must_use`
> - `require-pub-doc-comment` -> rustc `missing_docs`
> - `avoid-nested-matches` -> `clippy::collapsible_match`
> - `prefer-vec-with-capacity` -> `clippy::reserve_after_initialization`
> - `optimize-string-concat` -> `clippy::manual_string_new` + 既存の文字列系 lint
> - `prefer-iterator-over-loop` -> `clippy::manual_filter_map` + `map_flatten` + pedantic グループ
>
> 以下のルールは Rust コミュニティ標準に反するため削除されました:
>
> - `no-file-level-external-use` - ファイル先頭の外部 `use` は Rust の標準スタイル
> - `no-type-result-override` - `type Result<T> = ...` は `std::io::Result` 等と同じ慣用パターン
> - `no-use-alias` - `use X as Y` は名前衝突解消、re-export で正当に使用される
> - `prefer-nested-result` - ネスト Result は非標準。`thiserror`/`anyhow` が標準的アプローチ

---

## コード組織

### module-size-limit

**目的**: 大きすぎるモジュールの警告

#### ダメな例

```rust
// 1つのファイルに10個以上の関数 - 責務が不明確
mod user_management {
    pub fn create_user() { /* ... */ }
    pub fn update_user() { /* ... */ }
    pub fn delete_user() { /* ... */ }
    pub fn validate_email() { /* ... */ }
    pub fn hash_password() { /* ... */ }
    pub fn send_welcome_email() { /* ... */ }
    pub fn log_user_action() { /* ... */ }
    pub fn calculate_permissions() { /* ... */ }
    pub fn format_username() { /* ... */ }
    pub fn cleanup_old_sessions() { /* ... */ }
    pub fn generate_api_key() { /* ... */ }
    // さらに多数の関数...
}
```

#### 良い例

```rust
// 責務で分割
mod user {
    pub fn create() { /* ... */ }
    pub fn update() { /* ... */ }
    pub fn delete() { /* ... */ }
}

mod validation {
    pub fn validate_email() { /* ... */ }
    pub fn validate_password() { /* ... */ }
}

mod auth {
    pub fn hash_password() { /* ... */ }
    pub fn generate_api_key() { /* ... */ }
}

mod notification {
    pub fn send_welcome_email() { /* ... */ }
}
```

#### 理由

- 単一責務原則の遵守
- コードの可読性・保守性向上
- テストのしやすさ向上

---

### error-context-required

**目的**: エラーにコンテキスト情報を追加

#### ダメな例

```rust
fn load_user_config(user_id: u32) -> Result<Config, Error> {
    let path = format!("/users/{}/config.toml", user_id);
    // エラー情報不足 - どのファイルで何が失敗したか不明
    let content = std::fs::read_to_string(&path)?;
    let config = toml::from_str(&content)?;
    Ok(config)
}
```

#### 良い例

```rust
use anyhow::{Context, Result};

fn load_user_config(user_id: u32) -> Result<Config> {
    let path = format!("/users/{}/config.toml", user_id);

    // 詳細なエラーコンテキスト
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("ユーザー{}の設定ファイル読み込み失敗: {}", user_id, path))?;

    let config = toml::from_str(&content)
        .with_context(|| format!("ユーザー{}の設定ファイル解析失敗", user_id))?;

    Ok(config)
}
```

#### 理由

- デバッグ時の問題特定が容易
- エラーログの品質向上
- 運用時のトラブルシューティング効率化

---

### no-blocking-in-async

**目的**: `async` 関数内での同期 I/O 操作を禁止

#### ダメな例

```rust
async fn load_config() -> Result<Config, Error> {
    // async関数内で同期I/O - スレッドブロッキング
    let content = std::fs::read_to_string("config.toml")?;

    // 同期sleep - 他のタスクもブロック
    std::thread::sleep(Duration::from_secs(1));

    parse_config(&content)
}
```

#### 良い例

```rust
async fn load_config() -> Result<Config, Error> {
    // async版I/O - 他のタスクをブロックしない
    let content = tokio::fs::read_to_string("config.toml").await?;

    // async版sleep
    tokio::time::sleep(Duration::from_secs(1)).await;

    parse_config(&content)
}
```

#### 理由

- 非同期実行環境でのスレッドブロッキング回避
- 並行性の維持
- スケーラビリティの確保

---

## セキュリティ

### no-hardcoded-credentials

**目的**: ハードコードされた認証情報を検出

#### ダメな例

```rust
fn connect_to_database() -> Connection {
    // ハードコードされた認証情報 - セキュリティリスク
    let password = "super_secret_password_123";
    let api_key = "sk-1234567890abcdef1234567890abcdef";

    Database::connect("localhost", "admin", password)
}
```

#### 良い例

```rust
use std::env;

fn connect_to_database() -> Result<Connection, DatabaseError> {
    // 環境変数から取得 - セキュア
    let password = env::var("DB_PASSWORD")
        .context("DB_PASSWORD環境変数が設定されていません")?;
    let api_key = env::var("API_KEY")
        .context("API_KEY環境変数が設定されていません")?;

    Database::connect("localhost", "admin", &password)
}
```

#### 理由

- 認証情報の漏洩防止
- 環境ごとの設定分離
- セキュリティベストプラクティス遵守

---

### secure-random-required

**目的**: セキュリティ用途にセキュアな乱数生成を要求

#### ダメな例

```rust
fn generate_session_token() -> String {
    use rand::Rng;

    // 非セキュアな乱数 - 予測可能性のリスク
    let mut rng = rand::thread_rng();
    (0..32).map(|_| rng.gen::<u8>()).collect()
}
```

#### 良い例

```rust
fn generate_session_token() -> Result<String, CryptoError> {
    use rand::{rngs::OsRng, RngCore};

    // セキュアな乱数生成器
    let mut rng = OsRng;
    let mut token = vec![0u8; 32];
    rng.fill_bytes(&mut token);

    Ok(hex::encode(token))
}
```

#### 理由

- 暗号学的に安全な乱数生成
- セキュリティトークンの品質保証
- 攻撃耐性の向上

---

## プロジェクト固有ルール

### no-get-prefix

**目的**: Rust の `getter` 命名規則遵守

#### ダメな例

```rust
impl User {
    pub fn get_name(&self) -> &str { &self.name } // get_プレフィックス不要
}
```

#### 良い例

```rust
impl User {
    pub fn name(&self) -> &str { &self.name } // Rustの慣例
}
```

---

## ルール無効化

特定の箇所でルールを無効にする場合:

```rust
// 単一ルール無効化
// ast-grep-ignore: error-context-required
let value = some_operation()?;

// 複数ルール無効化
// ast-grep-ignore: no-hardcoded-credentials, no-blocking-in-async
let config = std::fs::read_to_string("config.toml")?;

// 全ルール無効化
// ast-grep-ignore
dangerous_code_here();
```

---

## 参考資料

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [ast-grep Documentation](https://ast-grep.github.io/)
- [Clippy Lints Reference](https://rust-lang.github.io/rust-clippy/master/)
- プロジェクト `project_rules.md`
