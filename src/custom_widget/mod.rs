mod imp;

use gtk::{gdk::{self, Texture}, glib::{self, Receiver, Sender}, gsk::RenderNode, subclass::prelude::ObjectSubclassExt};

glib::wrapper! {
    pub struct MainWidget(ObjectSubclass<imp::MainWidget>);
}

impl Default for MainWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl MainWidget {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a CustomPaintable")
    }
    pub fn set_render_node(&self, render_node: RenderNode) {
        let self_ = imp::MainWidget::from_instance(self);
        //*self_.render_node.borrow_mut() = Some(render_node);
    }
    pub fn size_change_channel(&self) -> Receiver<(i32,i32)> {
        let self_ = imp::MainWidget::from_instance(self);
        self_.size_receiver.take().unwrap()
    }
    pub fn frame_pipeline_channel(&self) -> Sender<RenderNode> {
        let self_ = imp::MainWidget::from_instance(self);
        self_.frame_sender.clone()
    }
}
