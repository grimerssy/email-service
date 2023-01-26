use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub struct SubscriberName(String);

impl std::fmt::Display for SubscriberName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for SubscriberName {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        if value.trim().is_empty()
            || value.graphemes(true).count() > 256
            || value.chars().any(|c| forbidden_characters.contains(&c))
        {
            Err(format!("{} is not a valid username", value))
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberName;

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "John Doe".to_string();
        assert!(SubscriberName::try_from(name).is_ok());
    }

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert!(SubscriberName::try_from(name).is_ok());
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert!(SubscriberName::try_from(name).is_err());
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert!(SubscriberName::try_from(name).is_err());
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert!(SubscriberName::try_from(name).is_err());
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert!(SubscriberName::try_from(name).is_err());
        }
    }
}
