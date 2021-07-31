use std::cell::Cell;
use std::cell::RefCell;
use std::ptr::slice_from_raw_parts;
use std::rc::Rc;

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
use skia_safe::Color;
use skia_safe::Paint;
use skia_safe::Path;
use skia_safe::Surface;

fn main() {
    let app = Application::new(Some("org.xournalpp.xournalr"), Default::default());
    app.connect_activate(build_ui);

    app.run();
}

enum Action {
    StylusDown(StylusDownAction),
    StylusMotion(StylusMotionAction),
    StylusUp(StylusUpAction),
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
    picture.set_halign(gtk::Align::Center);
    picture.set_size_request(300, 300);
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
        gesture.set_sequence_state(
            &gesture.current_sequence().unwrap(),
            gtk::EventSequenceState::Claimed,
        );
        sender_gesture_motion
            .send(Action::StylusMotion(StylusMotionAction { x, y }))
            .unwrap();
    });
    let sender_gesture_up = sender;
    gesture.connect_motion(move |gesture, x, y| {
        gesture.set_sequence_state(
            &gesture.current_sequence().unwrap(),
            gtk::EventSequenceState::Claimed,
        );
        sender_gesture_up
            .send(Action::StylusUp(StylusUpAction { x, y }))
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
    match action {
        Action::StylusDown(_) => state.drawing.dispatch(action, &widgets.picture),
        Action::StylusMotion(_) => state.drawing.dispatch(action, &widgets.picture),
        Action::StylusUp(_) => state.drawing.dispatch(action, &widgets.picture),
    }
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
        }

        let gdk_paintable = widget.paintable().unwrap();
        let paintable = gdk_paintable.downcast_ref::<CustomPaintable>().unwrap();
        let width = widget.width();
        let height = widget.height();
        let texture = self.draw(width, height);
        paintable.set_texture(texture.upcast());
        widget.queue_draw();
    }

    fn draw(&self, width: i32, height: i32) -> MemoryTexture {
        let mut surface = Surface::new_raster_n32_premul((width, height)).expect("no surface!");
        let canvas = surface.canvas();
        let mut path = Path::new();
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.set_anti_alias(true);
        paint.set_stroke_width(1.0);
        canvas.clear(Color::WHITE);
        for point in &self.points {
            let mut iter = point.iter();
            let (x, y) = iter.next().unwrap();
            canvas.draw_path(&path, &paint);
            path.move_to((*x as f32, *y as f32));
            for (x, y) in iter {
                path.line_to((*x as f32, *y as f32));
            }
        }
        path.close();
        canvas.save();
        let pixmap = surface.peek_pixels().unwrap();
        let size = pixmap.compute_byte_size();
        let pixmap_ptr = unsafe { pixmap.addr() };
        let data = slice_from_raw_parts(pixmap_ptr, size) as *const [u8];
        MemoryTexture::new(
            width,
            height,
            MemoryFormat::R8g8b8a8,
            &Bytes::from_owned(unsafe { &*data }),
            size,
        )
    }
}
