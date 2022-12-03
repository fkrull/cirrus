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

fn sni_model(activated: bool) -> sni::Model {
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
        id: "layout-change".to_string(),
        title: "StatusNotifierItem layout change example".to_string(),
        ..Default::default()
    }
}

fn menu_model(activated: bool) -> menu::Model<u32> {
    let items = if activated {
        menubuilder::MenuBuilder::default()
            .disabled("Item 1")
            .sub_menu(menubuilder::MenuBuilder::new("Item 2").standard_item("Subitem 3", 3))
            .standard_item("Sneparator", 4)
            .standard_item("Exit", 66)
            .build()
    } else {
        menubuilder::MenuBuilder::new_with_item(Default::default())
            .standard_item("Item 1", 1)
            .standard_item("Item 2", 2)
            .standard_item("Item 3", 3)
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
    let handle = Handle::new(
        sni_model(activated),
        menu_model(activated),
        Box::new(send.clone()),
        Box::new(send.clone()),
    )
    .await
    .unwrap();
    let handle2 = handle.clone();
    tokio::spawn(async move { handle2.register_loop().await.unwrap() });
    while let Some(event) = recv.recv().await {
        println!("event={event:?}");
        match event {
            Event::Notifier(sni::Event::Activate { .. }) => {
                activated = !activated;
                handle.update(|m| *m = sni_model(activated)).await.unwrap();
                handle
                    .update_menu(|m| *m = menu_model(activated))
                    .await
                    .unwrap();
            }
            Event::Menu(66) => break,
            _ => {}
        }
    }
}
