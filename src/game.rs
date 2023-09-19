use crate::{
    card::{Card, PlacedCard, Placement, Suit},
    Position, Style,
};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::io::Write;
use termion::{clear, color, cursor::Goto};

pub struct Game {
    pub deck: Vec<Card>,
    pub pick: Vec<Card>,
    pub end_piles: Vec<Vec<Card>>,
    pub game_piles: Vec<Vec<PlacedCard>>,
}

impl Game {
    pub fn new() -> Game {
        let mut deck: Vec<Card> = [Suit::Diamonds, Suit::Hearts, Suit::Clubs, Suit::Spades]
            .iter()
            .flat_map(|suit| (0..13).map(|value| Card(value, suit.clone())))
            .collect();
        let mut rng = StdRng::from_entropy();
        deck.shuffle(&mut rng);

        let game_piles: Vec<Vec<PlacedCard>> = (0..7)
            .map(|seed| {
                (0..seed + 1)
                    .map(|pos| PlacedCard {
                        card: deck.pop().unwrap(),
                        visible: pos == seed,
                    })
                    .collect()
            })
            .collect();
        let end_piles: Vec<Vec<Card>> = (0..4).map(|_| Vec::new()).collect();
        let pick: Vec<Card> = Vec::new();

        Game {
            deck,
            pick,
            end_piles,
            game_piles,
        }
    }

    pub fn print_selection<W>(&self, w: W, pos: &Position, style: Style)
    where
        W: Write,
    {
        self.print_pile(w, pos, !matches!(pos, Position::Deck), style);
    }

    pub fn print_pile<W>(&self, mut w: W, pos: &Position, display: bool, style: Style)
    where
        W: Write,
    {
        let (x, y) = self.as_screen_coords(pos);
        write!(w, "{}", Goto(x, y)).unwrap();

        if let Some(card) = self.card_at(pos) {
            if display {
                write!(
                    w,
                    "{}{}",
                    color::Bg(style.to_color().unwrap_or(&color::LightWhite)),
                    card
                )
                .unwrap();
            } else {
                write!(
                    w,
                    "{}{} XXX {}",
                    color::Bg(style.to_color().unwrap_or(&color::LightBlue)),
                    color::Fg(color::Blue),
                    color::Fg(color::Reset)
                )
                .unwrap();
            }
        } else {
            write!(
                w,
                "{}{} ( ) {}",
                color::Bg(style.to_color().unwrap_or(&color::Green)),
                color::Fg(color::LightGreen),
                color::Fg(color::Reset)
            )
            .unwrap();
        }

        write!(w, "{}", color::Bg(color::Reset)).unwrap();
    }

    pub fn as_screen_coords(&self, pos: &Position) -> (u16, u16) {
        match pos {
            Position::Deck => (1, 1),
            Position::Pick => (7, 1),
            Position::EndPile(n) => (1 + (*n as u16 + 3) * 6, 1),
            Position::GamePile(row, col) => {
                let pile = &self.game_piles[*row as usize];
                if pile.is_empty() {
                    (1 + (*row as u16) * 6, 3)
                } else {
                    (
                        1 + (*row as u16) * 6,
                        2 + (pile.len() as u16) - (*col as u16),
                    )
                }
            }
        }
    }

    pub fn card_at(&self, pos: &Position) -> Option<&Card> {
        match pos {
            Position::Deck => self.deck.last(),
            Position::Pick => self.pick.last(),
            Position::EndPile(n) => self.end_piles[*n as usize].last(),
            Position::GamePile(pile, card) => {
                let pile_ref = &self.game_piles[*pile as usize];
                if pile_ref.is_empty() {
                    None
                } else {
                    let PlacedCard { card, .. } = &pile_ref[pile_ref.len() - 1 - (*card as usize)];
                    Some(card)
                }
            }
        }
    }

    pub fn pop_card_at(&mut self, pos: &Position) -> Option<Card> {
        match pos {
            Position::Deck => self.deck.pop(),
            Position::Pick => self.pick.pop(),
            Position::EndPile(n) => self.end_piles[*n as usize].pop(),
            Position::GamePile(pile, card) => {
                assert_eq!(0, *card);
                self.game_piles[*pile as usize].pop().map(|c| c.card)
            }
        }
    }

    pub fn will_move_multiple_cards(&self, pos: &Position) -> bool {
        match pos {
            Position::Deck | Position::Pick | Position::EndPile(_) => false,
            Position::GamePile(_, idx) => *idx > 0,
        }
    }

