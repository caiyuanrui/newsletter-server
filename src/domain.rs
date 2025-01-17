use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

pub struct SubscriberName(String);

impl SubscriberName {
    /// # Panics
    /// If the provided name is empty, longer than 256 characters, or contains any of the following characters: `/`, `\\`, `<`, `>`, `"`, `|`, `?`, `*`, `:`, `;`, `,`, `(`, `)`.
    pub fn parse(s: String) -> Self {
        match Self::try_parse(s) {
            Ok(s) => s,
            Err(e) => panic!("Invalid subscriber name: {e}"),
        }
    }

    pub fn try_parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = [
            '/', '\\', '<', '>', '"', '|', '?', '*', ':', ';', ',', '(', ')',
        ];
        let has_forbidden_characters = s.chars().any(|ch| forbidden_characters.contains(&ch));

        if is_empty_or_whitespace {
            Err("Name is empty.".to_string())
        } else if is_too_long {
            Err("Name is too long.".to_string())
        } else if has_forbidden_characters {
            Err("Name contains forbidden characters.".to_string())
        } else {
            Ok(Self(s))
        }
    }

    pub fn inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
