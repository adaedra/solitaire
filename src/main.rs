mod card;
mod game;

use crate::game::Game;
use std::io::{self, Write};
use termion::{color, cursor::Goto, event::Key, input::TermRead, raw::IntoRawMode};

pub enum Style {
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

#[derive(PartialEq, Clone)]
pub enum Position {
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

    let mut game = Game::new();
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
                Key::Char('n') => {
                    game = Game::new();
                    game.redraw(&mut stdout);

                    cur = Position::Deck;
                    sel = None;
                }
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
