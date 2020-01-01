use crate::datetime;
use core::fmt::Write;

use embedded_graphics::coord::Coord;
use embedded_graphics::fonts::Font8x16;
use embedded_graphics::prelude::*;
use epd_waveshare::epd2in9::Display2in9;
use epd_waveshare::graphics::Display;
use epd_waveshare::prelude::{Color, DisplayRotation};
use heapless::{consts::*, String, Vec};

mod header;
mod statemanager;
mod seven_segments;
mod menu;

#[derive(Debug)]
pub enum Msg {
    DateTime(datetime::DateTime),
    ButtonCancel,
    ButtonMinus,
    ButtonPlus,
    ButtonOk,
}
impl Msg {
    fn is_button(&self) -> bool {
        use self::Msg::*;
        match self {
            ButtonCancel | ButtonMinus | ButtonPlus | ButtonOk => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum Cmd {
    UpdateRtc(datetime::DateTime),
    FullUpdate,
}

#[derive(Clone)]
pub struct Model {
    new_time: datetime::DateTime,
	last_time: u32,
	statemanager: statemanager::StateManager,
}

impl Model {
    pub fn init() -> Self {
        Self {
            new_time: datetime::DateTime::new(0),
			last_time: 0,
			statemanager: statemanager::StateManager::Clock,
        }
    }
    pub fn update(&mut self, msg: Msg) -> Vec<Cmd, U4> {
        use self::statemanager::StateManager::*;
        let mut cmds = Vec::new();

        if msg.is_button() {
            self.update_last_input();
        }

        match msg {
            Msg::DateTime(dt) => {
                self.new_time = dt;
                if self.statemanager != statemanager::StateManager::Clock
                    && self
                        .new_time
                        .to_epoch()
                        .map_or(false, |n| n - self.last_time > 10 * 60)
                {
                    self.statemanager = statemanager::StateManager::Clock;
                }
                if self.new_time.hour == 0 && self.new_time.min == 0 && self.new_time.sec == 0 {
                    cmds.push(Cmd::FullUpdate).unwrap();
                }
            }
            Msg::ButtonOk => {
                use self::statemanager::{EditDateTime, MenuElt};
                self.statemanager = match ::core::mem::replace(&mut self.statemanager, Clock) {
                    Clock => Menu(MenuElt::Clock),
                    Menu(MenuElt::Clock) => Clock,
                    Menu(MenuElt::SetClock) => {
                        let mut dt = self.new_time.clone();
                        dt.sec = 0;
                        SetClock(EditDateTime::new(dt))
					}
                    SetClock(mut edit) => {
                        if let Some(dt) = edit.ok() {
                            cmds.push(Cmd::UpdateRtc(dt)).unwrap();
                            Clock
                        } else {
                            SetClock(edit)
                        }
                    }
                };
                if let Clock = self.statemanager {
                    cmds.push(Cmd::FullUpdate).unwrap();
                }
            }
            Msg::ButtonCancel => {
                self.statemanager = match ::core::mem::replace(&mut self.statemanager, Clock) {
                    Clock => Clock,
                    Menu(mut state) => state.cancel(),
                    SetClock(mut state) => state.cancel(),
                };
                if let Clock = self.statemanager {
                    cmds.push(Cmd::FullUpdate).unwrap();
                }
            }
            Msg::ButtonPlus => match &mut self.statemanager {
                Clock => {}
                Menu(state) => state.next(),
                SetClock(state) => state.next(),
            },
            Msg::ButtonMinus => match &mut self.statemanager {
                Clock => {}
                Menu(state) => state.prev(),
                SetClock(state) => state.prev(),
            },
        }
        cmds
    }
    pub fn view(&self) -> Display2in9 {
        let mut display = Display2in9::default();
        display.set_rotation(DisplayRotation::Rotate270);

        self.render_header(&mut display);

        use self::statemanager::StateManager::*;
        match &self.statemanager {
            Clock => self.render_clock(&mut display),
            Menu(elt) => self.render_menu(*elt, &mut display),
            SetClock(datetime) => self.render_set_clock(datetime, &mut display),
        }
        display
    }
    fn update_last_input(&mut self) {
        if let Some(epoch) = self.new_time.to_epoch() {
            self.last_time = epoch;
        }
    }
    fn render_header(&self, display: &mut Display2in9) {
        let mut header = header::Header::new(display);
        let mut s: String<U128> = String::new();

        write!(
            s,
            "{:4}-{:02}-{:02} {}",
            self.new_time.year, self.new_time.month, self.new_time.day, self.new_time.day_of_week,
        )
        .unwrap();
        header.top_left(&s);
    }
    fn render_clock(&self, display: &mut Display2in9) {
        let mut seven = seven_segments::SevenSegments::new(display, 0, 18);

        if self.new_time.hour >= 10 {
            seven.digit(self.new_time.hour / 10);
        } else {
            seven.digit_space();
        }
        seven.digit(self.new_time.hour % 10);
        if self.new_time.sec % 2 == 0 {
            seven.colon();
        } else {
            seven.colon_space();
        }
        seven.digit(self.new_time.min / 10);
        seven.digit(self.new_time.min % 10);

        let display = seven.into_display();
        let mut s: String<U4> = String::new();
        write!(s, ":{:02}", self.new_time.sec).unwrap();
        display.draw(
            Font8x16::render_str(&s)
                .with_stroke(Some(Color::Black))
                .with_fill(Some(Color::White))
                .translate(Coord::new(296 - 3 * 8, 17))
                .into_iter(),
        );
    }
    fn render_menu(&self, elt: statemanager::MenuElt, display: &mut Display2in9) {
        menu::render("Menu:", elt.items(), elt as i32, display);
    }
    fn render_set_clock(&self, dt: &statemanager::EditDateTime, display: &mut Display2in9) {
        let mut title: String<U128> = String::new();
        write!(
            title,
            "Edit: {:04}-{:02}-{:02} {:02}:{:02}",
            dt.datetime.year, dt.datetime.month, dt.datetime.day, dt.datetime.hour, dt.datetime.min
        )
        .unwrap();
        menu::render(&title, &[dt.as_edit_str()], 0, display);
    }
}
