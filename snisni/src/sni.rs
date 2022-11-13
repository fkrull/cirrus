use crate::OnEvent;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
}

impl From<Category> for &'static str {
    fn from(v: Category) -> Self {
        match v {
            Category::ApplicationStatus => "ApplicationStatus",
            Category::Communications => "Communications",
            Category::SystemServices => "SystemServices",
            Category::Hardware => "Hardware",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Passive,
    Active,
    NeedsAttention,
}

impl From<Status> for &'static str {
    fn from(v: Status) -> Self {
        match v {
            Status::Passive => "Passive",
            Status::Active => "Active",
            Status::NeedsAttention => "NeedsAttention",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    /// Image data in ARGB32 format in network byte order.
    pub data: Vec<u8>,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ScrollOrientation {
    Horizontal,
    Vertical,
}

impl<'a> TryFrom<&'a str> for ScrollOrientation {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.eq_ignore_ascii_case("horizontal") {
            Ok(ScrollOrientation::Horizontal)
        } else if value.eq_ignore_ascii_case("vertical") {
            Ok(ScrollOrientation::Vertical)
        } else {
            Err(value)
        }
    }
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

async fn signal_changes(
    ctx: &zbus::SignalContext<'_>,
    old: &Hashes,
    new: &Hashes,
) -> zbus::Result<()> {
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
        StatusNotifierItem::new_status(ctx, new.status_value.into()).await?;
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
        StatusNotifierItem { model, on_event }
    }

    pub async fn update(
        &mut self,
        ctx: &zbus::SignalContext<'_>,
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

fn convert_pixmap(p: &Pixmap) -> (i32, i32, &[u8]) {
    (p.width, p.height, &p.data)
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
    pub(crate) async fn new_attention_icon(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    /// NewIcon signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_icon(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    /// NewOverlayIcon signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_overlay_icon(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    /// NewStatus signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_status(ctx: &zbus::SignalContext<'_>, status: &str)
        -> zbus::Result<()>;

    /// NewTitle signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_title(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    /// NewToolTip signal
    #[dbus_interface(signal)]
    pub(crate) async fn new_tool_tip(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

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
        zbus::zvariant::OwnedObjectPath::try_from(crate::MENU_OBJECT_PATH)
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
