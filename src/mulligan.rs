use crate::{Card, WinCondition, count_tag};

/// Chooses whether to keep a provisional opening hand.
pub trait MulliganPolicy: Send + Sync {
    fn keep(&self, opening_hand: &[Card]) -> bool;
}

/// Adapts a closure into a mulligan policy.
pub struct KeepIf<F> {
    predicate: F,
}

impl<F> KeepIf<F> {
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<F> MulliganPolicy for KeepIf<F>
where
    F: Fn(&[Card]) -> bool + Send + Sync,
{
    fn keep(&self, opening_hand: &[Card]) -> bool {
        (self.predicate)(opening_hand)
    }
}

/// Keeps hands whose land count is inside an inclusive range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeepIfLandsBetween {
    pub min: usize,
    pub max: usize,
}

impl KeepIfLandsBetween {
    pub fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

impl MulliganPolicy for KeepIfLandsBetween {
    fn keep(&self, opening_hand: &[Card]) -> bool {
        let lands = count_tag(opening_hand, "land");
        (self.min..=self.max).contains(&lands)
    }
}

/// Keeps an immediately winning hand or one inside a land-count window.
pub struct KeepIfWinOrDecent<'a> {
    pub win: &'a dyn WinCondition,
    pub min_lands: usize,
    pub max_lands: usize,
}

impl<'a> KeepIfWinOrDecent<'a> {
    pub fn new(win: &'a dyn WinCondition, min_lands: usize, max_lands: usize) -> Self {
        Self {
            win,
            min_lands,
            max_lands,
        }
    }
}

impl MulliganPolicy for KeepIfWinOrDecent<'_> {
    fn keep(&self, opening_hand: &[Card]) -> bool {
        if self.win.satisfied(opening_hand) {
            return true;
        }

        let lands = count_tag(opening_hand, "land");
        (self.min_lands..=self.max_lands).contains(&lands)
    }
}

/// Selects hand indices to put on the bottom after a London mulligan.
pub trait BottomHeuristic: Send + Sync {
    fn cards_to_bottom(&self, hand: &[Card], count: usize, win: &dyn WinCondition) -> Vec<usize>;
}

/// Keeps win-condition pieces first, then lands, tutors, and draw spells.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultBottomHeuristic;

impl BottomHeuristic for DefaultBottomHeuristic {
    fn cards_to_bottom(&self, hand: &[Card], count: usize, win: &dyn WinCondition) -> Vec<usize> {
        let mut scored: Vec<_> = hand
            .iter()
            .enumerate()
            .map(|(index, card)| {
                let support_priority = if card.has_tag("land") {
                    80
                } else if card.has_tag("tutor") || card.has_tag("draw") {
                    70
                } else {
                    0
                };
                (index, win.card_priority(card).max(support_priority))
            })
            .collect();

        scored.sort_by_key(|&(index, priority)| (priority, index));
        scored
            .into_iter()
            .take(count.min(hand.len()))
            .map(|(index, _)| index)
            .collect()
    }
}
