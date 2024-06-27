use gtk4::{
    Label, Widget,
};
use gtk4::{prelude::*, ListItem};


use crate::library::LineItem;

pub(crate) fn new_item(list_item: &ListItem) {
    let label = Label::new(None);
    list_item
        .downcast_ref::<ListItem>()
        .expect("Needs to be ListItem")
        .set_child(Some(&label));

    // Bind the LineItem properties to the widget:
    list_item
        .property_expression("item")
        .chain_property::<LineItem>("name")
        .bind(&label, "label", Widget::NONE);
}
