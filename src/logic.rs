use geo::{algorithm::intersects::Intersects, LineString};
use gtk::{
    cairo::{LineCap, LineJoin},
    graphene::Rect,
    gsk::{CairoNode, IsRenderNode, RenderNode},
    prelude::WidgetExt,
};
use ring_channel::RingSender;
use rstar::{RTree, RTreeObject};

use crate::custom_widget::MainWidget;
use crate::quadtree::{Document, Stroke, Viewport};

#[derive(Clone, Copy)]
pub enum Action {
    MousePress(MousePressAction),
    MouseMotion(MouseMotionAction),
    MouseRelease(MouseReleaseAction),
    Allocation(AllocationAction),
    Zoom(ZoomEvent),
    Scroll(ScrollEvent),
    Motion(MotionEvent),
    ToolPen,
    ToolEraser,
    ToolObjEraser,
    ToolHand,
    ScrollStart,
    ScrollEnd,
}

#[derive(Clone, Copy)]
pub struct MotionEvent {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub struct ZoomEvent {
    pub dscale: f64,
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

#[derive(Clone, Default)]
pub struct ScrollState {
    pub x_old: f64,
    pub y_old: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Eraser,
    ObjEraser,

    #[allow(dead_code)]
    Hand,
}

#[derive(Clone)]
pub struct AppState {
    /// document
    pub drawing: RTree<LineString<f64>>,
    /// currently drawn stroke
    pub stroke: Option<LineString<f64>>,
    pub viewport: Viewport,
    pub scroll_state: Option<ScrollState>,
    pub pointer_old: Option<(f64, f64)>,
    pub tool: Tool,
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
        cairo_context.set_source_rgb(0f64, 0f64, 255f64);
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let elements = state.drawing.elements_in_viewport(&state.viewport);
        for elem in elements {
            elem.draw(&cairo_context, &state.viewport);
        }
        if let Some(stroke) = &state.stroke {
            if state.tool == Tool::Eraser || state.tool == Tool::ObjEraser {
                cairo_context.set_source_rgb(255f64, 255f64, 255f64);
                cairo_context.set_line_width(5.0);
            }
            stroke.draw_direct(&cairo_context);
        }
        self.pipeline.send(cairo_node.upcast()).unwrap();
        self.widget.queue_draw();
    }
}

impl AppState {
    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::MousePress(MousePressAction { x, y }) => match self.tool {
                Tool::Pen | Tool::Eraser | Tool::ObjEraser => {
                    self.stroke = Some(LineString(Vec::new()));
                    self.stroke.as_mut().unwrap().add(x, y);
                }
                Tool::Hand => todo!(),
            },
            Action::MouseMotion(MouseMotionAction { x, y }) => match self.tool {
                Tool::Pen | Tool::Eraser | Tool::ObjEraser => {
                    self.stroke.as_mut().unwrap().add(x, y);
                }
                Tool::Hand => todo!(),
            },
            Action::MouseRelease(MouseReleaseAction { x, y }) => match self.tool {
                Tool::Pen => {
                    let mut stroke = self.stroke.take().unwrap();
                    stroke.add(x, y);
                    self.drawing.add(stroke, &self.viewport);
                    self.stroke = None;
                }
                Tool::Eraser | Tool::ObjEraser => {
                    let mut stroke = self.stroke.take().unwrap();
                    stroke.add(x, y);
                    let stroke = stroke.normalize(&self.viewport);
                    let elements = self
                        .drawing
                        .drain_in_envelope_intersecting(stroke.envelope());
                    if self.tool == Tool::Eraser {
                        unimplemented!()
                    } else {
                        for e in elements
                            .filter(|e| !stroke.intersects(e))
                            .collect::<Vec<_>>()
                        {
                            self.drawing.insert(e);
                        }
                    }
                    self.stroke = None;
                }
                Tool::Hand => todo!(),
            },
            Action::Allocation(AllocationAction { width, height }) => {
                self.viewport.width = width;
                self.viewport.height = height;
            }
            Action::ScrollStart => {
                self.scroll_state = Some(ScrollState::default());
            }
            Action::Scroll(ScrollEvent { dx, dy }) => {
                let ddx;
                let ddy;
                if let Some(state) = &mut self.scroll_state {
                    ddx = dx - state.x_old;
                    ddy = dy - state.y_old;
                    state.x_old = dx;
                    state.y_old = dy;
                } else {
                    ddx = dx;
                    ddy = dy;
                }
                self.viewport.transform.m31 -= ddx * self.viewport.transform.m11;
                self.viewport.transform.m32 -= ddy * self.viewport.transform.m11;
            }
            Action::ScrollEnd => {
                self.scroll_state = None;
            }
            Action::ToolPen => {
                self.tool = Tool::Pen;
            }
            Action::ToolEraser => {
                self.tool = Tool::Eraser;
            }
            Action::ToolObjEraser => {
                self.tool = Tool::ObjEraser;
            }
            Action::ToolHand => todo!(),
            Action::Zoom(ZoomEvent { dscale }) => {
                let dscale = dscale / 10f64;
                let mut dx = 0f64;
                let mut dy = 0f64;
                let scale_y = self.viewport.transform.m22 + dscale;
                let scale_x = self.viewport.transform.m11 + dscale;
                if let Some((x, y)) = self.pointer_old {
                    dx = x * dscale;
                    dy = y * dscale;
                }
                if scale_y > 0f64 && scale_x > 0f64 {
                    self.viewport.transform.m22 = scale_y;
                    self.viewport.transform.m11 = scale_x;
                    self.viewport.transform.m31 -= dx;
                    self.viewport.transform.m32 -= dy;
                }
            }
            Action::Motion(MotionEvent { x, y }) => {
                self.pointer_old = Some((x, y));
            }
        }
    }
}
