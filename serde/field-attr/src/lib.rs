use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use std::borrow::Cow;
use std::fmt::{self, Debug, Display};
use std::path::{PathBuf, absolute};
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize, Default, Eq, PartialEq)]
pub enum FieldAttrValue {
    A,
    #[default]
    B,
}

impl Display for FieldAttrValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldAttrValue::A => write!(f, "A"),
            FieldAttrValue::B => write!(f, "B"),
        }
    }
}

impl FromStr for FieldAttrValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(FieldAttrValue::A),
            "B" => Ok(FieldAttrValue::B),
            _ => Err(format!("invalid value: {}", s)),
        }
    }
}

impl FieldAttrValue {
    pub fn value() -> Self {
        FieldAttrValue::B
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldAttr {
    #[serde(rename = "renamed", skip_serializing_if = "Option::is_none")]
    pub rename: Option<String>,

    #[serde(alias = "aliased", skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    #[serde(
        default,
        serialize_with = "ser_path",
        deserialize_with = "de_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldAttrDefault {
    #[serde(default)]
    pub default_: FieldAttrValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldAttrDefaultPath {
    #[serde(default = "FieldAttrValue::value")]
    pub default_: FieldAttrValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldAttrWrap {
    #[serde(flatten)]
    pub inner: FieldAttr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldAttrBorrow<'a> {
    #[serde(borrow)]
    pub borrowed: Cow<'a, FieldAttrValue>,

    pub owned: Cow<'a, FieldAttrValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldAttrBound<T>
where
    T: Debug,
{
    #[serde(bound(serialize = "T: Debug + Display"))]
    #[serde(bound(deserialize = "T: Debug + FromStr, T::Err: Display"))]
    #[serde(serialize_with = "ser_bound")]
    #[serde(deserialize_with = "de_bound")]
    pub inner: T,
}

fn de_path<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    struct PathBufVisitor;

    impl<'de> de::Visitor<'de> for PathBufVisitor {
        type Value = Option<PathBuf>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing a path")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let abs = PathBuf::from(v);
            let filename = abs.file_name().ok_or_else(|| E::custom("invalid path"))?;
            Ok(Some(PathBuf::from(filename)))
        }
    }

    deserializer.deserialize_any(PathBufVisitor)
}

fn ser_path<S>(path: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let abs = path.as_ref().and_then(|p| absolute(p).ok());
    let abs_str = abs
        .as_ref()
        .and_then(|p| p.as_os_str().to_str())
        .ok_or_else(|| serde::ser::Error::custom("invalid path"))?;
    serializer.serialize_str(abs_str)
}

fn de_bound<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Debug + FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

fn ser_bound<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Debug + Display,
{
    serializer.serialize_str(&value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_field_attr_rename() {
        let json = r#"{"renamed":"value"}"#;
        let field_attr: FieldAttr = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.rename, Some("value".to_string()));

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, json);
    }

    #[test]
    fn test_field_attr_alias() {
        let json = r#"{"aliased":"value"}"#;
        let field_attr: FieldAttr = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.alias, Some("value".to_string()));

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, r#"{"alias":"value"}"#);
    }

    #[test]
    fn test_field_attr_default() {
        let json = r#"{}"#;
        let field_attr: FieldAttrDefault = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.default_, FieldAttrValue::B);

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, r#"{"default_":"B"}"#);
    }

    #[test]
    fn test_field_attr_default_path() {
        let json = r#"{}"#;
        let field_attr: FieldAttrDefaultPath = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.default_, FieldAttrValue::B);

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, r#"{"default_":"B"}"#);
    }

    #[test]
    fn test_field_attr_flatten() {
        let json = r#"{"renamed":"value"}"#;
        let field_attr: FieldAttrWrap = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.inner.rename, Some("value".to_string()));

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, json);
    }

    #[test]
    fn test_field_attr_serde_with() {
        let json = r#"{"path":"/path/to/../file.txt"}"#;
        let field_attr: FieldAttr = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.path, Some(PathBuf::from("file.txt")));

        let abs_path = absolute(Path::new("file.txt")).unwrap();
        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, format!(r#"{{"path":"{}"}}"#, abs_path.display()));
    }

    #[test]
    fn test_field_attr_borrow() {
        let json = r#"{"borrowed":"A","owned":"B"}"#;
        let field_attr: FieldAttrBorrow = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.borrowed, Cow::Borrowed(&FieldAttrValue::A));
        assert_eq!(field_attr.owned, Cow::Owned(FieldAttrValue::B));

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, json);
    }

    #[test]
    fn test_field_attr_bound() {
        let json = r#"{"inner":"A"}"#;
        let field_attr: FieldAttrBound<FieldAttrValue> = serde_json::from_str(json).unwrap();
        assert_eq!(field_attr.inner, FieldAttrValue::A);

        let result = serde_json::to_string(&field_attr).unwrap();
        assert_eq!(result, json);
    }
}
