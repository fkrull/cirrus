use crate::OnEvent;
use std::collections::HashMap;
use zbus::{
    fdo::Error,
    zvariant::{Array, OwnedValue, Signature, Str, Structure, Value},
    SignalContext,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Disposition {
    Normal,
    Informative,
    Warning,
    Alert,
}

impl From<Disposition> for OwnedValue {
    fn from(v: Disposition) -> Self {
        let s = match v {
            Disposition::Normal => "normal",
            Disposition::Informative => "informative",
            Disposition::Warning => "warning",
            Disposition::Alert => "alert",
        };
        Str::from(s).into()
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
    SubMenu { children: Vec<usize> },
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
pub enum Prop {
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
            Prop::Disposition => self.disposition.into(),
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
                Type::Checkmark { selected } => Some(i32::from(*selected).into()),
                Type::Radio { selected } => Some(i32::from(*selected).into()),
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
                    Some(self.disposition.into())
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
        let props = if !property_names.is_empty() {
            property_names.iter()
        } else {
            Prop::ALL_PROPS.iter()
        };
        props
            .copied()
            .filter_map(|p| self.get_property(p).map(|v| (p, v)))
            .collect()
    }

    fn children(&self) -> Option<&[usize]> {
        if let Type::SubMenu { children } = &self.r#type {
            Some(children)
        } else {
            None
        }
    }

    fn diff(&self, other: &Item<M>) -> (HashMap<Prop, OwnedValue>, Vec<Prop>, bool) {
        if self.children() != other.children() {
            (HashMap::new(), Vec::new(), true)
        } else {
            let old_props = self.get_properties_filtered(&[]);
            let new_props = other.get_properties_filtered(&[]);
            let mut removed_props = Vec::new();
            for old_key in old_props.keys() {
                if !new_props.contains_key(old_key) {
                    removed_props.push(*old_key);
                }
            }
            (new_props, removed_props, false)
        }
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

impl<M> Default for Model<M> {
    fn default() -> Self {
        Model {
            text_direction: TextDirection::LeftToRight,
            status: Status::Normal,
            icon_theme_path: Vec::new(),
            items: Vec::new(),
        }
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
    zbus::zvariant::Value,
)]
#[serde(transparent)]
pub struct AdjustedId(pub i32);

impl std::fmt::Display for AdjustedId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

enum LayoutDiff {
    PropertiesUpdated {
        props_updated: Vec<(AdjustedId, HashMap<Prop, OwnedValue>)>,
        props_removed: Vec<(AdjustedId, Vec<Prop>)>,
    },
    LayoutInvalidated,
}

pub struct DBusMenu<M> {
    revision: u32,
    offset: usize,
    model: Model<M>,
    on_event: Box<dyn OnEvent<Event<M>>>,
}

type Layout = (
    AdjustedId,
    HashMap<Prop, OwnedValue>,
    Vec<zbus::zvariant::OwnedValue>,
);

impl<M> DBusMenu<M> {
    pub fn new(model: Model<M>, on_event: Box<dyn OnEvent<Event<M>>>) -> DBusMenu<M> {
        DBusMenu {
            revision: 0,
            offset: 0,
            model,
            on_event,
        }
    }

    async fn on_event(&self, event: Event<M>) {
        let pinned = Box::into_pin(self.on_event.on_event(event));
        pinned.await;
    }

    fn to_offset(&self, id: AdjustedId) -> usize {
        if id.0 == 0 {
            0
        } else {
            id.0 as usize - self.offset
        }
    }

    fn to_id(&self, offset: usize) -> AdjustedId {
        if offset == 0 {
            AdjustedId(0)
        } else {
            AdjustedId((offset + self.offset) as i32)
        }
    }

    fn get(&self, id: AdjustedId) -> Option<&Item<M>> {
        self.model.items.get(self.to_offset(id))
    }

    fn diff_layouts(&self, old: &[Item<M>]) -> LayoutDiff {
        let new = &self.model.items;
        let mut invalidated = false;
        let mut props_updated = Vec::new();
        let mut props_removed = Vec::new();
        if old.len() != new.len() {
            invalidated = true;
        } else {
            let diffs = old
                .iter()
                .zip(new.iter())
                .map(|(a, b)| a.diff(b))
                .enumerate();
            for (offset, (updated, removed, invalidate)) in diffs {
                if invalidate {
                    invalidated = true;
                    break;
                }
                if !updated.is_empty() {
                    props_updated.push((self.to_id(offset), updated));
                }
                if !removed.is_empty() {
                    props_removed.push((self.to_id(offset), removed));
                }
            }
        }

        if invalidated {
            LayoutDiff::LayoutInvalidated
        } else {
            LayoutDiff::PropertiesUpdated {
                props_updated,
                props_removed,
            }
        }
    }

    fn get_layout_recursive(
        &self,
        id: AdjustedId,
        recursion_depth: i32,
        property_names: &[Prop],
    ) -> Result<Layout, Error> {
        let item = self
            .get(id)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid ID {id}")))?;
        let props = item.get_properties_filtered(property_names);
        let children = if recursion_depth != 0 {
            let new_recursion_depth = if recursion_depth < 0 {
                recursion_depth
            } else {
                recursion_depth - 1
            };
            item.children()
                .iter()
                .flat_map(|o| o.iter())
                .map(|&offset| {
                    let id = self.to_id(offset);
                    self.get_layout_recursive(id, new_recursion_depth, property_names)
                })
                .map(|result| result.map(|s| Structure::from(s).into()))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        Ok((id, props, children))
    }
}

impl<M: Clone + Send + Sync + 'static> DBusMenu<M> {
    pub async fn update(
        &mut self,
        ctx: &SignalContext<'_>,
        f: impl FnOnce(&mut Model<M>),
    ) -> zbus::Result<()> {
        let old = self.model.items.clone();
        f(&mut self.model);
        match self.diff_layouts(&old) {
            LayoutDiff::PropertiesUpdated {
                props_updated,
                props_removed,
            } => {
                DBusMenu::<M>::items_properties_updated(ctx, &props_updated, &props_removed)
                    .await?;
            }
            LayoutDiff::LayoutInvalidated => {
                self.revision += 1;
                self.offset += old.len();
                DBusMenu::<M>::layout_updated(ctx, self.revision, AdjustedId(0)).await?;
            }
        }
        Ok(())
    }
}

#[zbus::dbus_interface(interface = "com.canonical.dbusmenu")]
impl<M: Clone + Send + Sync + 'static> DBusMenu<M> {
    /// AboutToShow method
    fn about_to_show(&self, _id: AdjustedId) -> bool {
        false
    }

    /// AboutToShowGroup method
    fn about_to_show_group(&self, _ids: Vec<AdjustedId>) -> (Vec<AdjustedId>, Vec<AdjustedId>) {
        (Vec::new(), Vec::new())
    }

    /// Event method
    async fn event(
        &self,
        id: AdjustedId,
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
        events: Vec<(AdjustedId, EventType, zbus::zvariant::Value<'_>, u32)>,
    ) -> Result<Vec<AdjustedId>, Error> {
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
        ids: Vec<AdjustedId>,
        property_names: Vec<Prop>,
    ) -> Vec<(AdjustedId, HashMap<Prop, zbus::zvariant::OwnedValue>)> {
        if ids.is_empty() {
            self.model
                .items
                .iter()
                .enumerate()
                .map(|(offset, item)| {
                    (
                        self.to_id(offset),
                        item.get_properties_filtered(&property_names),
                    )
                })
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
        parent_id: AdjustedId,
        recursion_depth: i32,
        property_names: Vec<Prop>,
    ) -> Result<(u32, Layout), Error> {
        let layout = self.get_layout_recursive(parent_id, recursion_depth, &property_names)?;
        Ok((self.revision, layout))
    }

    /// GetProperty method
    fn get_property(&self, id: AdjustedId, name: Prop) -> Result<OwnedValue, Error> {
        let item = self
            .get(id)
            .ok_or_else(|| Error::InvalidArgs(format!("invalid ID {id}")))?;
        Ok(item.get_property_or_default(name))
    }

    /// The server is requesting that all clients displaying this
    /// menu open it to the user. This would be for things like
    /// hotkeys that when the user presses them the menu should
    /// open and display itself to the user.
    ///
    /// * `id` - ID of the menu that should be activated
    /// * `timestamp` - The time that the event occured
    #[dbus_interface(signal)]
    pub async fn item_activation_requested(
        ctx: &SignalContext<'_>,
        id: AdjustedId,
        timestamp: u32,
    ) -> zbus::Result<()>;

    /// Triggered when there are lots of property updates across many items so they all get grouped
    /// into a single dbus message. The format is the ID of the item with a hashtable of names and
    /// values for those properties.
    #[dbus_interface(signal)]
    pub async fn items_properties_updated(
        ctx: &SignalContext<'_>,
        updated_props: &[(AdjustedId, HashMap<Prop, OwnedValue>)],
        removed_props: &[(AdjustedId, Vec<Prop>)],
    ) -> zbus::Result<()>;

    ///Triggered by the application to notify display of a layout update, up to
    /// revision.
    ///
    /// * `revision` - The revision of the layout that we're currently on
    /// * `parent` - If the layout update is only of a subtree, this is the parent item for the
    ///   entries that have changed. It is zero if the whole layout should be considered invalid.
    #[dbus_interface(signal)]
    pub async fn layout_updated(
        ctx: &SignalContext<'_>,
        revision: u32,
        parent: AdjustedId,
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
        // that's what ksni uses *shrug emoji*
        3
    }
}
