use snisni::*;
use std::time::Duration;

fn main() {
    let name = SniName::new(1);
    let model = sni::Model {
        icon: sni::Icon {
            name: "network-cellular-5g-symbolic".to_string(),
            pixmaps: vec![],
        },
        id: "simple-example".to_string(),
        title: "Basic example".to_string(),
        ..Default::default()
    };
    let conn = zbus::blocking::ConnectionBuilder::session()
        .unwrap()
        .name(name)
        .unwrap()
        .serve_at(
            ITEM_OBJECT_PATH,
            sni::StatusNotifierItem::new(
                model,
                Box::new(|ev| async move { println!("ev={ev:?}") }),
            ),
        )
        .unwrap()
        .build()
        .unwrap();
    let watcher = watcher::StatusNotifierWatcherProxyBlocking::new(&conn).unwrap();
    watcher
        .register_status_notifier_item(&String::from(name))
        .unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
