use std::alloc::Layout;
use std::alloc;
use std::cell::Cell;
use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use std::ptr::slice_from_raw_parts;
use std::rc::Rc;
use std::slice;

use gtk::EventSequenceState;
use gtk::cairo::Format;
use gtk::cairo::ImageSurface;
use gtk::cairo::ffi::cairo_version_string;
use gtk::cairo::Context;
use gtk::gdk;
use gtk::gdk::MemoryFormat;
use gtk::gdk::MemoryTexture;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::Bytes;
use gtk::glib::MainContext;
use gtk::glib::PRIORITY_DEFAULT;
use gtk::Picture;

use gtk::gsk;
use gtk::prelude::*;
use gtk::ApplicationWindow;
use gtk::{subclass::prelude::*, Application};

mod custom_paintable;
use custom_paintable::CustomPaintable;

fn main() {
    let app = Application::new(Some("org.xournalpp.xournalr"), Default::default());
    app.connect_activate(build_ui);

    app.run();
}

enum Action {
    StylusDown(StylusDownAction),
    StylusMotion(StylusMotionAction),
    StylusUp(StylusUpAction),
    MousePress(MousePressAction),
    MouseMotion(MouseMotionAction),
    MouseRelease(MouseReleaseAction,)
}

struct MousePressAction {
    x: f64,
    y: f64,
}

struct MouseMotionAction {
    x: f64,
    y: f64,
}

struct MouseReleaseAction {
    x: f64,
    y: f64,
}

struct StylusDownAction {
    x: f64,
    y: f64,
}
struct StylusMotionAction {
    x: f64,
    y: f64,
}
struct StylusUpAction {
    x: f64,
    y: f64,
}

#[derive(Clone)]
struct Widgets {
    picture: gtk::Picture,
}

#[derive(Clone)]
struct AppState {
    drawing: Drawing,
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_title(Some("XournalR"));

    let paintable = CustomPaintable::new();
    let picture = gtk::Picture::new();
    picture.set_hexpand(true);
    picture.set_vexpand(true);
    picture.set_paintable(Some(&paintable));

    let (sender, receiver) = MainContext::sync_channel::<Action>(PRIORITY_DEFAULT, 1);

    let gesture = gtk::GestureStylus::new();
    let sender_gesture_down = sender.clone();
    gesture.connect_down(move |gesture, x, y| {
        gesture.set_sequence_state(
            &gesture.current_sequence().unwrap(),
            gtk::EventSequenceState::Claimed,
        );
        sender_gesture_down
            .send(Action::StylusDown(StylusDownAction { x, y }))
            .unwrap();
    });
    let sender_gesture_motion = sender.clone();
    gesture.connect_motion(move |gesture, x, y| {
        
        sender_gesture_motion
            .send(Action::StylusMotion(StylusMotionAction { x, y }))
            .unwrap();
    });
    let sender_gesture_up = sender.clone();
    gesture.connect_motion(move |gesture, x, y| {
        sender_gesture_up
            .send(Action::StylusUp(StylusUpAction { x, y }))
            .unwrap();
    });
    picture.add_controller(&gesture);

    let gesture = gtk::GestureDrag::new();
    let sender_gesture_down = sender.clone();
    gesture.connect_drag_begin(move |gesture, x, y| {
        sender_gesture_down
            .send(Action::MousePress(MousePressAction { x, y }))
            .unwrap();
    });
    let sender_gesture_motion = sender.clone();
    gesture.connect_drag_update(move |gesture, x, y| {
        sender_gesture_motion
            .send(Action::MouseMotion(MouseMotionAction { x, y }))
            .unwrap();
    });
    let sender_gesture_up = sender;
    gesture.connect_drag_end(move |gesture, x, y| {
        sender_gesture_up
            .send(Action::MouseRelease(MouseReleaseAction { x, y }))
            .unwrap();
    });
    picture.add_controller(&gesture);

    let mut widgets = Widgets {
        picture: picture.clone(),
    };
    let state = Rc::new(RefCell::new(AppState {
        drawing: Drawing { points: Vec::new(), drawing: false },
    }));

    receiver.attach(None, move |action| {
        update(action, &mut widgets, &mut state.borrow_mut());
        Continue(true)
    });

    window.set_child(Some(&picture));
    window.present();
}

fn update(action: Action, widgets: &mut Widgets, state: &mut AppState) {
    state.drawing.dispatch(action, &widgets.picture)
}

#[derive(Clone)]
struct Drawing {
    points: Vec<Vec<(f64, f64)>>,
    drawing: bool,
}

impl Drawing {
    fn dispatch(&mut self, action: Action, widget: &Picture) {
        match action {
            Action::StylusDown(StylusDownAction { x  , y }) => {
                self.points.push(vec![(x,y)]);
                self.drawing = true;
            },
            Action::StylusMotion(StylusMotionAction { x, y }) => {
                if self.drawing {
                    self.points.last_mut().unwrap().push((x,y));
                }
            },
            Action::StylusUp(StylusUpAction { x, y }) => {
                self.points.last_mut().unwrap().push((x,y));
                self.drawing = false;                
            },
            Action::MousePress(MousePressAction{x,y}) => {
                self.points.push(vec![(x,y)]);
                self.drawing = true;
            },
            Action::MouseMotion(MouseMotionAction { x, y }) => {
                if self.drawing {
                    let current_stroke = self.points.last_mut().unwrap();
                    let (offset_x, offest_y) = current_stroke.first().unwrap();
                    let new_x = x + offset_x;
                    let new_y = y + offest_y;
                    current_stroke.push((new_x, new_y));
                }
            },
            Action::MouseRelease(MouseReleaseAction{x,y}) => {
                if self.drawing{
                let current_stroke = self.points.last_mut().unwrap();
                let (offset_x, offest_y) = current_stroke.first().unwrap();
                let new_x = x + offset_x;
                let new_y = y + offest_y;
                current_stroke.push((new_x, new_y));
                self.drawing = false;
                }
            },
        }

        println!("{:?}", self.points);

        let gdk_paintable = widget.paintable().unwrap();
        let paintable = gdk_paintable.downcast_ref::<CustomPaintable>().unwrap();
        let width = widget.width();
        let height = widget.height();
        let texture = self.draw(width, height);
        paintable.set_texture(texture.upcast());
        widget.queue_draw();
    }

    fn draw(&self, width: i32, height: i32) -> MemoryTexture {
        let mut surface = ImageSurface::create(Format::ARgb32, width, height).expect("no surface!");
        let cairo_context = Context::new(&surface).unwrap();
        cairo_context.set_source_rgb(255f64,255f64,255f64);
        cairo_context.paint().unwrap();
        cairo_context.set_source_rgb(0f64,0f64,255f64);
        for point in &self.points {
            let mut iter = point.iter();
            let (x, y) = iter.next().unwrap();
            cairo_context.move_to(*x,*y);
            for (x, y) in iter {
                cairo_context.line_to(*x, *y);
            }
        }
        cairo_context.stroke().unwrap();
        drop(cairo_context);
        let data = {
            let image_data = surface.data().unwrap();
            let bytes = (&*image_data).clone();
            Bytes::from(bytes)
        };        
        MemoryTexture::new(
            width,
            height,
            MemoryFormat::A8r8g8b8,
            &data,
            surface.stride() as usize,
        )
    }
}
