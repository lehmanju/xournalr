use std::cell::RefCell;
use std::mem;
use std::num::NonZeroUsize;
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
use gtk::{Widget, WidgetPaintable, glib};
use gtk::glib::clone;
use gtk::glib::translate::IntoGlib;
use gtk::glib::Bytes;
use gtk::glib::MainContext;
use gtk::glib::PRIORITY_DEFAULT;
use gtk::graphene::{Point, Rect};
use gtk::EventSequenceState;
use gtk::Picture;
use gtk::{gdk, Native};

use gtk::gsk::{self, ContainerNode, IsRenderNode, RenderNode, TextureNode};
use gtk::prelude::*;
use gtk::ApplicationWindow;
use gtk::{subclass::prelude::*, Application};

use quadtree::{LeafNode, QuadTree, Stroke};

mod quadtree;
mod custom_widget;
use custom_widget::MainWidget;

use ring_channel::*;

static glib_logger: glib::GlibLogger = glib::GlibLogger::new(glib::GlibLoggerFormat::Plain, glib::GlibLoggerDomain::CrateTarget);

fn main() {
    log::set_logger(&glib_logger);
    log::set_max_level(log::LevelFilter::Debug);
    let app = Application::new(Some("org.xournalpp.xournalr"), Default::default());
    app.connect_activate(build_ui);

    app.run();
}

#[derive(Clone, Copy)]
enum Action {
    MousePress(MousePressAction),
    MouseMotion(MouseMotionAction),
    MouseRelease(MouseReleaseAction),
    Allocation(AllocationAction),
}

#[derive(Clone, Copy)]
struct AllocationAction {
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]

struct MousePressAction {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]

struct MouseMotionAction {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]

struct MouseReleaseAction {
    x: f64,
    y: f64,
}

#[derive(Clone)]
struct Widgets {
    widget: MainWidget,
    pipeline: RingSender<RenderNode>,
}

#[derive(Clone)]
struct AppState {
    /// document
    drawing: QuadTree,
    /// currently drawn stroke
    stroke: Option<Stroke>,
    /// scale factor
    scale: f64,
    /// x scroll offset (negative = picture moved left)
    x_offset: f64,
    /// y scroll offset (negative = picture moved up)
    y_offset: f64,
    /// width
    width: i32,
    /// height
    height: i32,
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_title(Some("XournalR"));

    let widget = MainWidget::new();
    widget.set_hexpand(true);
    widget.set_vexpand(true);

    let (sender, receiver) = MainContext::sync_channel::<Action>(PRIORITY_DEFAULT, 10);
    widget.set_size_channel(sender.clone());

    // render 3 frames in advance
    let (frame_sender, frame_receiver) = ring_channel(NonZeroUsize::new(3).unwrap());
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
    //widget.add_controller(&gesture);

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
    widget.add_controller(&gesture);

    let mut widgets = Widgets {
        widget: widget.clone(),
        pipeline: frame_sender,
    };
    let state = Rc::new(RefCell::new(AppState {
        drawing: QuadTree::Leaf(LeafNode::new()),
        stroke: None,
        scale: 1.0,
        x_offset: 0.0,
        y_offset: 0.0,
        width: 0,
        height: 0,
    }));

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

impl Widgets {
    fn update(&mut self, state: &AppState) {
        let mut render_node = state.drawing.render(state.width, state.height, state.scale, state.x_offset, state.y_offset);
        if let Some(stroke) = &state.stroke {
            let stroke_texture = stroke.draw(state.width, state.height);
            let rect = Rect::new(0.0,0.0, state.width as f32, state.height as f32);
            let texture_node = TextureNode::new(&stroke_texture,&rect);
            render_node = ContainerNode::new(&[render_node, texture_node.upcast()]).upcast();
        }
        self.pipeline.send(render_node);
        self.widget.queue_draw();
    }
}

impl AppState {
    fn dispatch(&mut self, action: Action) {
        match action {
            Action::MousePress(MousePressAction { x, y }) => {
                self.stroke = Some(Stroke::new());
                self.stroke.as_mut().unwrap().add(x,y);
            }
            Action::MouseMotion(MouseMotionAction { x, y }) => {
                self.stroke.as_mut().unwrap().add(x,y);
            }
            Action::MouseRelease(MouseReleaseAction { x, y }) => {
                let mut stroke = self.stroke.take().unwrap();
                stroke.add(x,y);
                self.drawing.push(stroke);
                self.stroke = None;
            }
            Action::Allocation(AllocationAction { width, height }) => {
                self.width = width;
                self.height = height;
            },
        }
    }
}
