use gtk::glib::{self, SyncSender};
use gtk::gsk::RenderNode;
use gtk::subclass::prelude::*;
use ring_channel::RingReceiver;
use crate::{Action, AllocationAction};

#[derive(Default)]
pub struct MainWidget {
    pub size_sender: Option<SyncSender<Action>>,
    pub frame_receiver: Option<RingReceiver<RenderNode>>,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWidget {
    const NAME: &'static str = "MainWidget";
    type Type = super::MainWidget;
    type ParentType = gtk::Widget;
}

impl ObjectImpl for MainWidget {}

impl WidgetImpl for MainWidget {
    fn size_allocate(&self, widget: &Self::Type, width: i32, height: i32, baseline: i32) {
        match self.size_sender {
            Some(sender) => {
                sender.send(Action::Allocation(AllocationAction { width, height }));
            }
            None => log::debug!("Sender not yet initialized"),
        }
    }
    fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
        match self.frame_receiver {
            Some(receiver) => {
                let result = receiver.recv();
                match result {
                    Ok(node) => snapshot.append_node(&node),
                    Err(_) => log::warn!("No render node"),
                }
            }
            None => log::debug!("Receiver not yet initialized"),
        }
    }
}
