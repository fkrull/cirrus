use futures::StreamExt;
use zbus::names::BusName;

/// DBus interface proxy for `org.kde.StatusNotifierWatcher`
#[zbus::dbus_proxy(
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
    fn status_notifier_item_registered(&self, service: &str) -> zbus::Result<()>;

    /// StatusNotifierItemUnregistered signal
    #[dbus_proxy(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> zbus::Result<()>;

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

impl<'a> StatusNotifierWatcherProxy<'a> {
    pub async fn register_loop(&self, name: &BusName<'_>) -> zbus::Result<()> {
        let name = name.to_string();
        if let Err(error) = self.register_status_notifier_item(&name).await {
            tracing::debug!(%error, "failed to initially register StatusNotifierItem");
        }
        let mut owner_changed = self.receive_owner_changed().await?;
        while let Some(new_name) = owner_changed.next().await {
            if let Some(new_name) = new_name {
                tracing::debug!(%new_name, "StatusNotifierWatcher owner changed");
                self.register_status_notifier_item(&name).await?;
            } else {
                tracing::debug!("StatusNotifierWatcher disappeared")
            }
        }
        Ok(())
    }
}

impl<'a> StatusNotifierWatcherProxyBlocking<'a> {
    pub fn register_loop(&self, name: &BusName<'_>) -> zbus::Result<()> {
        let name = name.to_string();
        if let Err(error) = self.register_status_notifier_item(&name) {
            tracing::debug!(%error, "failed to initially register StatusNotifierItem");
        }
        for new_name in self.receive_owner_changed()? {
            if let Some(new_name) = new_name {
                tracing::debug!(%new_name, "StatusNotifierWatcher owner changed");
                self.register_status_notifier_item(&name)?;
            } else {
                tracing::debug!("StatusNotifierWatcher disappeared")
            }
        }
        Ok(())
    }
}
