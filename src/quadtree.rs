use crate::float::Float;
use aatree::{AATreeMap, AATreeSet};
use euclid::{default::Point2D, default::Transform2D};
use geo::{Coordinate, LineString, Point, Polygon};
use gtk::cairo::LineCap;
use gtk::cairo::{Context, LineJoin};
use rstar::{RTree, AABB};

#[derive(Clone)]
pub struct Viewport {
    pub width: i32,
    pub height: i32,
    pub transform: Transform2D<f64>,
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

pub trait Document {
    fn add(&mut self, stroke: Polygon<f64>, viewport: &Viewport);
    fn elements_in_viewport<'a>(
        &'a self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a LineString<f64>> + 'a>;
    fn elements_in_viewport_mut<'a>(
        &'a mut self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a mut LineString<f64>> + 'a>;
}

impl Document for RTree<Polygon<f64>> {
    fn add(&mut self, stroke: Polygon<f64>, viewport: &Viewport) {
        let normalized_stroke = stroke.normalize(viewport);
        self.insert(normalized_stroke);
    }

    fn elements_in_viewport<'a>(
        &'a self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a Polygon<f64>> + 'a> {
        Box::new(self.locate_in_envelope_intersecting(&viewport.normalized()))
            as Box<dyn Iterator<Item = &Polygon<f64>>>
    }

    fn elements_in_viewport_mut<'a>(
        &'a mut self,
        viewport: &Viewport,
    ) -> Box<dyn Iterator<Item = &'a mut Polygon<f64>> + 'a> {
        Box::new(self.locate_in_envelope_intersecting_mut(&viewport.normalized()))
            as Box<dyn Iterator<Item = &mut Polygon<f64>>>
    }
}

pub trait Stroke: Sized {
    fn add(&mut self, x: f64, y: f64);
    fn draw(&self, cairo_context: &Context, viewport: &Viewport);
    fn draw_direct(&self, cairo_context: &Context);
    fn normalize(self, viewport: &Viewport) -> Self;

    /// Erase this stroke with `radius` from a list of strokes. Each intersection splits the stroke
    /// into two new strokes with the intersection part erased.
    fn erase_from<'a, I>(&self, strokes: I, radius: f64) -> Vec<LineString<f64>>
    where
        I: IntoIterator<Item = &'a LineString<f64>>;
}

impl Stroke for Polygon<f64> {
    fn add(&mut self, x: f64, y: f64) {
        self.0.push((x, y).into());
    }

    fn draw(&self, cairo_context: &Context, viewport: &Viewport) {
        cairo_context.set_source_rgb(0f64, 0f64, 255f64);
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.0.iter();
        let coordinate = iter.next().unwrap();
        let point_transformed = viewport.transform_to_viewport(*coordinate);
        cairo_context.move_to(point_transformed.0, point_transformed.1);
        for coordinate in iter {
            let point_transformed = viewport.transform_to_viewport(*coordinate);
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
            *p = viewport.normalize_from_viewport(*p).into();
        }
        self
    }

    fn erase_from<'a, I>(&self, strokes: I, radius: f64) -> Vec<LineString<f64>>
    where
        I: IntoIterator<Item = &'a LineString<f64>>,
    {
        
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweepline() {
        let strokes = vec![LineString(vec![
            (0.0, 0.0).into(),
            (1.0, 0.0).into(),
            (2.0, 1.0).into(),
        ])];
        let erase_stroke = LineString(vec![(0.0, 0.0).into(), (2.0, 2.0).into()]);

        let _ = Stroke::erase_from(&erase_stroke, strokes.iter(), 1.0);

        assert!(false);
    }
}
