use std::{fmt::Debug, future::Future, hash::Hash};

pub mod sni;
pub mod watcher;

//mod dbus;
//pub mod menu;

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

impl<Ev, F> OnEvent<Ev> for F
where
    F: Fn(Ev) -> Box<dyn Future<Output = ()> + Send> + Send + Sync,
{
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        (self)(event)
    }
}

#[cfg(feature = "tokio")]
impl<Ev: Debug + Send + 'static> OnEvent<Ev> for tokio::sync::mpsc::Sender<Ev> {
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        let send = self.clone();
        Box::new(async move {
            send.send(event).await.expect("channel to not be closed");
        })
    }
}

#[cfg(all(feature = "tokio"))]
impl<Ev: Debug + Send + 'static> OnEvent<Ev> for tokio::sync::mpsc::UnboundedSender<Ev> {
    fn on_event(&self, event: Ev) -> Box<dyn Future<Output = ()> + Send> {
        let send = self.clone();
        Box::new(async move {
            send.send(event).expect("channel to not be closed");
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
