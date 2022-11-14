use crate::OnEvent;
use std::collections::HashMap;
use zbus::fdo::Error;
use zbus::zvariant::{Array, OwnedValue, Signature, Str, Structure, Value};

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
        Item {
            message: None,
            r#type: Type::Standard,
            label: "".to_string(),
            enabled: true,
            visible: true,
            icon_name: "".to_string(),
            icon_data: vec![],
            shortcut: vec![],
            disposition: Disposition::Normal,
        }
    }
}

impl<M> Item<M> {
    fn get_property(&self, prop: &str) -> Option<OwnedValue> {
        let props = self.get_properties();
        let value = props.get(prop).cloned();
        let value = match prop {
            "type" => value.unwrap_or_else(|| Str::from("standard").into()),
            "label" => value.unwrap_or_else(|| Str::from("").into()),
            "enabled" => value.unwrap_or_else(|| true.into()),
            "visible" => value.unwrap_or_else(|| true.into()),
            "icon-name" => value.unwrap_or_else(|| Str::from("").into()),
            "icon-data" => value.unwrap_or_else(|| {
                Array::new(Signature::try_from("ay").expect("valid signature")).into()
            }),
            "shortcut" => value.unwrap_or_else(|| {
                Array::new(Signature::try_from("a(as)").expect("valid signature")).into()
            }),
            "toggle-type" => value.unwrap_or_else(|| Str::from("").into()),
            "toggle-state" => value.unwrap_or_else(|| (-1).into()),
            "children-display" => value.unwrap_or_else(|| Str::from("").into()),
            "disposition" => value.unwrap_or_else(|| Str::from("normal").into()),
            _ => return None,
        };
        Some(value)
    }

    fn get_properties(&self) -> HashMap<String, OwnedValue> {
        let mut props = HashMap::new();
        match &self.r#type {
            Type::Standard => {}
            Type::Separator => {
                props.insert("type".to_string(), Str::from("separator").into());
            }
            Type::Checkmark { selected } => {
                props.insert("toggle-type".to_string(), Str::from("checkmark").into());
                props.insert(
                    "toggle-state".to_string(),
                    if *selected { 1.into() } else { 0.into() },
                );
            }
            Type::Radio { selected } => {
                props.insert("toggle-type".to_string(), Str::from("radio").into());
                props.insert(
                    "toggle-state".to_string(),
                    if *selected { 1.into() } else { 0.into() },
                );
            }
            Type::SubMenu { .. } => {
                props.insert("children-display".to_string(), Str::from("submenu").into());
            }
        }
        if !self.label.is_empty() {
            props.insert("label".to_string(), Str::from(&self.label).into());
        }
        if !self.enabled {
            props.insert("enabled".to_string(), self.enabled.into());
        }
        if !self.visible {
            props.insert("visible".to_string(), self.visible.into());
        }
        if !self.icon_name.is_empty() {
            props.insert("icon-name".to_string(), Str::from(&self.icon_name).into());
        }
        if !self.icon_data.is_empty() {
            props.insert("icon-data".to_string(), Array::from(&self.icon_data).into());
        }
        if !self.shortcut.is_empty() {
            let mut shortcuts = Array::new(Signature::try_from("a(as)").expect("valid signature"));
            for shortcut in &self.shortcut {
                let shortcut = Array::from(shortcut);
                shortcuts
                    .append(Value::from(shortcut))
                    .expect("signature to match");
            }
            props.insert("shortcut".to_string(), shortcuts.into());
        }
        if self.disposition != Disposition::Normal {
            let s: &str = self.disposition.into();
            props.insert("disposition".to_string(), Str::from(s).into());
        }
        props
    }

    fn get_properties_filtered(&self, property_names: &[&str]) -> HashMap<String, OwnedValue> {
        let mut props = self.get_properties();
        if !property_names.is_empty() {
            props.retain(|k, _| property_names.contains(&k.as_str()));
        }
        props
    }
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
        DBusMenu {
            revision: 0,
            model,
            on_event,
        }
    }

    fn get(&self, id: i32) -> Option<&Item<M>> {
        self.model.items.get(id as usize)
    }

    async fn on_event(&self, event: Event<M>) {
        let pinned = Box::into_pin(self.on_event.on_event(event));
        pinned.await;
    }

    fn get_layout_recursive(
        &self,
        id: i32,
        recursion_depth: i32,
        property_names: &[&str],
    ) -> Result<
        (
            i32,
            HashMap<String, OwnedValue>,
            Vec<zbus::zvariant::OwnedValue>,
        ),
        Error,
    > {
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
                    .map(|&Id(id)| {
                        self.get_layout_recursive(id, new_recursion_depth, property_names)
                    })
                    .map(|x| match x {
                        Ok(x) => Ok(Structure::from(x).into()),
                        Err(e) => Err(e),
                    })
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

impl<M: Clone> DBusMenu<M> {
    async fn handle_single_event(&self, id: i32, event_id: &str) -> Result<bool, Error> {
        let r#type =
            EventType::try_from(event_id).map_err(|s| Error::InvalidArgs(s.to_string()))?;
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
    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    /// AboutToShowGroup method
    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }

    /// Event method
    async fn event(
        &self,
        id: i32,
        event_id: &str,
        _data: zbus::zvariant::Value<'_>,
        _timestamp: u32,
    ) -> Result<(), Error> {
        if !self.handle_single_event(id, event_id).await? {
            Err(Error::InvalidArgs(format!("unknown ID {id}")))
        } else {
            Ok(())
        }
    }

    /// EventGroup method
    async fn event_group(
        &self,
        events: Vec<(i32, &str, zbus::zvariant::Value<'_>, u32)>,
    ) -> Result<Vec<i32>, Error> {
        let mut errors = Vec::new();
        for &(id, event_type, _, _) in &events {
            if !self.handle_single_event(id, event_type).await? {
                errors.push(id);
            }
        }
        if errors.len() == events.len() {
            Err(Error::InvalidArgs("no valid IDs".to_string()))
        } else {
            Ok(errors)
        }
    }

    /// GetGroupProperties method
    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<&str>,
    ) -> Vec<(i32, HashMap<String, zbus::zvariant::OwnedValue>)> {
        if ids.is_empty() {
            self.model
                .items
                .iter()
                .enumerate()
                .map(|(id, item)| (id as i32, item.get_properties_filtered(&property_names)))
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
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<&str>,
    ) -> Result<
        (
            u32,
            (
                i32,
                HashMap<String, OwnedValue>,
                Vec<zbus::zvariant::OwnedValue>,
            ),
        ),
        Error,
    > {
        let layout = self.get_layout_recursive(parent_id, recursion_depth, &property_names)?;
        Ok((self.revision, layout))
    }

    /// GetProperty method
    fn get_property(&self, id: i32, name: &str) -> Result<OwnedValue, Error> {
        let item = self
            .get(id)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid ID {id}")))?;
        item.get_property(name)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid property name {name}")))
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
