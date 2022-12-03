use crate::OnEvent;
use zbus::zvariant::OwnedObjectPath;
use zbus::{
    zvariant::{Str, Structure, Value},
    SignalContext,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, zbus::zvariant::Type)]
#[zvariant(signature = "s")]
pub enum Category {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
}

impl From<Category> for Value<'static> {
    fn from(v: Category) -> Self {
        let s = match v {
            Category::ApplicationStatus => "ApplicationStatus",
            Category::Communications => "Communications",
            Category::SystemServices => "SystemServices",
            Category::Hardware => "Hardware",
        };
        Value::from(Str::from(s))
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
#[zvariant(signature = "s")]
pub enum Status {
    Passive,
    Active,
    NeedsAttention,
}

impl From<Status> for Value<'static> {
    fn from(v: Status) -> Self {
        let s = match v {
            Status::Passive => "Passive",
            Status::Active => "Active",
            Status::NeedsAttention => "NeedsAttention",
        };
        Value::from(Str::from(s))
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Default, zbus::zvariant::Type, zbus::zvariant::Value,
)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    /// Image data in ARGB32 format in network byte order.
    pub data: Vec<u8>,
}

impl<'a> From<&'a Pixmap> for Value<'a> {
    fn from(p: &'a Pixmap) -> Self {
        Value::from(Structure::from((p.width, p.height, &p.data)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Icon {
    pub name: String,
    pub pixmaps: Vec<Pixmap>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Tooltip {
    pub title: String,
    pub text: String,
    pub icon: Icon,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Model {
    pub icon: Icon,
    pub overlay_icon: Icon,
    pub attention_icon: Icon,
    pub attention_movie_name: String,
    pub icon_theme_path: String,

    pub id: String,
    pub title: String,
    pub tooltip: Tooltip,
    pub category: Category,
    pub status: Status,
    pub window_id: i32,
    pub item_is_menu: bool,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            icon: Icon::default(),
            overlay_icon: Icon::default(),
            attention_icon: Icon::default(),
            attention_movie_name: String::new(),
            icon_theme_path: String::new(),
            id: String::new(),
            title: String::new(),
            tooltip: Tooltip::default(),
            category: Category::ApplicationStatus,
            status: Status::Active,
            window_id: 0,
            item_is_menu: false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Deserialize, zbus::zvariant::Type)]
#[serde(rename_all = "lowercase")]
#[zvariant(signature = "s")]
pub enum ScrollOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    Activate {
        x: i32,
        y: i32,
    },
    ContextMenu {
        x: i32,
        y: i32,
    },
    Scroll {
        delta: i32,
        orientation: ScrollOrientation,
    },
    SecondaryActivate {
        x: i32,
        y: i32,
    },
}

struct Hashes {
    attention_icon: u64,
    icon: u64,
    overlay_icon: u64,
    status: u64,
    status_value: Status,
    title: u64,
    tooltip: u64,
}

impl Hashes {
    fn new(model: &Model) -> Hashes {
        let mut hasher = crate::Hasher::new();
        let Model {
            icon,
            overlay_icon,
            attention_icon,
            attention_movie_name,
            icon_theme_path: _,
            id: _,
            title,
            tooltip,
            category: _,
            status,
            window_id: _,
            item_is_menu: _,
        } = model;
        Hashes {
            attention_icon: hasher.hash((attention_icon, attention_movie_name)),
            icon: hasher.hash(icon),
            overlay_icon: hasher.hash(overlay_icon),
            status: hasher.hash(status),
            status_value: *status,
            title: hasher.hash(title),
            tooltip: hasher.hash(tooltip),
        }
    }
}

async fn signal_changes(ctx: &SignalContext<'_>, old: &Hashes, new: &Hashes) -> zbus::Result<()> {
    if old.attention_icon != new.attention_icon {
        StatusNotifierItem::new_attention_icon(ctx).await?;
    }
    if old.icon != new.icon {
        StatusNotifierItem::new_icon(ctx).await?;
    }
    if old.overlay_icon != new.overlay_icon {
        StatusNotifierItem::new_overlay_icon(ctx).await?;
    }
    if old.status != new.status {
        StatusNotifierItem::new_status(ctx, new.status_value).await?;
    }
    if old.title != new.title {
        StatusNotifierItem::new_title(ctx).await?;
    }
    if old.tooltip != new.tooltip {
        StatusNotifierItem::new_tool_tip(ctx).await?;
    }
    Ok(())
}

pub struct StatusNotifierItem {
    model: Model,
    menu: OwnedObjectPath,
    on_event: Box<dyn OnEvent<Event>>,
}

impl std::fmt::Debug for StatusNotifierItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusNotifierItem")
            .field("model", &self.model)
            .field("on_event", &"OnEvent { .. }")
            .finish()
    }
}

impl StatusNotifierItem {
    pub fn new(model: Model, on_event: Box<dyn OnEvent<Event>>) -> StatusNotifierItem {
        let menu = OwnedObjectPath::try_from(crate::MENU_OBJECT_PATH)
            .expect("constant string to be a valid path");
        StatusNotifierItem::new_with_menu(model, menu, on_event)
    }

    pub fn new_with_menu(
        model: Model,
        menu: OwnedObjectPath,
        on_event: Box<dyn OnEvent<Event>>,
    ) -> StatusNotifierItem {
        StatusNotifierItem {
            model,
            menu,
            on_event,
        }
    }

    pub async fn update(
        &mut self,
        ctx: &SignalContext<'_>,
        f: impl FnOnce(&mut Model),
    ) -> zbus::Result<()> {
        let (old, new) = {
            let old = Hashes::new(&self.model);
            f(&mut self.model);
            let new = Hashes::new(&self.model);
            (old, new)
        };
        signal_changes(ctx, &old, &new).await?;
        Ok(())
    }

    async fn on_event(&self, event: Event) {
        let pinned = Box::into_pin(self.on_event.on_event(event));
        pinned.await;
    }
}

#[zbus::dbus_interface(interface = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    /// Activate method
    async fn activate(&self, x: i32, y: i32) {
        self.on_event(Event::Activate { x, y }).await;
    }

    /// ContextMenu method
    async fn context_menu(&self, x: i32, y: i32) {
        self.on_event(Event::ContextMenu { x, y }).await;
    }

    /// Scroll method
    async fn scroll(&self, delta: i32, orientation: ScrollOrientation) {
        self.on_event(Event::Scroll { delta, orientation }).await;
    }

    /// SecondaryActivate method
    async fn secondary_activate(&self, x: i32, y: i32) {
        self.on_event(Event::SecondaryActivate { x, y }).await;
    }

    /// NewAttentionIcon signal
    #[dbus_interface(signal)]
    pub async fn new_attention_icon(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewIcon signal
    #[dbus_interface(signal)]
    pub async fn new_icon(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewOverlayIcon signal
    #[dbus_interface(signal)]
    pub async fn new_overlay_icon(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewStatus signal
    #[dbus_interface(signal)]
    pub async fn new_status(ctx: &SignalContext<'_>, status: Status) -> zbus::Result<()>;

    /// NewTitle signal
    #[dbus_interface(signal)]
    pub async fn new_title(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// NewToolTip signal
    #[dbus_interface(signal)]
    pub async fn new_tool_tip(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    /// AttentionIconName property
    #[dbus_interface(property)]
    fn attention_icon_name(&self) -> String {
        self.model.attention_icon.name.clone()
    }

    /// AttentionIconPixmap property
    #[dbus_interface(property)]
    fn attention_icon_pixmap(&self) -> Vec<&Pixmap> {
        self.model.attention_icon.pixmaps.iter().collect()
    }

    /// AttentionMovieName property
    #[dbus_interface(property)]
    fn attention_movie_name(&self) -> &str {
        &self.model.attention_movie_name
    }

    /// Category property
    #[dbus_interface(property)]
    fn category(&self) -> Category {
        self.model.category
    }

    /// IconName property
    #[dbus_interface(property)]
    fn icon_name(&self) -> &str {
        &self.model.icon.name
    }

    /// IconPixmap property
    #[dbus_interface(property)]
    fn icon_pixmap(&self) -> Vec<&Pixmap> {
        self.model.icon.pixmaps.iter().collect()
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
    fn menu(&self) -> zbus::zvariant::ObjectPath {
        self.menu.as_ref()
    }

    /// OverlayIconName property
    #[dbus_interface(property)]
    fn overlay_icon_name(&self) -> &str {
        &self.model.overlay_icon.name
    }

    /// OverlayIconPixmap property
    #[dbus_interface(property)]
    fn overlay_icon_pixmap(&self) -> Vec<&Pixmap> {
        self.model.overlay_icon.pixmaps.iter().collect()
    }

    /// Status property
    #[dbus_interface(property)]
    fn status(&self) -> Status {
        self.model.status
    }

    /// Title property
    #[dbus_interface(property)]
    fn title(&self) -> &str {
        &self.model.title
    }

    /// ToolTip property
    #[dbus_interface(property)]
    fn tool_tip(&self) -> (&str, Vec<&Pixmap>, &str, &str) {
        (
            &self.model.tooltip.icon.name,
            self.model.tooltip.icon.pixmaps.iter().collect(),
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
