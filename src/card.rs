use std::fmt;
use termion::color;

#[derive(PartialEq)]
enum SuitColor {
    Red,
    Black,
}

#[derive(PartialEq, Clone)]
pub enum Suit {
    Diamonds,
    Hearts,
    Clubs,
    Spades,
}

impl Suit {
    fn to_color(&self) -> SuitColor {
        match self {
            Suit::Diamonds | Suit::Hearts => SuitColor::Red,
            Suit::Clubs | Suit::Spades => SuitColor::Black,
        }
    }
}

pub enum Placement {
    EndPile,
    GamePile,
}

pub struct Card(pub u8, pub Suit);

impl Card {
    pub fn placeable_on(&self, other: &Card, placement: Placement) -> bool {
        match placement {
            Placement::EndPile => self.1 == other.1 && self.0 == other.0 + 1,
            Placement::GamePile => self.1.to_color() != other.1.to_color() && self.0 == other.0 - 1,
        }
    }

    pub fn value(&self) -> String {
        match self.0 {
            0 => " A".to_string(),
            v @ 1..=9 => format!("{:2}", v + 1),
            10 => " J".to_string(),
            11 => " Q".to_string(),
            12 => " K".to_string(),
            _ => panic!(),
        }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (fg, symbol): (&dyn color::Color, _) = match self.1 {
            Suit::Diamonds => (&color::Red, "♦"),
            Suit::Hearts => (&color::Red, "♥"),
            Suit::Clubs => (&color::Black, "♣"),
            Suit::Spades => (&color::Black, "♠"),
        };

        write!(
            f,
            "{} {}{} {}",
            color::Fg(fg),
            self.value(),
            symbol,
            color::Fg(color::Reset)
        )
    }
}

pub struct PlacedCard {
    pub card: Card,
    pub visible: bool,
}

#[cfg(test)]
mod tests {
    use super::{Card, Placement, Suit};

    #[test]
    fn test_placeable_on_valid() {
        assert!(Card(1, Suit::Hearts).placeable_on(&Card(2, Suit::Spades), Placement::GamePile));
    }

    #[test]
    fn test_placeable_on_same_suit() {
        assert!(!Card(1, Suit::Hearts).placeable_on(&Card(2, Suit::Hearts), Placement::GamePile));
    }

    #[test]
    fn test_placeable_on_same_color() {
        assert!(!Card(1, Suit::Hearts).placeable_on(&Card(2, Suit::Diamonds), Placement::GamePile));
    }

    #[test]
    fn test_placeable_on_wrong_value() {
        assert!(!Card(1, Suit::Hearts).placeable_on(&Card(4, Suit::Spades), Placement::GamePile));
    }
}
