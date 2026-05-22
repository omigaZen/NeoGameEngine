#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetPath {
    pub path: String,
    pub label: Option<String>,
}

impl AssetPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: normalize_path(path.into()),
            label: None,
        }
    }

    pub fn with_label(path: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            path: normalize_path(path.into()),
            label: Some(label.into()),
        }
    }

    pub fn parse(value: &str) -> Self {
        if let Some((path, label)) = value.split_once('#') {
            Self::with_label(path, label)
        } else {
            Self::new(value)
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn extension(&self) -> Option<&str> {
        self.path
            .rsplit_once('.')
            .map(|(_, extension)| extension)
            .filter(|extension| !extension.is_empty())
    }

    pub fn without_label(&self) -> Self {
        Self {
            path: self.path.clone(),
            label: None,
        }
    }

    pub fn display_string(&self) -> String {
        match &self.label {
            Some(label) => format!("{}#{label}", self.path),
            None => self.path.clone(),
        }
    }
}

impl From<&str> for AssetPath {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for AssetPath {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

fn normalize_path(value: String) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_owned()
}
