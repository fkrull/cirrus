use zbus::names::WellKnownName;

mod dbus;

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
    // TODO type
    // pub menu: Menu,
}

pub async fn run(id: u32, model: Model) -> zbus::Result<zbus::Connection> {
    let name = format!("org.kde.StatusNotifierItem-{}-{}", std::process::id(), id);
    let conn = zbus::ConnectionBuilder::session()?
        .name(WellKnownName::try_from(name.as_str())?)?
        .serve_at("/StatusNotifierItem", dbus::StatusNotifierItem { model })?
        .build()
        .await?;
    let watcher = dbus::StatusNotifierWatcherProxy::new(&conn).await?;
    watcher.register_status_notifier_item(&name).await?;
    Ok(conn)
}