    pub fn place_card_at(&mut self, pos: &Position, card: Card) {
        match pos {
            Position::Deck | Position::Pick => panic!(),
            Position::EndPile(n) => self.end_piles[*n as usize].push(card),
            Position::GamePile(pile, idx) => {
                assert_eq!(0, *idx);
                self.game_piles[*pile as usize].push(PlacedCard {
                    card,
                    visible: true,
                })
            }
        }
    }

    pub fn move_cards<W>(&mut self, mut w: W, from: &Position, to: &Position) -> bool
    where
        W: Write,
    {
        if from == to {
            return true;
        }

        if matches!((from, to), (Position::GamePile(a, _), Position::GamePile(b, _)) if *a == *b) {
            return true;
        }

        let card = self.card_at(from).unwrap();

        match to {
            Position::Deck => panic!(),
            Position::Pick => false,
            Position::EndPile(n) if !self.will_move_multiple_cards(from) => {
                let to_card = self.end_piles[*n as usize].last();

                if card.0 != 0 && to_card.is_none() {
                    return false;
                }

                if let Some(to_card) = to_card {
                    if !card.placeable_on(to_card, Placement::EndPile) {
                        return false;
                    }
                }

                if !matches!(from, Position::Pick) {
                    let (x, y) = self.as_screen_coords(from);
                    println!("{}     ", Goto(x, y));
                }

                let card = self.pop_card_at(from).unwrap();
                self.place_card_at(to, card);

                if let Position::GamePile(pile, _) = from {
                    if let Some(card) = self.game_piles[*pile as usize].last_mut() {
                        card.visible = true;
                    }
                }

                self.print_pile(w, from, true, Style::Normal);
                true
            }
            Position::GamePile(to_pile, _) => {
                let from_card = self.card_at(from).unwrap();
                let to_card = self.card_at(to);

                if let Some(to_card) = to_card {
                    if !from_card.placeable_on(to_card, Placement::GamePile) {
                        return false;
                    }
                } else {
                    if to_card.is_none() && from_card.0 != 12 {
                        return false;
                    }
                }

                match from {
                    Position::Deck => panic!(),
                    Position::Pick | Position::EndPile(_) => {
                        let card = self.pop_card_at(from).unwrap();
                        self.place_card_at(to, card);

                        self.print_pile(w, from, true, Style::Normal);
                        true
                    }
                    Position::GamePile(from_pile, idx) => {
                        let (x, y) = self.as_screen_coords(from);
                        let pile_idx =
                            self.game_piles[*from_pile as usize].len() - 1 - *idx as usize;

                        for i in 0..(*idx + 1) {
                            write!(w, "{}     ", Goto(x, y + i as u16)).unwrap();

                            let from_pile_ref = &mut self.game_piles[*from_pile as usize];
                            let PlacedCard { card, .. } = from_pile_ref.remove(pile_idx);

                            let to_pile_ref = &mut self.game_piles[*to_pile as usize];
                            to_pile_ref.push(PlacedCard {
                                card,
                                visible: true,
                            });
                            self.print_pile(
                                &mut w,
                                &Position::GamePile(*to_pile, 0),
                                true,
                                Style::Normal,
                            );
                        }

                        self.game_piles[*from_pile as usize]
                            .last_mut()
                            .and_then(|card| {
                                card.visible = true;
                                Some(())
                            });
                        self.print_pile(w, &Position::GamePile(*from_pile, 0), true, Style::Normal);

                        true
                    }
                }
            }
            _ => false,
        }
    }

    pub fn redraw<W>(&self, mut w: W)
    where
        W: Write,
    {
        write!(w, "{}", clear::All).unwrap();

        self.print_pile(&mut w, &Position::Deck, false, Style::Normal);
        self.print_pile(&mut w, &Position::Pick, true, Style::Normal);

        for (x, _) in self.end_piles.iter().enumerate() {
            self.print_pile(&mut w, &Position::EndPile(x as u8), true, Style::Normal);
        }

        for (x, pile) in self.game_piles.iter().enumerate() {
            if pile.is_empty() {
                self.print_pile(&mut w, &Position::GamePile(x as u8, 0), true, Style::Normal);
                continue;
            }

            for (y, card) in pile.iter().rev().enumerate() {
                self.print_pile(
                    &mut w,
                    &Position::GamePile(x as u8, y as u8),
                    card.visible,
                    Style::Normal,
                );
            }
        }

        w.flush().unwrap();
    }
}
