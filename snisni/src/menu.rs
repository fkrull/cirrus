#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Id(pub i32);

impl Id {
    pub const ROOT: Id = Id(0);
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Disposition {
    Normal,
    Informative,
    Warning,
    Alert,
}

impl From<Disposition> for &'static str {
    fn from(v: Disposition) -> Self {
        match v {
            Disposition::Normal => "normal",
            Disposition::Informative => "informative",
            Disposition::Warning => "warning",
            Disposition::Alert => "alert",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl From<TextDirection> for &'static str {
    fn from(v: TextDirection) -> Self {
        match v {
            TextDirection::LeftToRight => "ltr",
            TextDirection::RightToLeft => "rtl",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Normal,
    Notice,
}

impl From<Status> for &'static str {
    fn from(v: Status) -> Self {
        match v {
            Status::Normal => "normal",
            Status::Notice => "notice",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type<Ev> {
    Standard { event: Ev },
    Separator,
    Checkmark { selected: bool, event: Ev },
    Radio { selected: bool, event: Ev },
    SubMenu { children: Vec<Id>, event: Ev },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Item<Ev> {
    pub id: Id,
    pub r#type: Type<Ev>,
    /// Text of the item, except that:
    /// - two consecutive underscore characters `__` are displayed as a
    ///   single underscore,
    /// - any remaining underscore characters are not displayed at all,
    /// - the first of those remaining underscore characters (unless it is
    ///   the last character in the string) indicates that the following
    ///   character is the access key.
    pub label: String,
    /// Whether the item can be activated or not
    pub enabled: bool,
    /// True if the item is visible in the menu
    pub visible: bool,
    /// Icon name of the item, following the freedesktop.org icon spec
    pub icon_name: String,
    /// PNG data of the icon
    pub icon_data: Vec<u8>,
    /// The shortcut of the item. Each array represents the key press in the list of keypresses.
    /// Each list of strings contains a list of modifiers and then the key that is used. The
    /// modifier strings allowed are: `Control`, `Alt`, `Shift` and `Super`.
    /// - A simple shortcut like Ctrl+S is represented as: `[["Control", "S"]]`
    /// - A complex shortcut like Ctrl+Q, Alt+X is represented as: `[["Control", "Q"], ["Alt", "X"]]`
    pub shortcuts: Vec<Vec<String>>,
    /// How the menuitem feels the information it's displaying to the user should be presented.
    /// - `normal` a standard menu item
    /// - `informative` providing additional information to the user
    /// - `warning` looking at potentially harmful results
    /// - `alert` something bad could potentially happen
    pub disposition: Disposition,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Clicked,
    Hovered,
    Opened,
    Closed,
}

impl<'a> TryFrom<&'a str> for EventType {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "clicked" => Ok(EventType::Clicked),
            "hovered" => Ok(EventType::Hovered),
            "opened" => Ok(EventType::Opened),
            "closed" => Ok(EventType::Closed),
            _ => Err(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Menu<Ev> {
    pub text_direction: TextDirection,
    pub status: Status,
    pub icon_theme_path: String,
    pub items: Vec<Item<Ev>>,
}
