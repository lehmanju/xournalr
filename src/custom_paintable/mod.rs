mod imp;

use gtk::{
    gdk::{self, Texture},
    glib,
    subclass::prelude::ObjectSubclassExt,
};

glib::wrapper! {
    pub struct CustomPaintable(ObjectSubclass<imp::CustomPaintable>) @implements gdk::Paintable;
}

impl Default for CustomPaintable {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomPaintable {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a CustomPaintable")
    }
    pub fn set_texture(&self, texture: Texture) {
        let self_ = imp::CustomPaintable::from_instance(self);
        *self_.texture.borrow_mut() = Some(texture);
    }
}
