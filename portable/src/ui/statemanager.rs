use crate::datetime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateManager {
    Clock,
    Menu(MenuElt),
    SetClock(EditDateTime),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MenuElt {
    Clock,
    SetClock,
}
impl MenuElt {
    pub fn next(&mut self) {
        use self::MenuElt::*;
        *self = match *self {
            Clock => SetClock,
            SetClock => Clock,
        }
    }
    pub fn prev(&mut self) {
        use self::MenuElt::*;
        *self = match *self {
            Clock => SetClock,
            SetClock => Clock,
        }
    }
    pub fn cancel(&mut self) -> StateManager {
        StateManager::Clock
    }
    pub fn items(self) -> &'static [&'static str] {
        &["Main screen", "Set clock"]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDateTime {
    pub datetime: datetime::DateTime,
    state: EditDateTimeState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditDateTimeState {
    Year,
    Month,
    Day,
    Hour,
    Min,
}
impl EditDateTime {
    pub fn new(datetime: datetime::DateTime) -> Self {
        Self {
            datetime,
            state: EditDateTimeState::Year,
        }
    }
    pub fn next(&mut self) {
        use self::EditDateTimeState::*;
        match self.state {
            Year => {
                self.datetime.year += 1;
                if self.datetime.year > 2105 {
                    self.datetime.year = 1970;
                }
            }
            Month => self.datetime.month = self.datetime.month % 12 + 1,
            Day => self.datetime.day = self.datetime.day % 31 + 1,
            Hour => self.datetime.hour = (self.datetime.hour + 1) % 24,
            Min => self.datetime.min = (self.datetime.min + 1) % 60,
        }
    }
    pub fn prev(&mut self) {
        use self::EditDateTimeState::*;
        match self.state {
            Year => {
                self.datetime.year -= 1;
                if self.datetime.year < 1970 {
                    self.datetime.year = 2105;
                }
            }
            Month => self.datetime.month = (self.datetime.month + 12 - 2) % 12 + 1,
            Day => self.datetime.day = (self.datetime.day + 31 - 2) % 31 + 1,
            Hour => self.datetime.hour = (self.datetime.hour + 24 - 1) % 24,
            Min => self.datetime.min = (self.datetime.min + 60 - 1) % 60,
        }
    }
    pub fn cancel(&mut self) -> StateManager {
        use self::EditDateTimeState::*;
        match self.state {
            Year => return StateManager::Menu(MenuElt::SetClock),
            Month => self.state = Year,
            Day => self.state = Month,
            Hour => self.state = Day,
            Min => self.state = Hour,
        }
        StateManager::SetClock(self.clone())
    }
    pub fn ok(&mut self) -> Option<datetime::DateTime> {
        use self::EditDateTimeState::*;
        match self.state {
            Year => self.state = Month,
            Month => self.state = Day,
            Day => self.state = Hour,
            Hour => self.state = Min,
            Min => return Some(self.datetime.clone()),
        }
        None
    }
    pub fn as_edit_str(&self) -> &'static str {
        use self::EditDateTimeState::*;
        match self.state {
            Year => "Set year",
            Month => "Set month",
            Day => "Set day",
            Hour => "Set hour",
            Min => "Set minute",
        }
    }
}