use crate::menu::{Item, Type};

#[derive(Debug, Clone)]
enum SubMenuItem<M> {
    Item(Item<M>),
    SubMenu(MenuBuilder<M>),
}

#[derive(Debug, Clone)]
pub struct MenuBuilder<M> {
    item: Item<M>,
    children: Vec<SubMenuItem<M>>,
}

impl<M> Default for MenuBuilder<M> {
    fn default() -> Self {
        MenuBuilder::new_with_item(Item::default())
    }
}

impl<M> MenuBuilder<M> {
    pub fn new_root() -> Self {
        MenuBuilder::new_with_item(Item::default())
    }

    pub fn new(label: impl Into<String>) -> Self {
        MenuBuilder::new_with_item(Item {
            label: label.into(),
            ..Item::default()
        })
    }

    pub fn new_with_item(item: Item<M>) -> Self {
        MenuBuilder {
            item,
            children: Vec::new(),
        }
    }

    pub fn separator(self) -> Self {
        self.item(Item {
            r#type: Type::Separator,
            ..Item::default()
        })
    }

    pub fn standard_item(self, label: impl Into<String>, message: M) -> Self {
        self.item(Item {
            message: Some(message),
            r#type: Type::Standard,
            label: label.into(),
            ..Item::default()
        })
    }

    pub fn disabled(self, label: impl Into<String>) -> Self {
        self.item(Item {
            r#type: Type::Standard,
            label: label.into(),
            enabled: false,
            ..Item::default()
        })
    }

    pub fn item(mut self, item: Item<M>) -> Self {
        self.children.push(SubMenuItem::Item(item));
        self
    }

    pub fn items(mut self, items: impl Iterator<Item = Item<M>>) -> Self {
        self.children.extend(items.map(SubMenuItem::Item));
        self
    }

    pub fn sub_menu(mut self, menu: MenuBuilder<M>) -> Self {
        self.children.push(SubMenuItem::SubMenu(menu));
        self
    }

    pub fn build(self) -> Vec<Item<M>> {
        let mut vec = Vec::new();
        self.build_into(&mut vec);
        vec
    }

    fn build_into(self, vec: &mut Vec<Item<M>>) {
        let root_idx = vec.len();
        let mut children = Vec::new();
        vec.push(self.item);
        for child in self.children {
            children.push(vec.len());
            match child {
                SubMenuItem::Item(item) => {
                    vec.push(item);
                }
                SubMenuItem::SubMenu(submenu) => {
                    submenu.build_into(vec);
                }
            }
        }
        vec[root_idx].r#type = Type::SubMenu { children };
    }
}
