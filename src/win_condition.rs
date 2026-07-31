use crate::{Card, count_tag};

/// Decides whether the current hand satisfies a victory precondition.
pub trait WinCondition: Send + Sync {
    fn satisfied(&self, hand: &[Card]) -> bool;

    /// Gives the default mulligan heuristic a hint about cards relevant to
    /// this condition. Higher values are more valuable to keep.
    fn card_priority(&self, _card: &Card) -> i32 {
        0
    }
}

/// Requires at least one card from group A and one card from group B.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoCardSet {
    pub group_a: String,
    pub group_b: String,
}

impl TwoCardSet {
    pub fn new(group_a: impl Into<String>, group_b: impl Into<String>) -> Self {
        Self {
            group_a: group_a.into(),
            group_b: group_b.into(),
        }
    }
}

impl WinCondition for TwoCardSet {
    fn satisfied(&self, hand: &[Card]) -> bool {
        count_tag(hand, &self.group_a) > 0 && count_tag(hand, &self.group_b) > 0
    }

    fn card_priority(&self, card: &Card) -> i32 {
        if card.has_tag(&self.group_a) || card.has_tag(&self.group_b) {
            100
        } else {
            0
        }
    }
}

/// Requires at least `k` cards carrying a tag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KOfTag {
    pub tag: String,
    pub k: usize,
}

impl KOfTag {
    pub fn new(tag: impl Into<String>, k: usize) -> Self {
        Self { tag: tag.into(), k }
    }
}

impl WinCondition for KOfTag {
    fn satisfied(&self, hand: &[Card]) -> bool {
        count_tag(hand, &self.tag) >= self.k
    }

    fn card_priority(&self, card: &Card) -> i32 {
        if card.has_tag(&self.tag) { 100 } else { 0 }
    }
}

/// Accepts a hand when any child condition is satisfied.
#[derive(Default)]
pub struct AnyOf {
    pub conditions: Vec<Box<dyn WinCondition>>,
}

impl AnyOf {
    pub fn new(conditions: Vec<Box<dyn WinCondition>>) -> Self {
        Self { conditions }
    }

    pub fn push(&mut self, condition: impl WinCondition + 'static) {
        self.conditions.push(Box::new(condition));
    }
}

impl WinCondition for AnyOf {
    fn satisfied(&self, hand: &[Card]) -> bool {
        self.conditions
            .iter()
            .any(|condition| condition.satisfied(hand))
    }

    fn card_priority(&self, card: &Card) -> i32 {
        self.conditions
            .iter()
            .map(|condition| condition.card_priority(card))
            .max()
            .unwrap_or(0)
    }
}
