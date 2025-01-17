use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    const FORBIDDEN_CHARACTERS: &[char] = &[
        '/', '\\', '<', '>', '"', '|', '?', '*', ':', ';', ',', '(', ')',
    ];
    /// # Panics
    /// If the provided name is empty, longer than 256 characters, or contains any of the following characters: `/`, `\\`, `<`, `>`, `"`, `|`, `?`, `*`, `:`, `;`, `,`, `(`, `)`.
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;

        let has_forbidden_characters = s.chars().any(|ch| Self::FORBIDDEN_CHARACTERS.contains(&ch));

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

#[cfg(test)]
mod tests {
    use super::*;
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_graphme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_name_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in SubscriberName::FORBIDDEN_CHARACTERS {
            assert_err!(SubscriberName::parse(name.to_string()));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
