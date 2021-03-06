use std::cell::RefCell;

use crate::logic::AllocationAction;
use crate::Action;
use gtk::glib::{self, SyncSender};
use gtk::gsk::RenderNode;
use gtk::prelude::PopoverExt;
use gtk::subclass::prelude::*;
use gtk::PopoverMenu;
use ring_channel::RingReceiver;

#[derive(Default)]
pub struct MainWidget {
    pub size_sender: RefCell<Option<SyncSender<Action>>>,
    pub frame_receiver: RefCell<Option<RingReceiver<RenderNode>>>,
    last_node: RefCell<Option<RenderNode>>,
    pub popover: RefCell<Option<PopoverMenu>>,
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
        if let Some(popover) = self.popover.borrow().as_ref() {
            popover.present();
        }
    }
    fn snapshot(&self, _: &Self::Type, snapshot: &gtk::Snapshot) {
        match self.frame_receiver.borrow_mut().as_mut() {
            Some(receiver) => {
                let result = receiver.try_recv();
                match result {
                    Ok(node) => {
                        snapshot.append_node(&node);
                        *self.last_node.borrow_mut() = Some(node);
                    }
                    Err(_) => {
                        snapshot.append_node(self.last_node.borrow().as_ref().unwrap());
                    }
                }
            }
            None => log::debug!("Receiver not yet initialized"),
        }
    }
}
