/// 挨拶生成時のエラー種別
#[derive(Debug, PartialEq, Eq)]
pub enum GreetingError {
    /// 性別が未指定
    UnknownGender,
    /// 無効な性別が指定された
    InvalidGender(String),
}

impl std::fmt::Display for GreetingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownGender => write!(f, "gender not specified"),
            Self::InvalidGender(gender) => write!(f, "invalid gender: {gender}"),
        }
    }
}

impl std::error::Error for GreetingError {}

/// 性別を考慮した挨拶メッセージを生成
///
/// # Arguments
/// * `name` - 挨拶対象の名前
/// * `gender` - 性別（None, Some("man"), Some("woman"), その他）
///
/// # Returns
/// * `Ok(String)` - 正常な挨拶文字列
/// * `Err(GreetingError::InvalidGender)` - 無効な性別（回復可能）
///
/// # Errors
/// 無効な性別が指定された場合に `GreetingError::InvalidGender` を返します。
///
/// # Examples
/// ```
/// use hello::sayhello;
///
/// // 成功例
/// let result = sayhello("Alice", Some("woman"));
/// assert!(matches!(result, Ok(_)));
///
/// // エラー例
/// let result = sayhello("Bob", Some("invalid"));
/// assert!(matches!(result, Err(_)));
/// ```
pub fn sayhello(name: &str, gender: Option<&str>) -> Result<String, GreetingError> {
    match gender {
        Some("man") => Ok(format!("Hi, Mr. {name}")),
        Some("woman") => Ok(format!("Hi, Ms. {name}")),
        None => Ok(format!("Hi, {name}")),
        Some(invalid) => Err(GreetingError::InvalidGender(String::from(invalid))),
    }
}

#[cfg(test)]
mod tests {
    use super::{GreetingError, sayhello};

    #[test]
    fn test_sayhello_with_gender_man() {
        let result = sayhello("John", Some("man"));
        assert!(result.is_ok());
        if let Ok(greeting) = result {
            assert_eq!(greeting, "Hi, Mr. John");
        }
    }

    #[test]
    fn test_sayhello_with_gender_woman() {
        let result = sayhello("Alice", Some("woman"));
        assert!(result.is_ok());
        if let Ok(greeting) = result {
            assert_eq!(greeting, "Hi, Ms. Alice");
        }
    }

    #[test]
    fn test_sayhello_with_gender_none() {
        let result = sayhello("Bob", None);
        assert!(result.is_ok());
        if let Ok(greeting) = result {
            assert_eq!(greeting, "Hi, Bob");
        }
    }

    #[test]
    fn test_sayhello_with_gender_invalid() {
        let result = sayhello("Charlie", Some("other"));
        assert!(matches!(result, Err(GreetingError::InvalidGender(_))));
        if let Err(GreetingError::InvalidGender(gender)) = result {
            assert_eq!(gender, "other");
        }
    }

    #[test]
    fn test_sayhello_with_gender_empty_string() {
        let result = sayhello("Dave", Some(""));
        assert!(matches!(result, Err(GreetingError::InvalidGender(_))));
        if let Err(GreetingError::InvalidGender(gender)) = result {
            assert_eq!(gender, "");
        }
    }

    #[test]
    fn test_greeting_error_display() {
        assert_eq!(
            GreetingError::UnknownGender.to_string(),
            "gender not specified"
        );
        assert_eq!(
            GreetingError::InvalidGender(String::from("test")).to_string(),
            "invalid gender: test"
        );
    }
}
