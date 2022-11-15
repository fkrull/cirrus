use std::marker::PhantomData;
use std::{fmt::Debug, future::Future, hash::Hash};

pub mod menu;
pub mod menubuilder;
pub mod sni;
pub mod watcher;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SniName {
    pid: u32,
    app_internal_id: u32,
}

impl SniName {
    pub fn new(app_internal_id: u32) -> SniName {
        SniName {
            pid: std::process::id(),
            app_internal_id,
        }
    }
}

impl From<SniName> for String {
    fn from(v: SniName) -> Self {
        format!("org.kde.StatusNotifierItem-{}-{}", v.pid, v.app_internal_id)
    }
}

impl From<SniName> for zbus::names::WellKnownName<'static> {
    fn from(v: SniName) -> Self {
        zbus::names::WellKnownName::try_from(String::from(v)).expect("valid name")
    }
}

pub trait OnEvent<Ev>: Send + Sync {
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send>;
}

#[derive(Debug, Copy, Clone)]
pub struct DiscardEvents;

impl<Ev> OnEvent<Ev> for DiscardEvents {
    fn on_event(&self, _event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        Box::new(async move {})
    }
}

impl<Ev, F, Fut> OnEvent<Ev> for F
where
    F: Fn(Ev) -> Fut + Send + Sync,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        let fut = (self)(event);
        Box::new(fut)
    }
}

#[cfg(feature = "tokio")]
impl<T, Ev> OnEvent<Ev> for tokio::sync::mpsc::Sender<T>
where
    Ev: Send + 'static,
    T: From<Ev> + Debug + Send + 'static,
{
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        let send = self.clone();
        Box::new(async move {
            send.send(event.into())
                .await
                .expect("channel to not be closed");
        })
    }
}

#[cfg(all(feature = "tokio"))]
impl<T, Ev> OnEvent<Ev> for tokio::sync::mpsc::UnboundedSender<T>
where
    Ev: Send + 'static,
    T: From<Ev> + Debug + Send + 'static,
{
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        let send = self.clone();
        Box::new(async move {
            send.send(event.into()).expect("channel to not be closed");
        })
    }
}

pub const ITEM_OBJECT_PATH: &str = "/StatusNotifierItem";
pub const MENU_OBJECT_PATH: &str = "/StatusNotifierItem/Menu";

struct Hasher(fnv::FnvHasher);

impl Hasher {
    const INITIAL_KEY: u64 = 7581889071078416883;

    fn new() -> Hasher {
        Hasher(fnv::FnvHasher::with_key(Hasher::INITIAL_KEY))
    }

    fn hash(&mut self, v: impl Hash) -> u64 {
        use std::hash::Hasher as _;
        v.hash(&mut self.0);
        self.0.finish()
    }
}

#[derive(Debug)]
pub struct Handle<M> {
    name: SniName,
    conn: zbus::Connection,
    _m: PhantomData<M>,
}

impl<M: Clone + Send + Sync + 'static> Handle<M> {
    pub async fn new_with_connection(
        name: SniName,
        model: sni::Model,
        menu_model: menu::Model<M>,
        on_event: Box<dyn OnEvent<sni::Event>>,
        on_menu_event: Box<dyn OnEvent<menu::Event<M>>>,
        conn_builder: zbus::ConnectionBuilder<'_>,
    ) -> zbus::Result<Self> {
        let conn = conn_builder
            .name(String::from(name))?
            .serve_at(
                ITEM_OBJECT_PATH,
                sni::StatusNotifierItem::new(model, on_event),
            )?
            .serve_at(
                MENU_OBJECT_PATH,
                menu::DBusMenu::new(menu_model, on_menu_event),
            )?
            .build()
            .await?;
        Ok(Handle {
            name,
            conn,
            _m: PhantomData,
        })
    }

    pub async fn new(
        name: SniName,
        model: sni::Model,
        menu_model: menu::Model<M>,
        on_event: Box<dyn OnEvent<sni::Event>>,
        on_menu_event: Box<dyn OnEvent<menu::Event<M>>>,
    ) -> zbus::Result<Self> {
        Handle::new_with_connection(
            name,
            model,
            menu_model,
            on_event,
            on_menu_event,
            zbus::ConnectionBuilder::session()?,
        )
        .await
    }

    pub async fn register(&self) -> zbus::Result<()> {
        let watcher = watcher::StatusNotifierWatcherProxy::new(&self.conn).await?;
        watcher
            .register_status_notifier_item(&String::from(self.name))
            .await?;
        Ok(())
    }

    pub async fn update(&self, f: impl FnOnce(&mut sni::Model)) -> zbus::Result<()> {
        let object_server = self.conn.object_server();
        let iface = object_server
            .interface::<_, sni::StatusNotifierItem>(ITEM_OBJECT_PATH)
            .await?;
        iface
            .get_mut()
            .await
            .update(iface.signal_context(), f)
            .await?;
        Ok(())
    }

    pub async fn update_menu(&self, f: impl FnOnce(&mut menu::Model<M>)) -> zbus::Result<()> {
        let object_server = self.conn.object_server();
        let iface = object_server
            .interface::<_, menu::DBusMenu<M>>(MENU_OBJECT_PATH)
            .await?;
        iface
            .get_mut()
            .await
            .update(iface.signal_context(), f)
            .await?;
        Ok(())
    }
}
