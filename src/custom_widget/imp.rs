use std::cell::RefCell;

use crate::{Action, AllocationAction};
use gtk::glib::{self, SyncSender};
use gtk::gsk::RenderNode;
use gtk::subclass::prelude::*;
use ring_channel::RingReceiver;

#[derive(Default)]
pub struct MainWidget {
    pub size_sender: RefCell<Option<SyncSender<Action>>>,
    pub frame_receiver: RefCell<Option<RingReceiver<RenderNode>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWidget {
    const NAME: &'static str = "MainWidget";
    type Type = super::MainWidget;
    type ParentType = gtk::Widget;
}

impl ObjectImpl for MainWidget {}

impl WidgetImpl for MainWidget {
    fn size_allocate(&self, _: &Self::Type, width: i32, height: i32, _: i32) {
        match self.size_sender.borrow_mut().as_mut() {
            Some(sender) => {
                let result = sender.send(Action::Allocation(AllocationAction { width, height }));
                match result {
                    Ok(_) => log::debug!("Sent size allocation"),
                    Err(err) => log::warn!("SendError: {:?}", err),
                }
            }
            None => log::debug!("Sender not yet initialized"),
        }
    }
    fn snapshot(&self, _: &Self::Type, snapshot: &gtk::Snapshot) {
        match self.frame_receiver.borrow_mut().as_mut() {
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
