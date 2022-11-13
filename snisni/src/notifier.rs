use crate::{dbus, menu::Menu, Event, Item, Status, ITEM_OBJECT_PATH, MENU_OBJECT_PATH};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::{
    future::Future,
    hash::{Hash, Hasher},
};
use zbus::names::WellKnownName;

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
