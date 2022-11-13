use crate::{dbus, menu::Menu, Event, Item, Status, ITEM_OBJECT_PATH, MENU_OBJECT_PATH};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::{
    future::Future,
    hash::{Hash, Hasher},
};
use zbus::names::WellKnownName;

const INITIAL_KEY: u64 = 7581889071078416883;

fn hash(v: impl Hash, hasher: &mut impl Hasher) -> u64 {
    v.hash(hasher);
    hasher.finish()
}

#[derive(Debug)]
struct ItemHashes {
    attention_icon: u64,
    icon: u64,
    overlay_icon: u64,
    status: u64,
    status_value: Status,
    title: u64,
    tooltip: u64,
}

impl ItemHashes {
    fn hash(item: &Item) -> ItemHashes {
        let mut hasher = fnv::FnvHasher::with_key(INITIAL_KEY);
        let Item {
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
        } = item;
        ItemHashes {
            attention_icon: hash((attention_icon, attention_movie_name), &mut hasher),
            icon: hash(icon, &mut hasher),
            overlay_icon: hash(overlay_icon, &mut hasher),
            status: hash(status, &mut hasher),
            status_value: *status,
            title: hash(title, &mut hasher),
            tooltip: hash(tooltip, &mut hasher),
        }
    }
}

async fn signal_updates<Ev: Send + 'static>(
    ctx: &zbus::SignalContext<'_>,
    old: &ItemHashes,
    new: &ItemHashes,
) -> zbus::Result<()> {
    if old.attention_icon != new.attention_icon {
        dbus::StatusNotifierItem::<Ev>::new_attention_icon(ctx).await?;
    }
    if old.icon != new.icon {
        dbus::StatusNotifierItem::<Ev>::new_icon(ctx).await?;
    }
    if old.overlay_icon != new.overlay_icon {
        dbus::StatusNotifierItem::<Ev>::new_overlay_icon(ctx).await?;
    }
    if old.status != new.status {
        dbus::StatusNotifierItem::<Ev>::new_status(ctx, new.status_value.into()).await?;
    }
    if old.title != new.title {
        dbus::StatusNotifierItem::<Ev>::new_title(ctx).await?;
    }
    if old.tooltip != new.tooltip {
        dbus::StatusNotifierItem::<Ev>::new_tool_tip(ctx).await?;
    }
    Ok(())
}

pub trait OnEvent<Ev>: Send + Sync {
    fn on_event(&self, event: Event<Ev>) -> Box<dyn Future<Output = ()> + Send>;
}

impl<Ev, F> OnEvent<Ev> for F
where
    F: Fn(Event<Ev>) -> Box<dyn Future<Output = ()> + Send> + Send + Sync,
{
    fn on_event(&self, event: Event<Ev>) -> Box<dyn Future<Output = ()> + Send> {
        (self)(event)
    }
}

#[cfg(all(feature = "tokio"))]
impl<Ev: Debug + Send + 'static> OnEvent<Ev> for tokio::sync::mpsc::Sender<Event<Ev>> {
    fn on_event(&self, event: Event<Ev>) -> Box<dyn Future<Output = ()> + Send> {
        let send = self.clone();
        Box::new(async move {
            send.send(event).await.expect("channel to not be closed");
        })
    }
}

#[cfg(all(feature = "tokio"))]
impl<Ev: Debug + Send + 'static> OnEvent<Ev> for tokio::sync::mpsc::UnboundedSender<Event<Ev>> {
    fn on_event(&self, event: Event<Ev>) -> Box<dyn Future<Output = ()> + Send> {
        let send = self.clone();
        Box::new(async move {
            send.send(event).expect("channel to not be closed");
        })
    }
}

#[derive(Debug)]
pub struct StatusNotifier<Ev> {
    name: String,
    conn: zbus::Connection,
    _ev: PhantomData<Ev>,
}

impl<Ev: Send + Sync + 'static> StatusNotifier<Ev> {
    // TODO error type
    pub async fn new(
        app_internal_id: u32,
        item: Item,
        menu: Menu<Ev>,
        on_event: Box<dyn OnEvent<Ev>>,
    ) -> zbus::Result<StatusNotifier<Ev>> {
        StatusNotifier::new_with_connection_internal(
            zbus::ConnectionBuilder::session()?,
            app_internal_id,
            item,
            menu,
            on_event,
        )
        .await
    }

    // TODO error type
    async fn new_with_connection_internal(
        mut connection_builder: zbus::ConnectionBuilder<'_>,
        app_internal_id: u32,
        item: Item,
        menu: Menu<Ev>,
        on_event: Box<dyn OnEvent<Ev>>,
    ) -> zbus::Result<StatusNotifier<Ev>> {
        let name = format!(
            "org.kde.StatusNotifierItem-{}-{}",
            std::process::id(),
            app_internal_id
        );
        let conn = connection_builder
            .name(WellKnownName::try_from(name.as_str())?)?
            .serve_at(
                ITEM_OBJECT_PATH,
                dbus::StatusNotifierItem {
                    model: item,
                    on_event,
                },
            )?
            .serve_at(
                MENU_OBJECT_PATH,
                dbus::DBusMenu {
                    model: menu,
                    revision: 0,
                },
            )?
            .build()
            .await?;
        Ok(StatusNotifier {
            name,
            conn,
            _ev: PhantomData,
        })
    }

    // TODO error type
    #[cfg(feature = "zbus-api")]
    pub async fn new_with_connection(
        mut connection_builder: zbus::ConnectionBuilder<'_>,
        app_internal_id: u32,
        item: Item,
        menu: Menu<Ev>,
        on_event: Box<dyn OnEvent<Ev>>,
    ) -> zbus::Result<StatusNotifier<Ev>> {
        StatusNotifier::new_with_connection_internal(
            connection_builder,
            app_internal_id,
            item,
            menu,
            on_event,
        )
        .await
    }

    // TODO error type
    pub async fn register(&self) -> zbus::Result<()> {
        let watcher = dbus::StatusNotifierWatcherProxy::new(&self.conn).await?;
        watcher.register_status_notifier_item(&self.name).await?;
        Ok(())
    }

    #[cfg(feature = "zbus-api")]
    pub fn connection(&self) -> &zbus::Connection {
        &self.conn
    }

    pub async fn update_item(&self, f: impl FnOnce(&mut Item)) -> zbus::Result<()> {
        let object_server = self.conn.object_server();
        let iface = object_server
            .interface::<'_, _, dbus::StatusNotifierItem<Ev>>(ITEM_OBJECT_PATH)
            .await?;
        let (old, new) = {
            let mut item = iface.get_mut().await;
            let old = ItemHashes::hash(&item.model);
            f(&mut item.model);
            let new = ItemHashes::hash(&item.model);
            (old, new)
        };
        signal_updates::<Ev>(iface.signal_context(), &old, &new).await?;
        Ok(())
    }
}
