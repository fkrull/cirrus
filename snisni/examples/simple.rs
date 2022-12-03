use snisni::*;
use zbus::names::{BusName, OwnedWellKnownName};

fn main() {
    let name = OwnedWellKnownName::try_from("io.github.fkrull.snisni-example-simple").unwrap();
    let model = sni::Model {
        icon: sni::Icon {
            name: "network-cellular-5g-symbolic".to_string(),
            pixmaps: vec![],
        },
        id: "simple".to_string(),
        title: "Basic example".to_string(),
        ..Default::default()
    };
    let conn = zbus::blocking::ConnectionBuilder::session()
        .unwrap()
        .name(&name)
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
    watcher.register_loop(&BusName::from(&name)).unwrap();
}
