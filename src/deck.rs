use rand::Rng;
use rand::seq::SliceRandom;

use crate::Card;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    pub fn new(cards: Vec<Card>) -> Self {
        Self { cards }
    }

    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Returns a shuffled copy without changing the original deck list.
    pub fn shuffle<R>(&self, rng: &mut R) -> Self
    where
        R: Rng + ?Sized,
    {
        let mut cards = self.cards.clone();
        cards.shuffle(rng);
        Self { cards }
    }

    /// Returns up to `n` cards and a copy of the remaining deck.
    pub fn draw_n(&self, n: usize) -> (Vec<Card>, Self) {
        let split = n.min(self.cards.len());
        (
            self.cards[..split].to_vec(),
            Self::new(self.cards[split..].to_vec()),
        )
    }

    pub(crate) fn draw_n_mut(&mut self, n: usize) -> Vec<Card> {
        let split = n.min(self.cards.len());
        self.cards.drain(..split).collect()
    }

    pub(crate) fn put_on_bottom<I>(&mut self, cards: I)
    where
        I: IntoIterator<Item = Card>,
    {
        self.cards.extend(cards);
    }
}

pub fn count_tag(cards: &[Card], tag: &str) -> usize {
    cards.iter().filter(|card| card.has_tag(tag)).count()
}
