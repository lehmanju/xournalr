use std::cell::RefCell;

use gtk::gdk::{self, Texture};
use gtk::glib;
use gtk::{graphene, prelude::*, subclass::prelude::*};

#[derive(Default)]
pub struct CustomPaintable {
    pub texture: RefCell<Option<Texture>>,
}

#[glib::object_subclass]
impl ObjectSubclass for CustomPaintable {
    const NAME: &'static str = "CustomPaintable";
    type Type = super::CustomPaintable;
    type ParentType = glib::Object;
    type Interfaces = (gdk::Paintable,);
}

impl ObjectImpl for CustomPaintable {}

impl PaintableImpl for CustomPaintable {
    fn flags(&self, _paintable: &Self::Type) -> gdk::PaintableFlags {
        // Fixed size
        gdk::PaintableFlags::SIZE
    }

    fn intrinsic_width(&self, _paintable: &Self::Type) -> i32 {
        0
    }

    fn intrinsic_height(&self, _paintable: &Self::Type) -> i32 {
        0
    }

    fn snapshot(&self, _paintable: &Self::Type, snapshot: &gdk::Snapshot, width: f64, height: f64) {
        let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();
        match self.texture.borrow().as_ref() {
            Some(texture) => snapshot.append_texture(
                texture,
                &graphene::Rect::new(0_f32, 0_f32, width as f32, height as f32),
            ),
            None => (),
        };
    }
}
