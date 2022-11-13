use crate::{menu, Event, Item, OnEvent, Pixmap, ScrollOrientation, MENU_OBJECT_PATH};
use zbus::{dbus_interface, dbus_proxy, SignalContext};

/// DBus interface proxy for `org.kde.StatusNotifierWatcher`
#[dbus_proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
pub(crate) trait StatusNotifierWatcher {
    /// RegisterStatusNotifierHost method
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;

    /// RegisterStatusNotifierItem method
    fn register_status_notifier_item(&self, service: &str) -> zbus::Result<()>;

    /// StatusNotifierHostRegistered signal
    #[dbus_proxy(signal)]
    fn status_notifier_host_registered(&self) -> zbus::Result<()>;

    /// StatusNotifierHostUnregistered signal
    #[dbus_proxy(signal)]
    fn status_notifier_host_unregistered(&self) -> zbus::Result<()>;

    /// StatusNotifierItemRegistered signal
    #[dbus_proxy(signal)]
    fn status_notifier_item_registered(&self, arg_1: &str) -> zbus::Result<()>;

    /// StatusNotifierItemUnregistered signal
    #[dbus_proxy(signal)]
    fn status_notifier_item_unregistered(&self, arg_1: &str) -> zbus::Result<()>;

    /// IsStatusNotifierHostRegistered property
    #[dbus_proxy(property)]
    fn is_status_notifier_host_registered(&self) -> zbus::Result<bool>;

    /// ProtocolVersion property
    #[dbus_proxy(property)]
    fn protocol_version(&self) -> zbus::Result<i32>;

    /// RegisteredStatusNotifierItems property
    #[dbus_proxy(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;
}

pub(crate) struct StatusNotifierItem<Ev> {
    pub(crate) model: Item,
    pub(crate) on_event: Box<dyn OnEvent<Ev>>,
}

fn convert_pixmap(p: &Pixmap) -> (i32, i32, &[u8]) {
    (p.width, p.height, &p.data)
}

impl<Ev> StatusNotifierItem<Ev> {
    async fn on_event(&self, event: Event<Ev>) {
        let pinned = Box::into_pin(self.on_event.on_event(event));
        pinned.await;
    }
}

#[dbus_interface(interface = "org.kde.StatusNotifierItem")]
impl<Ev: Send + 'static> StatusNotifierItem<Ev> {
    /// Activate method
    async fn activate(&self, x: i32, y: i32) {
        self.on_event(Event::Activate { x, y }).await;
    }

    /// ContextMenu method
    async fn context_menu(&self, x: i32, y: i32) {
        self.on_event(Event::ContextMenu { x, y }).await;
    }

    /// Scroll method
    async fn scroll(&self, delta: i32, orientation: &str) -> Result<(), zbus::fdo::Error> {
        match ScrollOrientation::try_from(orientation) {
            Ok(orientation) => {
                self.on_event(Event::Scroll { delta, orientation }).await;
                Ok(())
            }
            Err(value) => Err(zbus::fdo::Error::InvalidArgs(value.to_string())),
        }
    }

    /// SecondaryActivate method
    async fn secondary_activate(&self, x: i32, y: i32) {
        self.on_event(Event::SecondaryActivate { x, y }).await;
    }

    /// NewAttentionIcon signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_attention_icon(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewIcon signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_icon(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewOverlayIcon signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_overlay_icon(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewStatus signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_status(ctx: &SignalContext<'_>, status: &str) -> zbus::Result<()>;

    /// NewTitle signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_title(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewToolTip signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_tool_tip(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// AttentionIconName property
    #[dbus_interface(property)]
    fn attention_icon_name(&self) -> String {
        self.model.attention_icon.name.clone()
    }

    /// AttentionIconPixmap property
    #[dbus_interface(property)]
    fn attention_icon_pixmap(&self) -> Vec<(i32, i32, &[u8])> {
        self.model
            .attention_icon
            .pixmaps
            .iter()
            .map(convert_pixmap)
            .collect()
    }

    /// AttentionMovieName property
    #[dbus_interface(property)]
    fn attention_movie_name(&self) -> &str {
        &self.model.attention_movie_name
    }

    /// Category property
    #[dbus_interface(property)]
    fn category(&self) -> &str {
        self.model.category.into()
    }

    /// IconName property
    #[dbus_interface(property)]
    fn icon_name(&self) -> &str {
        &self.model.icon.name
    }

    /// IconPixmap property
    #[dbus_interface(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, &[u8])> {
        self.model.icon.pixmaps.iter().map(convert_pixmap).collect()
    }

    /// IconThemePath property
    #[dbus_interface(property)]
    fn icon_theme_path(&self) -> &str {
        &self.model.icon_theme_path
    }

    /// Id property
    #[dbus_interface(property)]
    fn id(&self) -> &str {
        &self.model.id
    }

    /// ItemIsMenu property
    #[dbus_interface(property)]
    fn item_is_menu(&self) -> bool {
        self.model.item_is_menu
    }

    /// Menu property
    #[dbus_interface(property)]
    fn menu(&self) -> zbus::zvariant::OwnedObjectPath {
        zbus::zvariant::OwnedObjectPath::try_from(MENU_OBJECT_PATH)
            .expect("constant string to be a valid path")
    }

    /// OverlayIconName property
    #[dbus_interface(property)]
    fn overlay_icon_name(&self) -> &str {
        &self.model.overlay_icon.name
    }

    /// OverlayIconPixmap property
    #[dbus_interface(property)]
    fn overlay_icon_pixmap(&self) -> Vec<(i32, i32, &[u8])> {
        self.model
            .overlay_icon
            .pixmaps
            .iter()
            .map(convert_pixmap)
            .collect()
    }

    /// Status property
    #[dbus_interface(property)]
    fn status(&self) -> &str {
        self.model.status.into()
    }

    /// Title property
    #[dbus_interface(property)]
    fn title(&self) -> &str {
        &self.model.title
    }

    /// ToolTip property
    #[dbus_interface(property)]
    fn tool_tip(&self) -> (&str, Vec<(i32, i32, &[u8])>, &str, &str) {
        (
            &self.model.tooltip.icon.name,
            self.model
                .tooltip
                .icon
                .pixmaps
                .iter()
                .map(convert_pixmap)
                .collect(),
            &self.model.tooltip.title,
            &self.model.tooltip.text,
        )
    }

    /// WindowId property
    #[dbus_interface(property)]
    fn window_id(&self) -> i32 {
        self.model.window_id
    }
}

#[derive(Debug)]
pub(crate) struct DBusMenu<Ev> {
    pub(crate) model: menu::Menu<Ev>,
    pub(crate) revision: u32,
}

#[dbus_interface(interface = "com.canonical.dbusmenu")]
impl<Ev: Send + Sync + 'static> DBusMenu<Ev> {
    /// AboutToShow method
    async fn about_to_show(&self, id: i32) -> bool {
        todo!()
    }

    /// AboutToShowGroup method
    async fn about_to_show_group(&self, ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        todo!()
    }

    /// Event method
    async fn event(
        &self,
        id: i32,
        event_id: &str,
        data: zbus::zvariant::Value<'_>,
        timestamp: u32,
    ) {
        todo!()
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
