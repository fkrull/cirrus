use crate::OnEvent;
use std::collections::HashMap;

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
pub enum Type {
    Standard,
    Separator,
    Checkmark { selected: bool },
    Radio { selected: bool },
    SubMenu { children: Vec<Id> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Item<M> {
    pub message: Option<M>,
    pub r#type: Type,
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
pub struct Event<M> {
    pub r#type: EventType,
    pub message: M,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Model<M> {
    pub text_direction: TextDirection,
    pub status: Status,
    pub icon_theme_path: Vec<String>,
    pub items: Vec<Item<M>>,
}

pub struct DBusMenu<M> {
    revision: u32,
    model: Model<M>,
    on_event: Box<dyn OnEvent<Event<M>>>,
}

impl<M> DBusMenu<M> {
    pub fn new(model: Model<M>, on_event: Box<dyn OnEvent<Event<M>>>) -> DBusMenu<M> {
        // TODO hierarchical model I suppose
        let mut menu = DBusMenu {
            revision: 0,
            model,
            on_event,
        };
        menu
    }

    fn get(&self, id: i32) -> Option<&Item<M>> {
        self.model.items.get(id as usize)
    }

    async fn on_event(&self, event: Event<M>) {
        let pinned = Box::into_pin(self.on_event.on_event(event));
        pinned.await;
    }
}

impl<M: Clone> DBusMenu<M> {
    async fn handle_event(&self, id: i32, event_id: &str) -> Result<bool, zbus::fdo::Error> {
        let r#type = EventType::try_from(event_id)
            .map_err(|s| zbus::fdo::Error::InvalidArgs(s.to_string()))?;
        if let Some(item) = self.get(id) {
            if let Some(message) = &item.message {
                self.on_event(Event {
                    r#type,
                    message: message.clone(),
                })
                .await;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[zbus::dbus_interface(interface = "com.canonical.dbusmenu")]
impl<M: Clone + Send + Sync + 'static> DBusMenu<M> {
    /// AboutToShow method
    async fn about_to_show(&self, id: i32) -> bool {
        false
    }

    /// AboutToShowGroup method
    async fn about_to_show_group(&self, ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }

    /// Event method
    async fn event(
        &self,
        id: i32,
        event_id: &str,
        _data: zbus::zvariant::Value<'_>,
        _timestamp: u32,
    ) -> Result<(), zbus::fdo::Error> {
        if !self.handle_event(id, event_id).await? {
            Err(zbus::fdo::Error::InvalidArgs(format!("unknown ID {id}")))
        } else {
            Ok(())
        }
    }

    /// EventGroup method
    async fn event_group(
        &self,
        events: Vec<(i32, &str, zbus::zvariant::Value<'_>, u32)>,
    ) -> Result<Vec<i32>, zbus::fdo::Error> {
        let mut errors = Vec::new();
        for &(id, event_type, _, _) in &events {
            if !self.handle_event(id, event_type).await? {
                errors.push(id);
            }
        }
        if errors.len() == events.len() {
            Err(zbus::fdo::Error::InvalidArgs("no valid IDs".to_string()))
        } else {
            Ok(errors)
        }
    }

    /// GetGroupProperties method
    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<&str>,
    ) -> Vec<(i32, HashMap<String, zbus::zvariant::OwnedValue>)> {
        todo!()
    }

    /// GetLayout method
    async fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<&str>,
    ) -> (
        u32,
        (
            i32,
            HashMap<String, zbus::zvariant::OwnedValue>,
            Vec<zbus::zvariant::OwnedValue>,
        ),
    ) {
        todo!()
    }

    /// GetProperty method
    async fn get_property(&self, id: i32, name: &str) -> zbus::zvariant::OwnedValue {
        todo!()
    }

    /// ItemActivationRequested signal
    #[dbus_interface(signal)]
    async fn item_activation_requested(
        ctx: &zbus::SignalContext<'_>,
        id: i32,
        timestamp: u32,
    ) -> zbus::Result<()>;

    /// ItemsPropertiesUpdated signal
    #[dbus_interface(signal)]
    async fn items_properties_updated(
        ctx: &zbus::SignalContext<'_>,
        updated_props: &[(i32, HashMap<&str, zbus::zvariant::Value<'_>>)],
        removed_props: &[(i32, &[&str])],
    ) -> zbus::Result<()>;

    /// LayoutUpdated signal
    #[dbus_interface(signal)]
    async fn layout_updated(
        ctx: &zbus::SignalContext<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;

    /// IconThemePath property
    #[dbus_interface(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        self.model.icon_theme_path.clone()
    }

    /// Status property
    #[dbus_interface(property)]
    fn status(&self) -> &str {
        self.model.status.into()
    }

    /// TextDirection property
    #[dbus_interface(property)]
    fn text_direction(&self) -> &str {
        self.model.text_direction.into()
    }

    /// Version property
    #[dbus_interface(property)]
    fn version(&self) -> u32 {
        1
    }
}
