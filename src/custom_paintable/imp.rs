use std::cell::RefCell;

use gtk::gdk::{self, Texture};
use gtk::glib;
use gtk::gsk::RenderNode;
use gtk::{graphene, prelude::*, subclass::prelude::*};

#[derive(Default)]
pub struct CustomPaintable {
    pub render_node: RefCell<Option<RenderNode>>,
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
        gdk::PaintableFlags::empty()
    }

    fn intrinsic_width(&self, _paintable: &Self::Type) -> i32 {
        0
    }

    fn intrinsic_height(&self, _paintable: &Self::Type) -> i32 {
        0
    }

    fn snapshot(&self, _paintable: &Self::Type, snapshot: &gdk::Snapshot, width: f64, height: f64) {
        let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();
        match self.render_node.borrow().as_ref() {
            Some(node) => snapshot.append_node(node),
            None => (),
        };
    }
}
