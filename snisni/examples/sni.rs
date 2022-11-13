use snisni::*;
use std::time::Duration;

async fn on_event(event: Event) {
    println!("event={event:?}");
}

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
    let menu = Menu {};
    let _conn = run(1, item, menu, on_event).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3600)).await;
}
