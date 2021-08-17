use std::num::NonZeroUsize;
use std::slice::Windows;

use euclid::default::Box2D;
use euclid::{default::Point2D, default::Transform2D};
use geo::{LineString, Point};
use gtk::cairo::LineCap;
use gtk::{
    cairo::{Context, LineJoin},
    graphene::Rect,
    gsk::{CairoNode, IsRenderNode, RenderNode},
};
use rstar::{AABB, Envelope, PointDistance, RTree};

#[derive(Clone)]
pub struct Viewport {
    pub width: i32,
    pub height: i32,
    pub transform: Transform2D<f64>,
}

pub trait Document {
    fn add(&mut self, stroke: LineString<f64>, viewport: &Viewport);
    fn elements_in_viewport<'a>(
        &'a self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a LineString<f64>> + 'a>;
    fn elements_in_viewport_mut<'a>(
        &'a mut self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a mut LineString<f64>> + 'a>;
    fn remove_elements_in_radius(&mut self, point: (f64, f64), radius: f64)
        -> Vec<LineString<f64>>;
        fn remove_elements_in_enevelope(&mut self, envelope:  &AABB<Point<f64>>)
        -> Vec<LineString<f64>>;
}

impl Document for RTree<LineString<f64>> {
    fn add(&mut self, stroke: LineString<f64>, viewport: &Viewport) {
        let normalized_stroke = stroke.normalize(&viewport);
        self.insert(normalized_stroke);
    }

    fn elements_in_viewport<'a>(
        &'a self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a LineString<f64>> + 'a> {
        Box::new(self.locate_in_envelope_intersecting(&viewport.normalized()))
            as Box<dyn Iterator<Item = &LineString<f64>>>
    }

    fn elements_in_viewport_mut<'a>(
        &'a mut self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a mut LineString<f64>> + 'a> {
        Box::new(self.locate_in_envelope_intersecting_mut(&viewport.normalized()))
            as Box<dyn Iterator<Item = &mut LineString<f64>>>
    }

    fn remove_elements_in_radius(
        &mut self,
        point: (f64, f64),
        radius: f64,
    ) -> Vec<LineString<f64>> {
        let point = point.into();
        self.remove_within_distance(point, radius)
    }

    fn remove_elements_in_enevelope(&mut self, envelope: &AABB<Point<f64>>)
        -> Vec<LineString<f64>> {
        self.remove_in_envelope_intersecting(envelope)
    }
}

pub trait Stroke: Sized {
    fn add(&mut self, x: f64, y: f64);
    fn draw(&self, cairo_context: &Context, viewport: &Viewport);
    fn draw_direct(&self, cairo_context: &Context);
    fn normalize(self, viewport: &Viewport) -> Self;
    fn erase_point(self, point: (f64, f64), radius: f64) -> Vec<Self>;
}

pub enum Element {
    Stroke(LineString<f64>),
    Difference(Difference)
}

pub struct Difference {
    positive: Vec<Element>,
    negative: Vec<Element>,
}

impl Stroke for LineString<f64> {
    fn add(&mut self, x: f64, y: f64) {
        self.0.push((x, y).into());
    }

    fn draw(&self, cairo_context: &Context, viewport: &Viewport) {
        cairo_context.set_source_rgb(0f64, 0f64, 255f64);
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.0.iter();
        let coordinate = iter.next().unwrap();
        let point_transformed = viewport.transform_to_viewport(coordinate.clone());
        cairo_context.move_to(point_transformed.0, point_transformed.1);
        for coordinate in iter {
            let point_transformed = viewport.transform_to_viewport(coordinate.clone());
            cairo_context.line_to(point_transformed.0, point_transformed.1);
        }
        cairo_context.stroke().unwrap();
    }

    fn draw_direct(&self, cairo_context: &Context) {
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.0.iter();
        let coordinate = iter.next().unwrap();
        cairo_context.move_to(coordinate.x, coordinate.y);
        for coordinate in self.0.iter() {
            cairo_context.line_to(coordinate.x, coordinate.y);
        }
        cairo_context.stroke().unwrap();
    }

    fn normalize(mut self, viewport: &Viewport) -> Self {
        for p in &mut self.0 {
            *p = viewport.normalize_from_viewport(p.clone()).into();
        }
        self
    }

    fn erase_point(self, point: (f64, f64), radius: f64) -> Vec<Self> {
        let distance_2 = radius * radius;
        let mut result = Vec::new();
        let mut current_stroke = Vec::new();
        for line in self.lines() {
            if line
                .distance_2_if_less_or_equal(&point.into(), distance_2)
                .is_some()
            {
                if !current_stroke.is_empty() {
                    result.push(current_stroke.into());
                    current_stroke = Vec::new();
                }
            } else {
                current_stroke.push(line.start_point());
                current_stroke.push(line.end_point());
            }
        }
        if !current_stroke.is_empty() {
            result.push(current_stroke.into());
        }
        result
    }
}

impl Viewport {
    pub fn transform_to_viewport(&self, point: impl Into<(f64, f64)>) -> (f64, f64) {
        let point_transformed = self
            .transform
            .inverse()
            .unwrap()
            .transform_point(point.into().into());
        point_transformed.into()
    }
    pub fn normalize_from_viewport(&self, point: impl Into<(f64, f64)>) -> (f64, f64) {
        self.transform.transform_point(point.into().into()).into()
    }
    pub fn normalized(&self) -> AABB<Point<f64>> {
        let lower = self.transform.transform_point(Point2D::new(0.0, 0.0));
        let upper = self
            .transform
            .transform_point(Point2D::new(self.width as f64, self.height as f64));
        let lower_t: (f64, f64) = lower.into();
        let upper_t: (f64, f64) = upper.into();
        AABB::from_corners(lower_t.into(), upper_t.into())
    }
}
