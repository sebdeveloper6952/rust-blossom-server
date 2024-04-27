use crate::blossom::action::Action;
use nostr::event::Event;
use nostr::{Alphabet, Kind, SingleLetterTag, TagKind, Timestamp};
use std::str::FromStr;
use tracing::instrument;

/// logic to actually validate if an event is a valid blossom authentication event
#[instrument(skip(event, action))]
pub fn is_auth_event_valid(
    event: &Event,
    action: Action,
    payload_size: usize,
) -> Result<(), String> {
    if let Err(_) = event.verify() {
        return Err("event signature verification failed".into());
    }

    if event.kind() != Kind::Custom(24242) {
        return Err("kind must be 24242".into());
    }

    if event.created_at() > Timestamp::now() {
        return Err("created_at must be in the past".into());
    }

    match event.tags.iter().find(|t| {
        t.kind()
            == TagKind::SingleLetter(SingleLetterTag {
                character: Alphabet::T,
                uppercase: false,
            })
    }) {
        Some(tag) => {
            if let Some(tag_value) = tag.content() {
                match Action::from_str(&tag_value.to_string()) {
                    Ok(tag_action) => {
                        if tag_action != action {
                            return Err("action doesn't match".into());
                        }
                    }
                    _ => return Err("invalid action".into()),
                }
            }
        }
        _ => {
            return Err("t tag must be set".into());
        }
    }

    match event.tags.iter().find(|t| t.kind() == TagKind::Expiration) {
        Some(tag) => {
            if let Some(tag_value) = tag.content() {
                match Timestamp::from_str(&tag_value.to_string()) {
                    Ok(exp) => {
                        if exp < Timestamp::now() {
                            return Err("expiration must be in the future".into());
                        }
                    }
                    _ => return Err("invalid expiration".into()),
                }
            }
        }
        _ => {
            return Err("expiration tag must be set".into());
        }
    }

    if action == Action::Upload {
        match event.tags.iter().find(|t| t.kind() == TagKind::Size) {
            Some(tag) => {
                if let Some(tag_value) = tag.content() {
                    match tag_value.to_string().parse::<usize>() {
                        Ok(tag_size_value) => {
                            if tag_size_value != payload_size {
                                println!("expected: {}, found: {}", payload_size, tag_size_value);
                                return Err("payload size does not match size tag".into());
                            }
                        }
                        _ => return Err("invalid size".into()),
                    }
                }
            }
            _ => {
                return Err("size tag must be set".into());
            }
        }
    }

    if action == Action::Delete {
        match event.tags.iter().find(|t| {
            t.kind()
                == TagKind::SingleLetter(SingleLetterTag {
                    character: Alphabet::X,
                    uppercase: false,
                })
        }) {
            None => {
                return Err("x tag must be set".into());
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_auth_event_valid;
    use crate::blossom::action::Action;
    use nostr::prelude::*;
    use nostr_sdk::prelude::*;
    use std::time::Duration;

    #[test]
    fn valid_event_passes_through() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("upload".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_ok());
    }

    #[test]
    fn different_kind_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(69420),
            "auth event",
            vec![
                Tag::Hashtag("get".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }
    #[test]
    fn different_action_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("get".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }

    #[test]
    fn different_size_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("upload".into()),
                Tag::Size(36193),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }

    #[test]
    fn expiration_in_the_past_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("upload".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() - Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }
}
