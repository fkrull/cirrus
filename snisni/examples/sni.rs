use snisni::*;
use std::future::Future;

#[derive(Debug)]
enum MenuEvent {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let item = Item {
        icon: Icon {
            name: "folder-music".to_string(),
            pixmaps: Vec::new(),
        },
        overlay_icon: Icon::default(),
        attention_icon: Icon::default(),
        attention_movie_name: "".to_string(),
        icon_theme_path: "".to_string(),
        id: "snisni-test".to_string(),
        title: "StatusNotifierItem example".to_string(),
        tooltip: Default::default(),
        category: Category::ApplicationStatus,
        status: Status::Active,
        window_id: 0,
        item_is_menu: false,
    };
    let menu = menu::Menu::<MenuEvent> {
        text_direction: menu::TextDirection::LeftToRight,
        status: menu::Status::Normal,
        icon_theme_path: "".to_string(),
        items: vec![],
    };
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
    let notifier = StatusNotifier::new(1, item, menu, Box::new(send))
        .await
        .unwrap();
    notifier.register().await.unwrap();
    let mut download = false;
    while let Some(event) = recv.recv().await {
        println!("event={event:?}");
        if let Event::Activate { .. } = event {
            download = !download;
            let icon = if download {
                "folder-download"
            } else {
                "folder"
            };
            notifier
                .update_item(|item| {
                    item.icon.name = icon.to_string();
                    item.attention_icon.name = "audio-headphones".to_string();
                    item.status = Status::NeedsAttention;
                })
                .await
                .unwrap();
        }
    }
}
