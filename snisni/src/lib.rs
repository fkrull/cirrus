use std::future::Future;
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
pub struct Item {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Menu {
    // TODO
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
    // TODO menu event
}

const ITEM_OBJECT_PATH: &str = "/StatusNotifierItem";
const MENU_OBJECT_PATH: &str = "/StatusNotifierItem/Menu";

pub async fn run<F, Fut>(
    id: u32,
    item: Item,
    menu: Menu,
    on_event: F,
) -> zbus::Result<zbus::Connection>
where
    F: Fn(Event) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send,
{
    let name = format!("org.kde.StatusNotifierItem-{}-{}", std::process::id(), id);
    let conn = zbus::ConnectionBuilder::session()?
        .name(WellKnownName::try_from(name.as_str())?)?
        .serve_at(
            ITEM_OBJECT_PATH,
            dbus::StatusNotifierItem {
                model: item,
                on_event,
            },
        )?
        .serve_at(MENU_OBJECT_PATH, dbus::DBusMenu { model: menu })?
        .build()
        .await?;
    let watcher = dbus::StatusNotifierWatcherProxy::new(&conn).await?;
    watcher.register_status_notifier_item(&name).await?;
    Ok(conn)
}
