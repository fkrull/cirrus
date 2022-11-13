use snisni::*;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let model = Model {
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
    let _conn = run(1, model).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3600)).await;
}
