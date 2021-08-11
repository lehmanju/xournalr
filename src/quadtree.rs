use euclid::{
    default::Point2D,
    default::{Transform2D, Translation2D},
};
use geo_types::{LineString, Point};
use gtk::cairo::LineCap;
use gtk::{
    cairo::{Context, LineJoin},
    graphene::Rect,
    gsk::{CairoNode, IsRenderNode, RenderNode},
};
use rstar::{RTree, AABB};

#[derive(Clone)]
pub struct Viewport {
    pub width: i32,
    pub height: i32,
    pub transform: Transform2D<f64>,
    pub translate: Translation2D<f64>,
}

pub trait Document {
    fn add(&mut self, stroke: LineString<f64>, viewport: &Viewport);
    fn render(&self, viewport: &Viewport) -> RenderNode;
}

impl Document for RTree<LineString<f64>> {
    fn add(&mut self, stroke: LineString<f64>, viewport: &Viewport) {
        let inverse_transform = viewport.transform.inverse().unwrap();
        let normalized_stroke = stroke.transform(&inverse_transform);
        self.insert(normalized_stroke);
    }

    fn render(&self, viewport: &Viewport) -> RenderNode {
        let lower = viewport.translate.transform_point(Point2D::new(0.0, 0.0));
        let upper = Point2D::new(viewport.width as f64, viewport.height as f64);
        let inverse_transform = viewport.transform.inverse().unwrap();
        let lower_t = lower;
        let upper_t = inverse_transform.transform_point(upper);
        let envelope = AABB::from_corners(point2d_to_point(lower_t), point2d_to_point(upper_t));
        let items = self.locate_in_envelope(&envelope);
        let rect = Rect::new(0.0, 0.0, viewport.width as f32, viewport.height as f32);
        let cairo_node = CairoNode::new(&rect);
        let cairo_context = cairo_node.draw_context().unwrap();
        cairo_context.set_source_rgb(255f64, 255f64, 255f64);
        cairo_context.paint().unwrap();
        for item in items {
            let translator = viewport.translate.inverse();
            let translated = item.clone().transform(&translator);
            let transformed = translated.transform(&viewport.transform);
            transformed.draw(&cairo_context);
        }
        cairo_node.upcast()
    }
}

pub trait Stroke {
    fn add(&mut self, x: f64, y: f64);
    fn draw(&self, cairo_context: &Context);
    fn transform(self, transform: &impl Transformer) -> Self;
}

impl Stroke for LineString<f64> {
    fn add(&mut self, x: f64, y: f64) {
        self.0.push((x, y).into());
    }

    fn draw(&self, cairo_context: &Context) {
        cairo_context.set_source_rgb(0f64, 0f64, 255f64);
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.points_iter();
        let point = iter.next().unwrap();
        cairo_context.move_to(point.x(), point.y());
        for point in iter {
            cairo_context.line_to(point.x(), point.y());
        }
        cairo_context.stroke().unwrap();
    }

    fn transform(mut self, transform: &impl Transformer) -> Self {
        for point in self.0.iter_mut() {
            let new_point = transform.transform_point(Point2D::new(point.x, point.y));
            let tuple: (f64, f64) = new_point.into();
            *point = tuple.into();
        }
        self
    }
}

trait Transformer {
    fn transform_point(&self, point: Point2D<f64>) -> Point2D<f64>;
}

impl Transformer for Transform2D<f64> {
    fn transform_point(&self, point: Point2D<f64>) -> Point2D<f64> {
        self.transform_point(point)
    }
}

impl Transformer for Translation2D<f64> {
    fn transform_point(&self, point: Point2D<f64>) -> Point2D<f64> {
        self.transform_point(point)
    }
}

fn point2d_to_point(point: Point2D<f64>) -> Point<f64> {
    Point::new(point.x, point.y)
}
