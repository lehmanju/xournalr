use euclid::default::Transform2D;
use gtk::gdk::ffi::{GDK_AXIS_X, GDK_AXIS_Y};
use gtk::gdk::BUTTON_MIDDLE;
use gtk::gdk::{Rectangle, BUTTON_SECONDARY};
use gtk::gio::{Menu, SimpleAction};
use gtk::glib::MainContext;
use gtk::glib::PRIORITY_DEFAULT;
use gtk::Application;
use gtk::ApplicationWindow;
use gtk::EventSequenceState;
use gtk::{glib, EventControllerScroll, EventControllerScrollFlags, Inhibit};
use gtk::{prelude::*, GestureClick, PopoverMenu, PopoverMenuFlags, PositionType};
use logic::{
    Action, AppState, MouseMotionAction, MousePressAction, MouseReleaseAction, ScrollEvent, Widgets,
};
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::Rc;

use quadtree::Viewport;

mod custom_widget;
mod logic;
mod quadtree;
use custom_widget::MainWidget;

use ring_channel::*;
use rstar::RTree;

static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
    glib::GlibLoggerFormat::Plain,
    glib::GlibLoggerDomain::CrateTarget,
);

fn main() {
    log::set_logger(&GLIB_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    let app = Application::new(Some("org.xournalpp.xournalr"), Default::default());
    app.connect_activate(build_ui);

    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_title(Some("XournalR"));

    let widget = MainWidget::new();
    widget.set_hexpand(true);
    widget.set_vexpand(true);

    let (sender, receiver) = MainContext::sync_channel::<Action>(PRIORITY_DEFAULT, 10);
    widget.set_size_channel(sender.clone());

    let tool_action = SimpleAction::new_stateful(
        "tool",
        Some(&String::static_variant_type()),
        &"pen".to_variant(),
    );
    tool_action.set_enabled(true);
    let tool_action_sender = sender.clone();
    tool_action.connect_activate(move |action, state| {
        let state = state.unwrap();
        action.set_state(state);
        if state.to_string() == "pen" {
            tool_action_sender.send(Action::ToolPen).unwrap();
        } else if state.to_string() == "eraser" {
            tool_action_sender.send(Action::ToolEraser).unwrap();
        }
    });
    app.add_action(&tool_action);

    let menu = Menu::new();
    menu.append(Some("Pen"), Some("app.tool::pen"));
    menu.append(Some("Eraser"), Some("app.tool::eraser"));
    menu.append(Some("Hand"), Some("app.tool::hand"));
    let popover_menu = PopoverMenu::from_model_full(&menu, PopoverMenuFlags::empty());
    popover_menu.set_position(PositionType::Left);
    widget.set_popover_menu(&popover_menu);

    // render 3 frames in advance
    let (frame_sender, frame_receiver) = ring_channel(NonZeroUsize::new(1).unwrap());
    widget.set_render_channel(frame_receiver);

    let gesture = gtk::GestureStylus::new();
    let sender_gesture_down = sender.clone();
    gesture.connect_down(move |gesture, x, y| {
        gesture.set_state(EventSequenceState::Claimed);
        sender_gesture_down
            .send(Action::MousePress(MousePressAction { x, y }))
            .unwrap();
    });
    let sender_gesture_motion = sender.clone();
    gesture.connect_motion(move |gesture, _x, _y| {
        /*sender_gesture_motion
        .send(Action::MouseMotion(MouseMotionAction { x, y }))
        .unwrap();*/
        let backlog = gesture.backlog();
        if let Some(log) = backlog {
            for l in log {
                sender_gesture_motion
                    .send(Action::MouseMotion(MouseMotionAction {
                        x: l.axes()[GDK_AXIS_X as usize],
                        y: l.axes()[GDK_AXIS_Y as usize],
                    }))
                    .unwrap();
            }
        }
    });
    let sender_gesture_up = sender.clone();
    gesture.connect_motion(move |_gesture, x, y| {
        sender_gesture_up
            .send(Action::MouseRelease(MouseReleaseAction { x, y }))
            .unwrap();
    });
    //widget.add_controller(&gesture);

    let gesture = gtk::GestureDrag::new();
    let sender_gesture_down = sender.clone();
    gesture.connect_drag_begin(move |_gesture, x, y| {
        sender_gesture_down
            .send(Action::MousePress(MousePressAction { x, y }))
            .unwrap();
    });

    let sender_gesture_motion = sender.clone();
    gesture.connect_drag_update(move |gesture, x, y| {
        //gesture.set_state(EventSequenceState::Claimed);
        let (start_x, start_y) = gesture.start_point().unwrap();
        sender_gesture_motion
            .send(Action::MouseMotion(MouseMotionAction {
                x: x + start_x,
                y: y + start_y,
            }))
            .unwrap();
    });
    let sender_gesture_up = sender.clone();
    gesture.connect_drag_end(move |gesture, x, y| {
        let (start_x, start_y) = gesture.start_point().unwrap();
        sender_gesture_up
            .send(Action::MouseRelease(MouseReleaseAction {
                x: x + start_x,
                y: y + start_y,
            }))
            .unwrap();
    });

    widget.add_controller(&gesture);

    let gesture = gtk::GestureDrag::new();
    gesture.set_button(BUTTON_MIDDLE);

    let sender_gesture_up = sender.clone();
    gesture.connect_drag_begin(move |_gesture, _x, _y| {
        sender_gesture_up.send(Action::ScrollStart).unwrap();
    });

    let sender_gesture_motion = sender.clone();
    gesture.connect_drag_update(move |gesture, x, y| {
        gesture.set_state(EventSequenceState::Claimed);
        sender_gesture_motion
            .send(Action::Scroll(ScrollEvent { dx: x, dy: y }))
            .unwrap();
    });
    let sender_gesture_end = sender.clone();
    gesture.connect_drag_end(move |_gesture, _x, _y| {
        sender_gesture_end.send(Action::ScrollEnd).unwrap();
    });
    widget.add_controller(&gesture);

    let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::BOTH_AXES);
    let sender_scroll = sender;
    scroll_controller.connect_scroll(move |_, dx, dy| {
        sender_scroll
            .send(Action::Scroll(ScrollEvent { dx: -dx, dy: -dy }))
            .unwrap();
        Inhibit(false)
    });
    widget.add_controller(&scroll_controller);

    let click_controller = GestureClick::new();
    click_controller.set_button(BUTTON_SECONDARY);
    click_controller.connect_pressed(move |_, _, x, y| {
        let rect = Rectangle {
            x: x as i32,
            y: y as i32,
            width: 1,
            height: 1,
        };
        popover_menu.set_pointing_to(&rect);
        popover_menu.popup();
    });
    widget.add_controller(&click_controller);

    let mut widgets = Widgets {
        widget: widget.clone(),
        pipeline: frame_sender,
    };
    let state = Rc::new(RefCell::new(AppState {
        drawing: RTree::new(),
        stroke: None,
        viewport: Viewport {
            width: 0,
            height: 0,
            transform: Transform2D::identity(),
        },
        scroll_state: None,
    }));
    widgets.update(&state.borrow());
    receiver.attach(None, move |action| {
        update(action, &mut widgets, &mut state.borrow_mut());
        Continue(true)
    });

    window.set_child(Some(&widget));
    window.present();
}

fn update(action: Action, widgets: &mut Widgets, state: &mut AppState) {
    state.dispatch(action);
    widgets.update(state);
}
