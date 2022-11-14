use crate::OnEvent;
use std::collections::HashMap;
use zbus::{
    fdo::Error,
    zvariant::{Array, OwnedValue, Signature, Str, Structure, Value},
};

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    zbus::zvariant::Type,
    zbus::zvariant::Value,
)]
#[serde(transparent)]
pub struct Id(pub i32);

impl Id {
    pub const ROOT: Id = Id(0);
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, zbus::zvariant::Type)]
#[zvariant(signature = "s")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl From<TextDirection> for Value<'static> {
    fn from(v: TextDirection) -> Self {
        let s = match v {
            TextDirection::LeftToRight => "ltr",
            TextDirection::RightToLeft => "rtl",
        };
        Value::from(Str::from(s))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, zbus::zvariant::Type)]
#[zvariant(signature = "s")]
pub enum Status {
    Normal,
    Notice,
}

impl From<Status> for Value<'static> {
    fn from(v: Status) -> Self {
        let s = match v {
            Status::Normal => "normal",
            Status::Notice => "notice",
        };
        Value::from(Str::from(s))
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

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    zbus::zvariant::Type,
)]
#[serde(rename_all = "kebab-case")]
#[zvariant(signature = "s")]
enum Prop {
    Type,
    Label,
    Enabled,
    Visible,
    IconName,
    IconData,
    Shortcut,
    ToggleType,
    ToggleState,
    ChildrenDisplay,
    Disposition,
}

impl From<Prop> for Value<'static> {
    fn from(v: Prop) -> Self {
        let s = match v {
            Prop::Type => "type",
            Prop::Label => "label",
            Prop::Enabled => "enabled",
            Prop::Visible => "visible",
            Prop::IconName => "icon-name",
            Prop::IconData => "icon-data",
            Prop::Shortcut => "shortcut",
            Prop::ToggleType => "toggle-type",
            Prop::ToggleState => "toggle-state",
            Prop::ChildrenDisplay => "children-display",
            Prop::Disposition => "disposition",
        };
        Value::from(Str::from(s))
    }
}

impl Prop {
    const ALL_PROPS: [Prop; 11] = [
        Prop::Type,
        Prop::Label,
        Prop::Enabled,
        Prop::Visible,
        Prop::IconName,
        Prop::IconData,
        Prop::Shortcut,
        Prop::ToggleType,
        Prop::ToggleState,
        Prop::ChildrenDisplay,
        Prop::Disposition,
    ];
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
    pub shortcut: Vec<Vec<String>>,
    /// How the menuitem feels the information it's displaying to the user should be presented.
    /// - `normal` a standard menu item
    /// - `informative` providing additional information to the user
    /// - `warning` looking at potentially harmful results
    /// - `alert` something bad could potentially happen
    pub disposition: Disposition,
}

impl<M> Default for Item<M> {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<M> Item<M> {
    const DEFAULT: Item<M> = Item {
        message: None,
        r#type: Type::Standard,
        label: String::new(),
        enabled: true,
        visible: true,
        icon_name: String::new(),
        icon_data: Vec::new(),
        shortcut: Vec::new(),
        disposition: Disposition::Normal,
    };

    fn default_value(&self, prop: Prop) -> OwnedValue {
        match prop {
            Prop::Type => Str::from("standard").into(),
            Prop::Label => Str::from("").into(),
            Prop::Enabled => Self::DEFAULT.enabled.into(),
            Prop::Visible => Self::DEFAULT.enabled.into(),
            Prop::IconName => Str::from(&Self::DEFAULT.icon_name).into(),
            Prop::IconData => Array::from(&self.icon_data).into(),
            Prop::Shortcut => {
                Array::new(Signature::try_from("a(as)").expect("valid signature")).into()
            }
            Prop::ToggleType => Str::from("").into(),
            Prop::ToggleState => (-1).into(),
            Prop::ChildrenDisplay => Str::from("").into(),
            Prop::Disposition => Str::from(Into::<&str>::into(self.disposition)).into(),
        }
    }

