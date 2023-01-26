use serde::{de, Deserialize};
use validator::validate_email;

#[derive(Clone, Debug)]
pub struct SubscriberEmail(String);

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for SubscriberEmail {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if validate_email(&value) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid email", value))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SubscriberEmail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::try_from(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use fake::{faker::internet::en::SafeEmail, Fake};

    #[derive(Clone, Debug)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::try_from(valid_email.0).is_ok()
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert!(SubscriberEmail::try_from(email).is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "exampledomain.com".to_string();
        assert!(SubscriberEmail::try_from(email).is_err());
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert!(SubscriberEmail::try_from(email).is_err());
    }
}
