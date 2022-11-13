use crate::menu::Type;
use crate::{menu, Event, Item, OnEvent, Pixmap, ScrollOrientation, MENU_OBJECT_PATH};
use std::collections::HashMap;
use zbus::{dbus_interface, dbus_proxy, SignalContext};

#[derive(Debug)]
pub(crate) struct DBusMenu<Ev> {
    pub(crate) model: menu::Menu<Ev>,
    pub(crate) revision: u32,
    pub(crate) indices: HashMap<menu::Id, usize>,
}

#[dbus_interface(interface = "com.canonical.dbusmenu")]
impl<Ev: Send + Sync + 'static> DBusMenu<Ev> {
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
        let event_type = menu::EventType::try_from(event_id)
            .map_err(|s| zbus::fdo::Error::InvalidArgs(s.to_string()))?;
        if let Some(item) = self
            .indices
            .get(&menu::Id(id))
            .and_then(|&index| self.model.items.get(index))
        {
            match &item.r#type {
                Type::Standard { event, .. } => {}
                Type::Separator => {}
                Type::Checkmark { .. } => {}
                Type::Radio { .. } => {}
                Type::SubMenu { .. } => {}
            }
        }
        if let Some(&index) = self.indices.get(&menu::Id(id)) {}
    }

    /// EventGroup method
    async fn event_group(
        &self,
        events: Vec<(i32, &str, zbus::zvariant::Value<'_>, u32)>,
    ) -> Vec<i32> {
        todo!()
    }

    /// GetGroupProperties method
    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<&str>,
    ) -> Vec<(
        i32,
        std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    )> {
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
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
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
        ctx: &SignalContext<'_>,
        id: i32,
        timestamp: u32,
    ) -> zbus::Result<()>;

    /// ItemsPropertiesUpdated signal
    #[dbus_interface(signal)]
    async fn items_properties_updated(
        ctx: &SignalContext<'_>,
        updated_props: &[(
            i32,
            std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
        )],
        removed_props: &[(i32, &[&str])],
    ) -> zbus::Result<()>;

    /// LayoutUpdated signal
    #[dbus_interface(signal)]
    async fn layout_updated(
        ctx: &SignalContext<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;

    /// IconThemePath property
    #[dbus_interface(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        todo!()
    }

    /// Status property
    #[dbus_interface(property)]
    fn status(&self) -> String {
        todo!()
    }

    /// TextDirection property
    #[dbus_interface(property)]
    fn text_direction(&self) -> String {
        todo!()
    }

    /// Version property
    #[dbus_interface(property)]
    fn version(&self) -> u32 {
        todo!()
    }
}