    fn get_property(&self, prop: Prop) -> Option<OwnedValue> {
        match prop {
            Prop::Type => {
                if self.r#type == Type::Separator {
                    Some(Str::from("separator").into())
                } else {
                    None
                }
            }
            Prop::Label => {
                if self.label != Self::DEFAULT.label {
                    Some(Str::from(&self.label).into())
                } else {
                    None
                }
            }
            Prop::Enabled => {
                if self.enabled != Self::DEFAULT.enabled {
                    Some(self.enabled.into())
                } else {
                    None
                }
            }
            Prop::Visible => {
                if self.visible != Self::DEFAULT.visible {
                    Some(self.visible.into())
                } else {
                    None
                }
            }
            Prop::IconName => {
                if self.icon_name != Self::DEFAULT.icon_name {
                    Some(Str::from(&self.icon_name).into())
                } else {
                    None
                }
            }
            Prop::IconData => {
                if self.icon_data != Self::DEFAULT.icon_data {
                    Some(Array::from(&self.icon_data).into())
                } else {
                    None
                }
            }
            Prop::Shortcut => {
                if self.shortcut != Self::DEFAULT.shortcut {
                    let mut shortcuts =
                        Array::new(Signature::try_from("a(as)").expect("valid signature"));
                    for shortcut in &self.shortcut {
                        let shortcut = Array::from(shortcut);
                        shortcuts
                            .append(Value::from(shortcut))
                            .expect("signature to match");
                    }
                    Some(shortcuts.into())
                } else {
                    None
                }
            }
            Prop::ToggleType => match &self.r#type {
                Type::Checkmark { .. } => Some(Str::from("checkmark").into()),
                Type::Radio { .. } => Some(Str::from("radio").into()),
                _ => None,
            },
            Prop::ToggleState => match &self.r#type {
                Type::Checkmark { selected } => Some(if *selected { 1 } else { 0 }.into()),
                Type::Radio { selected } => Some(if *selected { 1 } else { 0 }.into()),
                _ => None,
            },
            Prop::ChildrenDisplay => {
                if let Type::SubMenu { .. } = &self.r#type {
                    Some(Str::from("submenu").into())
                } else {
                    None
                }
            }
            Prop::Disposition => {
                if self.disposition != Self::DEFAULT.disposition {
                    Some(Str::from(Into::<&str>::into(self.disposition)).into())
                } else {
                    None
                }
            }
        }
    }

    fn get_property_or_default(&self, prop: Prop) -> OwnedValue {
        self.get_property(prop)
            .unwrap_or_else(|| self.default_value(prop))
    }

    fn get_properties_filtered(&self, property_names: &[Prop]) -> HashMap<Prop, OwnedValue> {
        let props = if property_names.is_empty() {
            property_names.iter()
        } else {
            Prop::ALL_PROPS.iter()
        };
        props
            .copied()
            .filter_map(|p| self.get_property(p).map(|v| (p, v)))
            .collect()
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    zbus::zvariant::Type,
)]
#[serde(rename_all = "kebab-case")]
#[zvariant(signature = "s")]
pub enum EventType {
    Clicked,
    Hovered,
    Opened,
    Closed,
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

type Layout = (
    Id,
    HashMap<Prop, OwnedValue>,
    Vec<zbus::zvariant::OwnedValue>,
);

impl<M> DBusMenu<M> {
    pub fn new(model: Model<M>, on_event: Box<dyn OnEvent<Event<M>>>) -> DBusMenu<M> {
        DBusMenu {
            revision: 0,
            model,
            on_event,
        }
    }

    fn get(&self, id: Id) -> Option<&Item<M>> {
        self.model.items.get(id.0 as usize)
    }

    async fn on_event(&self, event: Event<M>) {
        let pinned = Box::into_pin(self.on_event.on_event(event));
        pinned.await;
    }

    fn get_layout_recursive(
        &self,
        id: Id,
        recursion_depth: i32,
        property_names: &[Prop],
    ) -> Result<Layout, Error> {
        let item = self
            .get(id)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid ID {id}")))?;
        let props = item.get_properties_filtered(&property_names);
        let children = if recursion_depth != 0 {
            let new_recursion_depth = if recursion_depth < 0 {
                recursion_depth
            } else {
                recursion_depth - 1
            };
            if let Type::SubMenu { children } = &item.r#type {
                children
                    .iter()
                    .map(|&id| self.get_layout_recursive(id, new_recursion_depth, property_names))
                    .map(|result| result.map(|s| Structure::from(s).into()))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        Ok((id, props, children))
    }
}

#[zbus::dbus_interface(interface = "com.canonical.dbusmenu")]
impl<M: Clone + Send + Sync + 'static> DBusMenu<M> {
    /// AboutToShow method
    fn about_to_show(&self, _id: Id) -> bool {
        false
    }

