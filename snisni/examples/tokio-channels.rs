use snisni::*;

#[derive(Debug)]
enum Event {
    Notifier(sni::Event),
    Menu(u32),
}

impl From<sni::Event> for Event {
    fn from(ev: sni::Event) -> Self {
        Event::Notifier(ev)
    }
}

impl From<menu::Event<u32>> for Event {
    fn from(ev: menu::Event<u32>) -> Self {
        Event::Menu(ev.message)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let name = SniName::new(1);
    let model = sni::Model {
        id: "tokio-channels".to_string(),
        title: "Tokio channels example".to_string(),
        ..Default::default()
    };
    let menu_model = menu::Model {
        items: menubuilder::MenuBuilder::new_with_item(Default::default())
            .standard_item("Item 1", 1)
            .standard_item("Item 2", 2)
            .standard_item("Item 3", 3)
            .separator()
            .standard_item("Exit", 66)
            .build(),
        ..Default::default()
    };
    let conn = zbus::ConnectionBuilder::session()
        .unwrap()
        .name(name)
        .unwrap()
        .serve_at(
            ITEM_OBJECT_PATH,
            sni::StatusNotifierItem::new(model, Box::new(send.clone())),
        )
        .unwrap()
        .serve_at(
            MENU_OBJECT_PATH,
            menu::DBusMenu::new(menu_model, Box::new(send)),
        )
        .unwrap()
        .build()
        .await
        .unwrap();
    let watcher = watcher::StatusNotifierWatcherProxy::new(&conn)
        .await
        .unwrap();
    watcher
        .register_status_notifier_item(&String::from(name))
        .await
        .unwrap();

    while let Some(event) = recv.recv().await {
        println!("event={event:?}");
        if let Event::Menu(66) = event {
            break;
        }
    }
}
