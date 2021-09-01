use euclid::{default::Point2D, default::Transform2D};
use geo::{LineString, Point};
use gtk::cairo::LineCap;
use gtk::cairo::{Context, LineJoin};
use rstar::{RTree, AABB};

use crate::logic::ColoredLineString;

#[derive(Clone)]
pub struct Viewport {
    pub width: i32,
    pub height: i32,
    pub transform: Transform2D<f64>,
}

pub trait Document {
    type Stroke;
    fn add(&mut self, stroke: Self::Stroke, viewport: &Viewport);
    fn elements_in_viewport<'a>(
        &'a self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a Self::Stroke> + 'a>;
    fn elements_in_viewport_mut<'a>(
        &'a mut self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a mut Self::Stroke> + 'a>;
}

impl Document for RTree<ColoredLineString> {
    type Stroke = ColoredLineString;

    fn add(&mut self, stroke: Self::Stroke, viewport: &Viewport) {
        let normalized_stroke = stroke.normalize(&viewport);
        self.insert(normalized_stroke);
    }

    fn elements_in_viewport<'a>(
        &'a self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a Self::Stroke> + 'a> {
        Box::new(self.locate_in_envelope_intersecting(&viewport.normalized()))
    }

    fn elements_in_viewport_mut<'a>(
        &'a mut self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a mut Self::Stroke> + 'a> {
        Box::new(self.locate_in_envelope_intersecting_mut(&viewport.normalized()))
    }
}

pub trait Stroke: Sized {
    fn add(&mut self, x: f64, y: f64);
    fn draw(&self, cairo_context: &Context, viewport: &Viewport);
    fn draw_direct(&self, cairo_context: &Context);
    fn normalize(self, viewport: &Viewport) -> Self;
    //    fn erase_point(self, point: (f64, f64), radius: f64) -> Vec<Self>;
}

pub enum Element {
    Stroke(LineString<f64>),
    Difference(Difference),
}

pub struct Difference {
    positive: Vec<Element>,
    negative: Vec<Element>,
}

impl Stroke for ColoredLineString {
    fn add(&mut self, x: f64, y: f64) {
        self.line_str.0.push((x, y).into());
    }

    fn draw(&self, cairo_context: &Context, viewport: &Viewport) {
        cairo_context.set_source_rgba(
            self.color.0 as f64,
            self.color.1 as f64,
            self.color.2 as f64,
            self.color.3,
        );
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.line_str.0.iter();
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
        cairo_context.set_source_rgba(
            self.color.0 as f64,
            self.color.1 as f64,
            self.color.2 as f64,
            self.color.3,
        );
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.line_str.0.iter();
        let coordinate = iter.next().unwrap();
        cairo_context.move_to(coordinate.x, coordinate.y);
        for coordinate in self.line_str.0.iter() {
            cairo_context.line_to(coordinate.x, coordinate.y);
        }
        cairo_context.stroke().unwrap();
    }

    fn normalize(mut self, viewport: &Viewport) -> Self {
        for p in &mut self.line_str.0 {
            *p = viewport.normalize_from_viewport(p.clone()).into();
        }
        self
    }

    /*fn erase_point(self, point: (f64, f64), radius: f64) -> Vec<Self> {
        let distance_2 = radius * radius;
        let mut lines = Vec::new();
        let mut current_stroke = Vec::new();
        for line in self.line_str.lines() {
            if line
                .distance_2_if_less_or_equal(&point.into(), distance_2)
                .is_some()
            {
                if !current_stroke.is_empty() {
                    lines.push(current_stroke.into());
                    current_stroke = Vec::new();
                }
            } else {
                current_stroke.push(line.start_point());
                current_stroke.push(line.end_point());
            }
        }
        if !current_stroke.is_empty() {
            lines.push(current_stroke.into());
        }
        ColoredLineString {
            line_str: lines.into(),
            color: self.color
        }
    }*/
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
