use euclid::{default::Point2D, default::Transform2D};
use glib::Bytes;
use gtk::{
    cairo::{Context, Format, ImageSurface, LineCap, LineJoin},
    gdk::{MemoryFormat, MemoryTexture},
    gsk::RenderNode,
};

#[derive(Clone)]
pub struct Viewport {
    pub width: i32,
    pub height: i32,
    pub transform: Transform2D<f64>,
}

#[derive(Clone)]
pub enum QuadTreeInner {
    Leaf(LeafNode),
    Meta(MetaNode),
}

pub struct QuadTree {
    inner: QuadTreeInner,
    height: u32,
}

const THRESHOLD: usize = 4;

impl QuadTree {
    pub fn render(
        &self,
        viewport: &Viewport,
    ) -> RenderNode {
        
    }
    pub fn push(&mut self, stroke: Stroke, viewport: &Viewport) {
        let inverse = viewport.transform.inverse().unwrap();
        let stroke = stroke.transform(&inverse);
        let bounding_box = 
        //split if 
        match self {
            QuadTree::Leaf(leaf) => {
                leaf.objects.push(stroke);
                if leaf.objects.len() >= THRESHOLD {
                    let mut leafs = QuadLeafs::new();
                    for stroke in &leaf.objects {
                        let data = stroke.split();
                        leafs.add_stroke(data);
                    }
                    let meta_node = MetaNode::from_quad_leafs(leafs);
                    *self = QuadTree::Meta(meta_node);
                }
            }
            QuadTree::Meta(meta) => {
                /*if let Some(leaf) = meta.try_merge() {
                    *self = QuadTree::Leaf(leaf);
                    self.push(stroke);
                } else {*/
                let data = stroke.split();
                meta.top_left.push(data.tl.into());
                meta.top_right.push(data.tr.into());
                meta.bottom_left.push(data.bl.into());
                meta.bottom_right.push(data.br.into());
                //}
            }
        }
    }
    
}

impl QuadTreeInner {
    fn render(
        &self,
        viewport: &Viewport,
    ) -> RenderNode {
        
    }
    /// Push stroke to this Quadtree. Return number splits
    fn push_transform(&mut self, mut stroke: Stroke, transform: &Transform2D<f64>) -> u32 {
        let inverse = transform.inverse().unwrap();
        let stroke = stroke.transform(&inverse);
        
    }
}

#[derive(Clone)]
pub struct MetaNode {
    top_left: Box<QuadTree>,
    top_right: Box<QuadTree>,
    bottom_left: Box<QuadTree>,
    bottom_right: Box<QuadTree>,
}

impl MetaNode {
    fn from_quad_leafs(leafs: QuadLeafs) -> Self {
        Self {
            top_left: Box::new(QuadTree::Leaf(leafs.tl)),
            top_right: Box::new(QuadTree::Leaf(leafs.tr)),
            bottom_left: Box::new(QuadTree::Leaf(leafs.bl)),
            bottom_right: Box::new(QuadTree::Leaf(leafs.br)),
        }
    }
    fn try_merge(&self) -> Option<LeafNode> {
        // merge if all children leaf and size < THRESHOLD -1

        match self.top_left.as_ref() {
            QuadTree::Leaf(_leaf) => todo!(),
            QuadTree::Meta(_meta) => todo!(),
        }
        todo!()
    }
}

