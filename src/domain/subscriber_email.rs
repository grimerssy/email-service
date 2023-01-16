use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

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

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};

    #[derive(Debug, Clone)]
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
        assert_err!(SubscriberEmail::try_from(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "exampledomain.com".to_string();
        assert_err!(SubscriberEmail::try_from(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::try_from(email));
    }
}
