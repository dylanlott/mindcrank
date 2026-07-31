use std::collections::HashSet;

/// A lightweight card model driven by free-form tags.
///
/// Typical tags include `land`, `tutor`, `draw`, `combo:oracle`, and
/// `combo:consult`.
/// 
/// Tags are intended to be flexible descriptors that can capture various 
/// aspects of a card's role or function within a deck.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Card {
    pub name: String,
    pub card_type: Option<String>,
    pub tags: HashSet<String>,
}

impl Card {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            card_type: None,
            tags: HashSet::new(),
        }
    }

    pub fn with_type(mut self, card_type: impl Into<String>) -> Self {
        self.card_type = Some(card_type.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }
}