impl LeafNode {
    fn size(&self) -> usize {
        self.objects.len()
    }
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct LeafNode {
    objects: Vec<Stroke>,
}

#[derive(Debug, Clone)]
pub struct Stroke {
    points: Vec<Point2D<f64>>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Quadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

struct QuadrantData {
    tl: Vec<Point2D<f64>>,
    tr: Vec<Point2D<f64>>,
    bl: Vec<Point2D<f64>>,
    br: Vec<Point2D<f64>>,
}

struct QuadLeafs {
    tl: LeafNode,
    tr: LeafNode,
    bl: LeafNode,
    br: LeafNode,
}

impl QuadLeafs {
    fn quadrant(&mut self, selector: Quadrant) -> &mut LeafNode {
        match selector {
            Quadrant::TopLeft => &mut self.tl,
            Quadrant::TopRight => &mut self.tr,
            Quadrant::BottomLeft => &mut self.bl,
            Quadrant::BottomRight => &mut self.br,
        }
    }
    fn add_stroke(&mut self, _data: QuadrantData) {}
    fn new() -> Self {
        todo!()
    }
}

impl QuadrantData {
    fn quadrant(&mut self, selector: Quadrant) -> &mut Vec<Point2D<f64>> {
        match selector {
            Quadrant::TopLeft => &mut self.tl,
            Quadrant::TopRight => &mut self.tr,
            Quadrant::BottomLeft => &mut self.bl,
            Quadrant::BottomRight => &mut self.br,
        }
    }
    fn new() -> Self {
        QuadrantData {
            tl: Vec::new(),
            tr: Vec::new(),
            bl: Vec::new(),
            br: Vec::new(),
        }
    }
}

impl Stroke {
    fn split(&self) -> QuadrantData {
        let mut data = QuadrantData::new();

        // 0-127

        let iterator = self.points.iter();

        // size greater 0 => first element exists
        let mut last_point: Option<Point2D<f64>> = None;
        let mut last_quadrant = None;

        for point in iterator {
            let (current_quadrant, cx, cy) = if point.x <= 64f64 && point.y <= 64f64 {
                data.quadrant(Quadrant::TopLeft).push((point.x, point.y).into());
                (Quadrant::TopLeft, point.x, point.y)
            } else if point.x <= 64f64 && point.y >= 64f64 {
                data.quadrant(Quadrant::BottomLeft).push((point.x, point.y - 64f64).into());
                (Quadrant::BottomLeft, point.x, point.y - 64f64)
            } else if point.x >= 64f64 && point.y <= 64f64 {
                data.quadrant(Quadrant::TopRight).push((point.x - 64f64, point.y).into());
                (Quadrant::TopRight, point.x - 64f64, point.y)
            } else {
                (Quadrant::BottomRight, point.x - 64f64, point.y - 64f64)
            };
            if let Some(last_point) = last_point {
                let last_quadrant = last_quadrant.unwrap();
                if last_quadrant != current_quadrant {
                    if (last_quadrant == Quadrant::TopLeft || last_quadrant == Quadrant::TopRight) //top down
                        && (current_quadrant == Quadrant::BottomLeft
                            || current_quadrant == Quadrant::BottomRight)
                    {
                        let distance_top = 64f64 - last_point.y;
                        let distance_bottom = cy;
                        let distance_x = (last_point.x - cx).abs();
                        let small_x = last_point.x.min(cx);
                        let x =
                            distance_x * distance_top / (distance_top + distance_bottom) + small_x;
                        data.quadrant(last_quadrant).push((x, 64f64).into());
                        data.quadrant(current_quadrant).push((x, 0f64).into());
                    } else if (current_quadrant == Quadrant::TopLeft //bottom up
                        || current_quadrant == Quadrant::TopRight)
                        && (last_quadrant == Quadrant::BottomLeft
                            || last_quadrant == Quadrant::BottomRight)
                    {
                        let distance_bottom = 64f64 - last_point.y;
                        let distance_top = cy;
                        let distance_x = (last_point.x - cx).abs();
                        let small_x = last_point.x.min(cx);
                        let x =
                            distance_x * distance_top / (distance_top + distance_bottom) + small_x;
                        data.quadrant(current_quadrant).push((x, 64f64).into());
                        data.quadrant(last_quadrant).push((x, 0f64).into());
                    } else if (last_quadrant == Quadrant::TopLeft //left right
                        || last_quadrant == Quadrant::BottomLeft)
                        && (current_quadrant == Quadrant::TopRight
                            || current_quadrant == Quadrant::BottomRight)
                    {
                        let distance_left = 64f64 - last_point.x;
                        let distance_right = cx;
                        let distance_y = (last_point.y - cy).abs();
                        let small_y = last_point.y.min(cy);
                        let y =
                            distance_y * distance_left / (distance_left + distance_right) + small_y;
                        data.quadrant(last_quadrant).push((64f64, y).into());
                        data.quadrant(current_quadrant).push((64f64, y).into());
                    } else {
                        //right left
                        let distance_left = 64f64 - last_point.x;
                        let distance_right = cx;
                        let distance_y = (last_point.y - cy).abs();
                        let small_y = last_point.y.min(cy);
                        let y =
                            distance_y * distance_left / (distance_left + distance_right) + small_y;
                        data.quadrant(current_quadrant).push((64f64, y).into());
                        data.quadrant(last_quadrant).push((64f64, y).into());
                    }
                }
            }

            last_point = Some(Point2D::new(cx, cy));
            last_quadrant = Some(current_quadrant);
        }

        data
    }
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }
    pub fn add(&mut self, x: f64, y: f64) {
        self.points.push((x, y).into());
    }
    pub fn draw(&self, width: i32, height: i32) -> MemoryTexture {
        let mut surface = ImageSurface::create(Format::ARgb32, width, height).expect("no surface!");
        let cairo_context = Context::new(&surface).unwrap();
        cairo_context.set_source_rgb(255f64, 255f64, 255f64);
        cairo_context.paint().unwrap();
        cairo_context.set_source_rgb(0f64, 0f64, 255f64);
        cairo_context.set_line_join(LineJoin::Round);
        cairo_context.set_line_cap(LineCap::Round);
        let mut iter = self.points.iter();
        let point = iter.next().unwrap();
        cairo_context.move_to(point.x, point.y);
        for point in iter {
            cairo_context.line_to(point.x, point.y);
        }
        cairo_context.stroke().unwrap();
        drop(cairo_context);
        let stride = surface.stride() as usize;
        let image_data = surface.data().unwrap();
        let bytes = &(*image_data);
        let data = Bytes::from(bytes);
        MemoryTexture::new(width, height, MemoryFormat::A8r8g8b8, &data, stride)
    }
    pub fn transform(mut self, transform: &Transform2D<f64>) -> Self {
        for point in self.points {
            let point = Point2D::new(point.x,point.y);
            let new_point = transform.transform_point(point);
            point = new_point;
        }
        self
    }
}