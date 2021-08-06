use gtk::gsk::RenderNode;

#[derive(Clone)]
pub enum QuadTree {
    Leaf(LeafNode),
    Meta(MetaNode),
}

const THRESHOLD: usize = 4;

impl QuadTree {
    pub fn render(&self, width: i32, height: i32, scale: f64, x_offset: f64, y_offset: f64) -> RenderNode {
        match self {
            QuadTree::Leaf(_) => todo!(),
            QuadTree::Meta(_) => todo!(),
        }
    }
    pub fn push(&mut self, stroke: Stroke) {
        //stroke coordinates are relative to current node
        //stroke is fully contained in current node
        //add to elements if total number of elements < threshhold
        //else split current node
        //split stroke in 4 equal chunks
        //add each chunk to the corresponding node
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
            QuadTree::Leaf(leaf) => todo!(),
            QuadTree::Meta(meta) => todo!(),
        }
        todo!()
    }
}

impl LeafNode {
    fn size(&self) -> usize {
        self.objects.len()
    }
    pub fn new() -> Self {
        Self{objects: Vec::new()}
    }
}

#[derive(Clone)]
pub struct LeafNode {
    objects: Vec<Stroke>,
}

#[derive(Debug, Clone)]
pub struct Stroke {
    points: Vec<(f64, f64)>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Quadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

struct QuadrantData {
    tl: Vec<(f64, f64)>,
    tr: Vec<(f64, f64)>,
    bl: Vec<(f64, f64)>,
    br: Vec<(f64, f64)>,
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
    fn add_stroke(&mut self, data: QuadrantData) {}
    fn new() -> Self {
        todo!()
    }
}

impl Into<Stroke> for Vec<(f64, f64)> {
    fn into(self) -> Stroke {
        Stroke { points: self }
    }
}

impl QuadrantData {
    fn quadrant(&mut self, selector: Quadrant) -> &mut Vec<(f64, f64)> {
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
        let mut last_point: Option<(f64, f64)> = None;
        let mut last_quadrant = None;

        for (px, py) in iterator {
            let (current_quadrant, cx, cy) = if *px <= 64f64 && *py <= 64f64 {
                data.quadrant(Quadrant::TopLeft).push((*px, *py));
                (Quadrant::TopLeft, *px, *py)
            } else if *px <= 64f64 && *py >= 64f64 {
                data.quadrant(Quadrant::BottomLeft).push((*px, *py - 64f64));
                (Quadrant::BottomLeft, *px, *py - 64f64)
            } else if *px >= 64f64 && *py <= 64f64 {
                data.quadrant(Quadrant::TopRight).push((*px - 64f64, *py));
                (Quadrant::TopRight, *px - 64f64, *py)
            } else {
                (Quadrant::BottomRight, *px - 64f64, *py - 64f64)
            };
            if let Some((lx, ly)) = last_point {
                let last_quadrant = last_quadrant.unwrap();
                if last_quadrant != current_quadrant {
                    if (last_quadrant == Quadrant::TopLeft || last_quadrant == Quadrant::TopRight) //top down
                        && (current_quadrant == Quadrant::BottomLeft
                            || current_quadrant == Quadrant::BottomRight)
                    {
                        let distance_top = 64f64 - ly;
                        let distance_bottom = cy;
                        let distance_x = (lx - cx).abs();
                        let small_x = lx.min(cx);
                        let x =
                            distance_x * distance_top / (distance_top + distance_bottom) + small_x;
                        data.quadrant(last_quadrant).push((x, 64f64));
                        data.quadrant(current_quadrant).push((x, 0f64));
                    } else if (current_quadrant == Quadrant::TopLeft //bottom up
                        || current_quadrant == Quadrant::TopRight)
                        && (last_quadrant == Quadrant::BottomLeft
                            || last_quadrant == Quadrant::BottomRight)
                    {
                        let distance_bottom = 64f64 - ly;
                        let distance_top = cy;
                        let distance_x = (lx - cx).abs();
                        let small_x = lx.min(cx);
                        let x =
                            distance_x * distance_top / (distance_top + distance_bottom) + small_x;
                        data.quadrant(current_quadrant).push((x, 64f64));
                        data.quadrant(last_quadrant).push((x, 0f64));
                    } else if (last_quadrant == Quadrant::TopLeft //left right
                        || last_quadrant == Quadrant::BottomLeft)
                        && (current_quadrant == Quadrant::TopRight
                            || current_quadrant == Quadrant::BottomRight)
                    {
                        let distance_left = 64f64 - lx;
                        let distance_right = cx;
                        let distance_y = (ly - cy).abs();
                        let small_y = ly.min(cy);
                        let y =
                            distance_y * distance_left / (distance_left + distance_right) + small_y;
                        data.quadrant(last_quadrant).push((64f64, y));
                        data.quadrant(current_quadrant).push((64f64, y));
                    } else {
                        //right left
                        let distance_left = 64f64 - lx;
                        let distance_right = cx;
                        let distance_y = (ly - cy).abs();
                        let small_y = ly.min(cy);
                        let y =
                            distance_y * distance_left / (distance_left + distance_right) + small_y;
                        data.quadrant(current_quadrant).push((64f64, y));
                        data.quadrant(last_quadrant).push((64f64, y));
                    }
                }
            }

            last_point = Some((cx, cy));
            last_quadrant = Some(current_quadrant);
        }

        data
    }
    pub fn bounding_box(&self) -> Rectangle {
        todo!()
    }
    pub fn new() -> Self {
        Self {points: Vec::new()}
    }
    pub fn add(&mut self, x: f64, y: f64) {
        self.points.push((x,y));
    }
}

struct Rectangle {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

mod test {
    use crate::quadtree::{LeafNode, QuadTree, Stroke};

    #[test]
    fn add_stroke() {
        //quadtree size = 128
        let stroke = Stroke {
            points: vec![(0.0, 0.0), (1.1, 1.1), (87.2, 22.3)],
        };
        let mut tree = LeafNode::new();
        tree.push(stroke);
    }
}
