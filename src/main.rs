use euclid::default::{Transform2D, Translation2D};
use geo_types::LineString;
use gtk::gdk::ffi::{GDK_AXIS_X, GDK_AXIS_Y};
use gtk::glib::MainContext;
use gtk::glib::PRIORITY_DEFAULT;
use gtk::graphene::Rect;
use gtk::gsk::{CairoNode, ContainerNode, IsRenderNode, RenderNode, TextureNode};
use gtk::prelude::*;
use gtk::Application;
use gtk::ApplicationWindow;
use gtk::EventSequenceState;
use gtk::{glib, EventControllerScroll, EventControllerScrollFlags, Inhibit};
use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::Rc;

use quadtree::{Document, Stroke, Viewport};

mod custom_widget;
mod quadtree;
use custom_widget::MainWidget;

use ring_channel::*;
use rstar::RTree;

static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
    glib::GlibLoggerFormat::Plain,
    glib::GlibLoggerDomain::CrateTarget,
);

fn main() {
    log::set_logger(&GLIB_LOGGER);
    log::set_max_level(log::LevelFilter::Debug);
    let app = Application::new(Some("org.xournalpp.xournalr"), Default::default());
    app.connect_activate(build_ui);

    app.run();
}

#[derive(Clone, Copy)]
pub enum Action {
    MousePress(MousePressAction),
    MouseMotion(MouseMotionAction),
    MouseRelease(MouseReleaseAction),
    Allocation(AllocationAction),
    Scroll(ScrollEvent),
}

#[derive(Clone, Copy)]
pub struct AllocationAction {
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
pub struct MousePressAction {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]

pub struct MouseMotionAction {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]
pub struct MouseReleaseAction {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]
pub struct ScrollEvent {
    dx: f64,
    dy: f64,
}

#[derive(Clone)]
struct Widgets {
    widget: MainWidget,
    pipeline: RingSender<RenderNode>,
}

#[derive(Clone)]
struct AppState {
    /// document
    drawing: RTree<LineString<f64>>,
    /// currently drawn stroke
    stroke: Option<LineString<f64>>,
    viewport: Viewport,
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
        gesture.set_state(EventSequenceState::Claimed);
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

    let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::BOTH_AXES);
    let sender_scroll = sender;
    scroll_controller.connect_scroll(move |_, dx, dy| {
        sender_scroll
            .send(Action::Scroll(ScrollEvent { dx, dy }))
            .unwrap();
        Inhibit(false)
    });
    widget.add_controller(&scroll_controller);

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
            translate: Translation2D::identity(),
        },
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

impl Widgets {
    fn update(&mut self, state: &AppState) {
        let mut render_node = state.drawing.render(&state.viewport);
        if let Some(stroke) = &state.stroke {
            let rect = Rect::new(
                0.0,
                0.0,
                state.viewport.width as f32,
                state.viewport.height as f32,
            );
            let cairo_node = CairoNode::new(&rect);
            let cairo_context = cairo_node.draw_context().unwrap();
            stroke.draw(&cairo_context);
            render_node = ContainerNode::new(&[render_node, cairo_node.upcast()]).upcast();
        }
        self.pipeline.send(render_node).unwrap();
        self.widget.queue_draw();
    }
}

impl AppState {
    fn dispatch(&mut self, action: Action) {
        match action {
            Action::MousePress(MousePressAction { x, y }) => {
                self.stroke = Some(LineString(Vec::new()));
                self.stroke.as_mut().unwrap().add(x, y);
            }
            Action::MouseMotion(MouseMotionAction { x, y }) => {
                self.stroke.as_mut().unwrap().add(x, y);
            }
            Action::MouseRelease(MouseReleaseAction { x, y }) => {
                let mut stroke = self.stroke.take().unwrap();
                stroke.add(x, y);
                self.drawing.add(stroke, &self.viewport);
                self.stroke = None;
            }
            Action::Allocation(AllocationAction { width, height }) => {
                self.viewport.width = width;
                self.viewport.height = height;
            }
            Action::Scroll(ScrollEvent { dx, dy }) => {
                self.viewport.translate.x += dx;
                self.viewport.translate.y += dy;
            }
        }
    }
}
