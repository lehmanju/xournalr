use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use std::ptr::slice_from_raw_parts;
use std::rc::Rc;
use std::slice;
use std::{
    alloc::{self, Layout},
    cell::Cell,
};

use gtk::cairo::ffi::cairo_version_string;
use gtk::cairo::Format;
use gtk::cairo::ImageSurface;
use gtk::cairo::{Context, LineCap, LineJoin};
use gtk::gdk::ffi::{GDK_AXIS_X, GDK_AXIS_Y};
use gtk::gdk::MemoryTexture;
use gtk::gdk::{AxisFlags, AxisUse, MemoryFormat};
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::translate::IntoGlib;
use gtk::glib::Bytes;
use gtk::glib::MainContext;
use gtk::glib::PRIORITY_DEFAULT;
use gtk::graphene::Point;
use gtk::EventSequenceState;
use gtk::Picture;
use gtk::{gdk, Native};

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
    MouseRelease(MouseReleaseAction),
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

    let (sender, receiver) = MainContext::sync_channel::<Action>(PRIORITY_DEFAULT, 50);

    let gesture = gtk::GestureStylus::new();
    let sender_gesture_down = sender.clone();
    gesture.connect_down(move |gesture, x, y| {
        gesture.set_state(EventSequenceState::Claimed);
        sender_gesture_down
            .send(Action::MousePress(MousePressAction { x, y }))
            .unwrap();
    });
    let sender_gesture_motion = sender.clone();
    gesture.connect_motion(move |gesture, x, y| {
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
    gesture.connect_motion(move |gesture, x, y| {
        sender_gesture_up
            .send(Action::MouseRelease(MouseReleaseAction { x, y }))
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
        gesture.set_state(EventSequenceState::Claimed);
        let sequence = gesture.current_sequence();
        let event = gesture.last_event(sequence.as_ref());
        sender_gesture_motion
            .send(Action::MouseMotion(MouseMotionAction { x, y }))
            .unwrap();
        match event {
            Some(event) => {
                /*
                event widget = event surface -> native for surface

                native = gtk_widget_get_native (gtk_get_event_widget (event));
                gtk_native_get_surface_transform (native, &surf_x, &surf_y);

                backlog_array = g_array_new (FALSE, FALSE, sizeof (GdkTimeCoord));
                event_widget = gtk_get_event_widget (event);
                controller_widget = gtk_event_controller_get_widget (GTK_EVENT_CONTROLLER (gesture));
                for (i = 0; i < n_coords; i++)
                  {
                    const GdkTimeCoord *time_coord = &history[i];
                    graphene_point_t p;

                    if (gtk_widget_compute_point (event_widget, controller_widget,
                                                  &GRAPHENE_POINT_INIT (time_coord->axes[GDK_AXIS_X] - surf_x,
                                                                        time_coord->axes[GDK_AXIS_Y] - surf_y),
                                                  &p))
                      {
                        GdkTimeCoord translated_coord = *time_coord;

                        translated_coord.axes[GDK_AXIS_X] = p.x;
                        translated_coord.axes[GDK_AXIS_Y] = p.y;

                        g_array_append_val (backlog_array, translated_coord);
                      }
                    }

                               */
                // native = event_widget
                let native = Native::for_surface(&event.surface().unwrap()).unwrap();
                let controller_widget = gesture.widget().unwrap();
                let history = event.history();
                //println!("{}", history.len());
                let (surf_x, surf_y) = native.surface_transform();
                for e in history {
                    let x = e.axes()[AxisFlags::X.bits() as usize] - surf_x;
                    let y = e.axes()[AxisFlags::X.bits() as usize] - surf_y;
                    let point = Point::new(x as f32, y as f32);
                    if native.compute_point(&controller_widget, &point).is_some() {
                        sender_gesture_motion
                            .send(Action::MouseMotion(MouseMotionAction { x, y }))
                            .unwrap();
                    }
                }
            }
            None => (),
        }
    });
    let sender_gesture_up = sender;
    gesture.connect_drag_end(move |gesture, x, y| {
        sender_gesture_up
            .send(Action::MouseRelease(MouseReleaseAction { x, y }))
            .unwrap();
    });
    //picture.add_controller(&gesture);

    let mut widgets = Widgets {
        picture: picture.clone(),
    };
    let state = Rc::new(RefCell::new(AppState {
        drawing: Drawing { points: Vec::new() },
    }));

    receiver.attach(None, move |action| {
        update(action, &mut widgets, &mut state.borrow_mut());
        Continue(true)
    });

    window.set_child(Some(&picture));
    window.present();
}

fn update(action: Action, widgets: &mut Widgets, state: &mut AppState) {
    state.drawing.dispatch(action);
    state.drawing.update(&widgets.picture);
}

#[derive(Clone)]
struct Drawing {
    points: Vec<Vec<(f64, f64)>>,
}

impl Drawing {
    fn dispatch(&mut self, action: Action) {
        match action {
            Action::StylusDown(StylusDownAction { x, y }) => {
                self.points.push(vec![(x, y)]);
            }
            Action::StylusMotion(StylusMotionAction { x, y }) => {
                self.points.last_mut().unwrap().push((x, y));
            }
            Action::StylusUp(StylusUpAction { x, y }) => {
                self.points.last_mut().unwrap().push((x, y));
            }
            Action::MousePress(MousePressAction { x, y }) => {
                self.points.push(vec![(x, y)]);
            }
            Action::MouseMotion(MouseMotionAction { x, y }) => {
                let current_stroke = self.points.last_mut().unwrap();
                current_stroke.push((x, y));
            }
            Action::MouseRelease(MouseReleaseAction { x, y }) => {
                let current_stroke = self.points.last_mut().unwrap();
                current_stroke.push((x, y));
            }
        }

        println!("{:?}", self.points);
    }

    fn update(&self, widget: &Picture) {
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
        cairo_context.set_source_rgb(255f64, 255f64, 255f64);
        cairo_context.paint().unwrap();
        cairo_context.set_source_rgb(0f64, 0f64, 255f64);
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        for point in &self.points {
            let mut iter = point.iter();
            let (x, y) = iter.next().unwrap();
            cairo_context.move_to(*x, *y);
            for (x, y) in iter {
                cairo_context.line_to(*x, *y);
            }
        }
        cairo_context.stroke().unwrap();
        drop(cairo_context);
        let stride = surface.stride() as usize;
        let image_data = surface.data().unwrap();
        let bytes = &(*image_data);
        let data = Bytes::from(bytes);
        MemoryTexture::new(width, height, MemoryFormat::A8r8g8b8, &data, stride)
    }
}