    /// AboutToShowGroup method
    fn about_to_show_group(&self, _ids: Vec<Id>) -> (Vec<Id>, Vec<Id>) {
        (Vec::new(), Vec::new())
    }

    /// Event method
    async fn event(
        &self,
        id: Id,
        event_id: EventType,
        _data: zbus::zvariant::Value<'_>,
        _timestamp: u32,
    ) -> Result<(), Error> {
        let item = self
            .get(id)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid ID {id}")))?;
        if let Some(message) = &item.message {
            self.on_event(Event {
                r#type: event_id,
                message: message.clone(),
            })
            .await;
        }
        Ok(())
    }

    /// EventGroup method
    async fn event_group(
        &self,
        events: Vec<(Id, EventType, zbus::zvariant::Value<'_>, u32)>,
    ) -> Result<Vec<Id>, Error> {
        let mut errors = Vec::new();
        let count = events.len();
        for (id, event_type, data, timestamp) in events {
            if self.event(id, event_type, data, timestamp).await.is_err() {
                errors.push(id);
            }
        }
        if errors.len() == count {
            Err(Error::InvalidArgs("no valid IDs".to_string()))
        } else {
            Ok(errors)
        }
    }

    /// GetGroupProperties method
    fn get_group_properties(
        &self,
        ids: Vec<Id>,
        property_names: Vec<Prop>,
    ) -> Vec<(Id, HashMap<Prop, zbus::zvariant::OwnedValue>)> {
        if ids.is_empty() {
            self.model
                .items
                .iter()
                .enumerate()
                .map(|(id, item)| (Id(id as i32), item.get_properties_filtered(&property_names)))
                .collect()
        } else {
            ids.iter()
                .filter_map(|&id| self.get(id).map(|item| (id, item)))
                .map(|(id, item)| (id, item.get_properties_filtered(&property_names)))
                .collect()
        }
    }

    /// GetLayout method
    fn get_layout(
        &self,
        parent_id: Id,
        recursion_depth: i32,
        property_names: Vec<Prop>,
    ) -> Result<(u32, Layout), Error> {
        let layout = self.get_layout_recursive(parent_id, recursion_depth, &property_names)?;
        Ok((self.revision, layout))
    }

    /// GetProperty method
    fn get_property(&self, id: Id, name: Prop) -> Result<OwnedValue, Error> {
        let item = self
            .get(id)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid ID {id}")))?;
        Ok(item.get_property_or_default(name))
    }

    /// ItemActivationRequested signal
    #[dbus_interface(signal)]
    async fn item_activation_requested(
        ctx: &zbus::SignalContext<'_>,
        id: Id,
        timestamp: u32,
    ) -> zbus::Result<()>;

    /// ItemsPropertiesUpdated signal
    #[dbus_interface(signal)]
    async fn items_properties_updated(
        ctx: &zbus::SignalContext<'_>,
        updated_props: &[(Id, HashMap<Prop, Value<'_>>)],
        removed_props: &[(Id, &[Prop])],
    ) -> zbus::Result<()>;

    /// LayoutUpdated signal
    #[dbus_interface(signal)]
    async fn layout_updated(
        ctx: &zbus::SignalContext<'_>,
        revision: u32,
        parent: Id,
    ) -> zbus::Result<()>;

    /// IconThemePath property
    #[dbus_interface(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        self.model.icon_theme_path.clone()
    }

    /// Status property
    #[dbus_interface(property)]
    fn status(&self) -> Status {
        self.model.status
    }

    /// TextDirection property
    #[dbus_interface(property)]
    fn text_direction(&self) -> TextDirection {
        self.model.text_direction
    }

    /// Version property
    #[dbus_interface(property)]
    fn version(&self) -> u32 {
        1
    }
}
