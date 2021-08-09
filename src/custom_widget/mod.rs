mod imp;

use glib::SyncSender;
use gtk::{gsk::RenderNode, subclass::prelude::ObjectSubclassExt};
use ring_channel::RingReceiver;

use crate::Action;

glib::wrapper! {
    pub struct MainWidget(ObjectSubclass<imp::MainWidget>) @extends gtk::Widget;
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
    pub fn set_render_channel(&self, receiver: RingReceiver<RenderNode>) {
        let self_ = imp::MainWidget::from_instance(self);
        self_.frame_receiver = Some(receiver);
    }
    pub fn set_size_channel(&self, sender: SyncSender<Action>) {
        let self_ = imp::MainWidget::from_instance(self);
        self_.size_sender = Some(sender);
    }
}
