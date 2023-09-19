mod card;

use crate::card::{Card, PlacedCard, Placement, Suit};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::io::{self, Write};
use termion::{clear, color, cursor::Goto, event::Key, input::TermRead, raw::IntoRawMode};

enum Style {
    Normal,
    Cursor,
    Selection,
    CursorSelection,
}

impl Style {
    fn to_color(&self) -> Option<&dyn color::Color> {
        match self {
            Self::Normal => None,
            Self::Cursor => Some(&color::LightYellow),
            Self::Selection => Some(&color::Cyan),
            Self::CursorSelection => Some(&color::LightCyan),
        }
    }
}

struct Game {
    deck: Vec<Card>,
    pick: Vec<Card>,
    end_piles: Vec<Vec<Card>>,
    game_piles: Vec<Vec<PlacedCard>>,
}

impl Game {
    fn print_selection<W>(&self, w: W, pos: &Position, style: Style)
    where
        W: Write,
    {
        self.print_pile(w, pos, !matches!(pos, Position::Deck), style);
    }

    fn print_pile<W>(&self, mut w: W, pos: &Position, display: bool, style: Style)
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

    fn as_screen_coords(&self, pos: &Position) -> (u16, u16) {
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

    fn card_at(&self, pos: &Position) -> Option<&Card> {
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

    fn pop_card_at(&mut self, pos: &Position) -> Option<Card> {
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

    fn will_move_multiple_cards(&self, pos: &Position) -> bool {
        match pos {
            Position::Deck | Position::Pick | Position::EndPile(_) => false,
            Position::GamePile(_, idx) => *idx > 0,
        }
    }

    fn place_card_at(&mut self, pos: &Position, card: Card) {
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

    fn move_cards<W>(&mut self, mut w: W, from: &Position, to: &Position) -> bool
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

    fn redraw<W>(&self, mut w: W)
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

#[derive(PartialEq, Clone)]
enum Position {
    Deck,
    Pick,
    EndPile(u8),
    GamePile(u8, u8),
}

impl Position {
    fn prev(&self) -> Position {
        match self {
            Position::Deck => panic!(),
            Position::Pick => Position::Deck,
            Position::EndPile(n) if *n == 0 => Position::Pick,
            Position::EndPile(n) => Position::EndPile(n - 1),
            Position::GamePile(n, _) if *n == 0 => Position::EndPile(3),
            Position::GamePile(n, _) => Position::GamePile(n - 1, 0),
        }
    }

    fn next(&self) -> Position {
        match self {
            Position::Deck => Position::Pick,
            Position::Pick => Position::EndPile(0),
            Position::EndPile(n) if *n < 3 => Position::EndPile(n + 1),
            Position::EndPile(_) => Position::GamePile(0, 0),
            Position::GamePile(n, _) => Position::GamePile(n + 1, 0),
        }
    }
}

fn main() {
    let mut stdout = io::stdout().into_raw_mode().unwrap();

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

    let mut game = Game {
        deck,
        pick,
        end_piles,
        game_piles,
    };

    game.redraw(&mut stdout);

    let mut stdin = io::stdin().keys();
    let mut cur = Position::Deck;
    let mut sel: Option<Position> = None;

    game.print_selection(&mut stdout, &cur, Style::Cursor);

    loop {
        write!(stdout, "{}", Goto(1, 2)).unwrap();
        stdout.flush().unwrap();

        if sel.as_ref().map(|sel| sel == &cur).unwrap_or(false) {
            game.print_selection(&mut stdout, &cur, Style::Selection);
        } else {
            game.print_selection(&mut stdout, &cur, Style::Normal);
        }

        if let Some(key) = stdin.next() {
            let key = key.unwrap();

            match key {
                Key::Left if cur != Position::Deck && !(cur == Position::Pick && sel.is_some()) => {
                    cur = cur.prev();
                }
                Key::Right if !matches!(cur, Position::GamePile(6, _)) => {
                    cur = cur.next();
                }
                Key::Up if sel.is_none() => match cur {
                    Position::GamePile(pile, ref mut card) => {
                        let pile = &game.game_piles[pile as usize];
                        if *card as usize + 1 < pile.len()
                            && pile[pile.len() - (*card as usize) - 2].visible
                        {
                            *card += 1;
                        }
                    }
                    _ => (),
                },
                Key::Down if sel.is_none() => match cur {
                    Position::GamePile(_, ref mut card) if *card > 0 => {
                        *card -= 1;
                    }
                    _ => (),
                },
                Key::Char('q') | Key::Esc => break,
                Key::Char(' ') | Key::Char('\n') if sel.is_none() => match &cur {
                    Position::Deck => {
                        if let Some(card) = game.deck.pop() {
                            game.pick.push(card);
                        } else {
                            game.pick.drain(..).rev().for_each(|c| game.deck.push(c));
                        }
                        game.print_selection(&mut stdout, &Position::Pick, Style::Normal);
                    }
                    Position::Pick => {
                        if !game.pick.is_empty() {
                            sel = Some(cur.clone());
                        }
                    }
                    Position::EndPile(n) => {
                        if !game.end_piles[*n as usize].is_empty() {
                            sel = Some(cur.clone());
                        }
                    }
                    Position::GamePile(pile, _) => {
                        if !game.game_piles[*pile as usize].is_empty() {
                            sel = Some(cur.clone());
                            cur = Position::GamePile(*pile, 0);
                        }
                    }
                },
                Key::Char(' ') | Key::Char('\n') if sel.is_some() => {
                    if game.move_cards(&mut stdout, sel.as_ref().unwrap(), &cur) {
                        sel = None;
                    }
                }
                Key::Char('r') => game.redraw(&mut stdout),
                _ => (),
            }
        } else {
            break;
        }

        if sel.is_some() {
            game.print_selection(&mut stdout, &cur, Style::CursorSelection);
        } else {
            game.print_selection(&mut stdout, &cur, Style::Cursor);
        }
    }

    write!(stdout, "{}", Goto(1, 2)).unwrap();
    stdout.flush().unwrap();
}
