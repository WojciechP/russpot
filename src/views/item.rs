use gtk::{prelude::*, Button, ListItem};
use gtk::{Label, Widget};

use crate::library::LineItem;

pub(crate) fn new_item(list_item: &ListItem) {
    let btn = Button::new();
    let bx = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();
    let title = Label::new(Some("?"));
    bx.append(&title);
    btn.set_child(Some(&bx));
    list_item.set_child(Some(&btn));

    // Bind the LineItem properties to the widget:
    list_item
        .property_expression("item")
        .chain_property::<LineItem>("name")
        .bind(&title, "label", Widget::NONE);

    btn.add_css_class("line-item");
}
