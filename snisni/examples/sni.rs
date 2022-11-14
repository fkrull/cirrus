use snisni::*;

#[derive(Debug)]
enum MenuEvent {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let model = sni::Model {
        icon: sni::Icon {
            name: "folder-music".to_string(),
            pixmaps: Vec::new(),
        },
        overlay_icon: sni::Icon::default(),
        attention_icon: sni::Icon::default(),
        attention_movie_name: "".to_string(),
        icon_theme_path: "".to_string(),
        id: "snisni-test".to_string(),
        title: "StatusNotifierItem example".to_string(),
        tooltip: Default::default(),
        category: sni::Category::ApplicationStatus,
        status: sni::Status::Active,
        window_id: 0,
        item_is_menu: false,
    };
    let menu = menu::Model::<()> {
        text_direction: menu::TextDirection::LeftToRight,
        status: menu::Status::Normal,
        icon_theme_path: vec![],
        items: vec![
            menu::Item {
                r#type: menu::Type::SubMenu {
                    children: vec![menu::Id(1)],
                },
                ..menu::Item::default()
            },
            menu::Item {
                label: "Menu Item".to_string(),
                icon_name: "folder".to_string(),
                ..menu::Item::default()
            },
        ],
    };
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let notifier = sni::StatusNotifierItem::new(model, Box::new(send));
    let name = SniName::new(1);
    let conn = zbus::ConnectionBuilder::session()
        .unwrap()
        .name(name)
        .unwrap()
        .serve_at(ITEM_OBJECT_PATH, notifier)
        .unwrap()
        .serve_at(
            MENU_OBJECT_PATH,
            menu::DBusMenu::new(menu, Box::new(|_| async move {})),
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

    let mut download = false;
    while let Some(event) = recv.recv().await {
        println!("event={event:?}");
        if let sni::Event::Activate { .. } = event {
            download = !download;
            let icon = if download {
                "folder-download"
            } else {
                "folder"
            };
            let iface = object_server
                .interface::<_, sni::StatusNotifierItem>(ITEM_OBJECT_PATH)
                .await
                .unwrap();
            iface
                .get_mut()
                .await
                .update(iface.signal_context(), |model| {
                    model.icon.name = icon.to_string()
                })
                .await
                .unwrap();
        }
    }
}
