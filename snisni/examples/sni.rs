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

fn notifier(activated: bool) -> sni::Model {
    let icon = if activated {
        "folder-download"
    } else {
        "folder"
    };
    sni::Model {
        icon: sni::Icon {
            name: icon.to_string(),
            pixmaps: Vec::new(),
        },
        id: "snisni-test".to_string(),
        title: "StatusNotifierItem example".to_string(),
        ..Default::default()
    }
}

fn menu(activated: bool) -> menu::Model<u32> {
    let items = if activated {
        menubuilder::MenuBuilder::default()
            .disabled("Item 1")
            .sub_menu(menubuilder::MenuBuilder::new("Menu 2").standard_item("Subitem 3", 3))
            .standard_item("Dummy", 4)
            .separator()
            .standard_item("Exit", 66)
            .build()
    } else {
        menubuilder::MenuBuilder::new_with_item(Default::default())
            .standard_item("Item 1", 1)
            .standard_item("Item 2", 2)
            .separator()
            .standard_item("Exit", 66)
            .build()
    };
    menu::Model {
        items,
        ..Default::default()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut activated = false;
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let name = SniName::new(1);
    let conn = zbus::ConnectionBuilder::session()
        .unwrap()
        .name(name)
        .unwrap()
        .serve_at(
            ITEM_OBJECT_PATH,
            sni::StatusNotifierItem::new(notifier(activated), Box::new(send.clone())),
        )
        .unwrap()
        .serve_at(
            MENU_OBJECT_PATH,
            menu::DBusMenu::new(menu(activated), Box::new(send)),
        )
        .unwrap()
        .build()
        .await
        .unwrap();
    let object_server = conn.object_server();
    let watcher = watcher::StatusNotifierWatcherProxy::new(&conn)
        .await
        .unwrap();
    watcher
        .register_status_notifier_item(&String::from(name))
        .await
        .unwrap();

    while let Some(event) = recv.recv().await {
        println!("event={event:?}");
        match event {
            Event::Notifier(sni::Event::Activate { .. }) => {
                activated = !activated;
                let iface = object_server
                    .interface::<_, sni::StatusNotifierItem>(ITEM_OBJECT_PATH)
                    .await
                    .unwrap();
                iface
                    .get_mut()
                    .await
                    .replace(iface.signal_context(), notifier(activated))
                    .await
                    .unwrap();
                let iface = object_server
                    .interface::<_, menu::DBusMenu<u32>>(MENU_OBJECT_PATH)
                    .await
                    .unwrap();
                iface
                    .get_mut()
                    .await
                    .replace(iface.signal_context(), menu(activated))
                    .await
                    .unwrap();
            }
            Event::Menu(66) => break,
            _ => {}
        }
    }
}
