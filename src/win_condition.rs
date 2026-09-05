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

/// One required slot in a combo route.
///
/// `role` is the tag carried by the natural combo piece. `tutor_kind` is the
/// tag carried by tutors that may find that piece, such as `tutor:creature`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Piece {
    pub role: String,
    pub tutor_kind: String,
}

impl Piece {
    pub fn new(role: impl Into<String>, tutor_kind: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            tutor_kind: tutor_kind.into(),
        }
    }

    fn naturally_matches(&self, card: &Card) -> bool {
        card.has_tag(&self.role)
    }

    fn access_matches(&self, card: &Card) -> bool {
        self.naturally_matches(card)
            || (card.has_tag("tutor")
                && (card.has_tag("tutor:any") || card.has_tag(&self.tutor_kind)))
    }
}

/// A named set of distinct pieces that form one combo route.
///
/// Natural and tutor-aware matching both consume each physical card at most
/// once. A route with two pieces bearing the same role therefore needs two
/// matching cards, and one universal tutor cannot cover two missing pieces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    pub name: String,
    pub pieces: Vec<Piece>,
}

impl Route {
    pub fn new(name: impl Into<String>, pieces: impl IntoIterator<Item = Piece>) -> Self {
        Self {
            name: name.into(),
            pieces: pieces.into_iter().collect(),
        }
    }

    /// Whether distinct cards in `hand` naturally fill every piece.
    pub fn naturally_satisfied(&self, hand: &[Card]) -> bool {
        match_route(&self.pieces, hand, Piece::naturally_matches)
    }

    /// Whether distinct natural pieces or eligible tutors fill every piece.
    ///
    /// Tutors must carry the general `tutor` tag plus either `tutor:any` or the
    /// piece's `tutor_kind` tag.
    pub fn accessible(&self, hand: &[Card]) -> bool {
        match_route(&self.pieces, hand, Piece::access_matches)
    }
}

impl WinCondition for Route {
    fn satisfied(&self, hand: &[Card]) -> bool {
        self.naturally_satisfied(hand)
    }

    fn card_priority(&self, card: &Card) -> i32 {
        if self
            .pieces
            .iter()
            .any(|piece| piece.naturally_matches(card))
        {
            100
        } else {
            0
        }
    }
}

/// Accepts a hand when any route is naturally complete or tutor-accessible.
///
/// ```
/// use mindcrank::{Card, Piece, Route, TutorAwareWin, WinCondition};
///
/// let win = TutorAwareWin::new([Route::new(
///     "Oracle consultation",
///     [
///         Piece::new("combo:oracle", "tutor:creature"),
///         Piece::new("combo:consult", "tutor:instant"),
///     ],
/// )]);
/// let hand = vec![
///     Card::new("Thassa's Oracle").with_tag("combo:oracle"),
///     Card::new("Demonic Tutor").with_tags(["tutor", "tutor:any"]),
/// ];
///
/// assert!(win.satisfied(&hand));
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TutorAwareWin {
    pub routes: Vec<Route>,
}

impl TutorAwareWin {
    pub fn new(routes: impl IntoIterator<Item = Route>) -> Self {
        Self {
            routes: routes.into_iter().collect(),
        }
    }

    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    pub fn accessible_route(&self, hand: &[Card], route: &Route) -> bool {
        route.accessible(hand)
    }
}

impl WinCondition for TutorAwareWin {
    fn satisfied(&self, hand: &[Card]) -> bool {
        self.routes.iter().any(|route| route.accessible(hand))
    }

    fn card_priority(&self, card: &Card) -> i32 {
        if self
            .routes
            .iter()
            .flat_map(|route| &route.pieces)
            .any(|piece| piece.naturally_matches(card))
        {
            100
        } else if card.has_tag("tutor") {
            95
        } else {
            0
        }
    }
}

fn match_route(pieces: &[Piece], hand: &[Card], matches: fn(&Piece, &Card) -> bool) -> bool {
    if pieces.is_empty() {
        return false;
    }

    let mut used = vec![false; hand.len()];
    match_piece(pieces, hand, &mut used, 0, matches)
}

fn match_piece(
    pieces: &[Piece],
    hand: &[Card],
    used: &mut [bool],
    piece_index: usize,
    matches: fn(&Piece, &Card) -> bool,
) -> bool {
    if piece_index == pieces.len() {
        return true;
    }

    for (card_index, card) in hand.iter().enumerate() {
        if !used[card_index] && matches(&pieces[piece_index], card) {
            used[card_index] = true;
            if match_piece(pieces, hand, used, piece_index + 1, matches) {
                return true;
            }
            used[card_index] = false;
        }
    }

    false
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

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str, tags: &[&str]) -> Card {
        Card::new(name).with_tags(tags.iter().copied())
    }

    fn route() -> Route {
        Route::new(
            "Oracle consultation",
            [
                Piece::new("combo:oracle", "tutor:creature"),
                Piece::new("combo:consult", "tutor:instant"),
            ],
        )
    }

    #[test]
    fn route_matches_natural_pieces() {
        let route = route();
        let hand = [
            card("Oracle", &["combo:oracle"]),
            card("Consultation", &["combo:consult"]),
        ];

        assert!(route.naturally_satisfied(&hand));
        assert!(route.accessible(&hand));
    }

    #[test]
    fn typed_tutors_only_cover_eligible_pieces() {
        let route = route();
        let wrong_tutor = [
            card("Oracle", &["combo:oracle"]),
            card("Creature tutor", &["tutor", "tutor:creature"]),
        ];
        let right_tutor = [
            card("Oracle", &["combo:oracle"]),
            card("Universal tutor", &["tutor", "tutor:any"]),
        ];

        assert!(!route.accessible(&wrong_tutor));
        assert!(route.accessible(&right_tutor));
    }

    #[test]
    fn cards_and_tutors_are_each_consumed_once() {
        let duplicate_role = Route::new(
            "Two artifacts",
            [
                Piece::new("combo:artifact", "tutor:artifact"),
                Piece::new("combo:artifact", "tutor:artifact"),
            ],
        );
        let one_piece = [card("Artifact", &["combo:artifact"])];
        let one_tutor = [card("Tutor", &["tutor", "tutor:any"])];
        let both = [
            card("Artifact", &["combo:artifact"]),
            card("Tutor", &["tutor", "tutor:artifact"]),
        ];

        assert!(!duplicate_role.naturally_satisfied(&one_piece));
        assert!(!duplicate_role.accessible(&one_tutor));
        assert!(duplicate_role.accessible(&both));
    }

    #[test]
    fn empty_routes_do_not_win() {
        let route = Route::new("Empty", []);
        assert!(!route.naturally_satisfied(&[]));
        assert!(!route.accessible(&[]));
        assert!(!TutorAwareWin::new([route]).satisfied(&[]));
    }
}
