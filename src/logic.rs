use geo_types::LineString;
use gtk::{
    graphene::Rect,
    gsk::{CairoNode, ContainerNode, IsRenderNode, RenderNode},
    prelude::WidgetExt,
};
use ring_channel::RingSender;
use rstar::RTree;

use crate::custom_widget::MainWidget;
use crate::quadtree::{Document, Stroke, Viewport};

#[derive(Clone, Copy)]
pub enum Action {
    MousePress(MousePressAction),
    MouseMotion(MouseMotionAction),
    MouseRelease(MouseReleaseAction),
    Allocation(AllocationAction),
    Scroll(ScrollEvent),
    ToolPen,
    ToolEraser,
    ToolHand,
}

#[derive(Clone, Copy)]
pub struct AllocationAction {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy)]
pub struct MousePressAction {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]

pub struct MouseMotionAction {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub struct MouseReleaseAction {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub struct ScrollEvent {
    pub dx: f64,
    pub dy: f64,
}

#[derive(Clone)]
pub struct Widgets {
    pub widget: MainWidget,
    pub pipeline: RingSender<RenderNode>,
}

#[derive(Clone)]
pub struct AppState {
    /// document
    pub drawing: RTree<LineString<f64>>,
    /// currently drawn stroke
    pub stroke: Option<LineString<f64>>,
    pub viewport: Viewport,
}

impl Widgets {
    pub fn update(&mut self, state: &AppState) {
        let rect = Rect::new(
            0.0,
            0.0,
            state.viewport.width as f32,
            state.viewport.height as f32,
        );
        let cairo_node = CairoNode::new(&rect);
        let cairo_context = cairo_node.draw_context().unwrap();
        cairo_context.set_source_rgb(255f64, 255f64, 255f64);
        cairo_context.paint().unwrap();
        let elements = state.drawing.elements_in_viewport(&state.viewport);
        for elem in elements {
            elem.draw(&cairo_context, &state.viewport);
        }
        if let Some(stroke) = &state.stroke {
            stroke.draw_direct(&cairo_context);
        }
        self.pipeline.send(cairo_node.upcast()).unwrap();
        self.widget.queue_draw();
    }
}

impl AppState {
    pub fn dispatch(&mut self, action: Action) {
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
                self.viewport.transform = self.viewport.transform.then_translate((-dx, -dy).into());
            }
            Action::ToolPen => todo!(),
            Action::ToolEraser => todo!(),
            Action::ToolHand => todo!(),
        }
    }
}
