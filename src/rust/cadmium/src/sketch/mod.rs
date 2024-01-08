#![allow(unused)]
use geo::line_intersection::{line_intersection, LineIntersection};
use geo::Line;
use geo::{point, Contains};
use geo::{within, Intersects};
use serde_with::{serde_as, DisplayFromStr};
use truck_polymesh::stl::PolygonMeshSTLFaceIterator;

use core::panic;
use geo::LineString;
use geo::Polygon;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::f64::consts::{PI, TAU};
use std::hash::{Hash, Hasher};
use svg::node::element::path::Data;
// use svg::node::element::Circle;
use svg::node::element::Path;
use svg::Document;

use crate::project::{Plane, Project, RealSketch};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sketch {
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub points: HashMap<u64, Point2>,
    pub highest_point_id: u64,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub line_segments: HashMap<u64, Line2>,
    pub highest_line_segment_id: u64,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub circles: HashMap<u64, Circle2>,
    pub highest_circle_id: u64,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub arcs: HashMap<u64, Arc2>,
    pub highest_arc_id: u64,

    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub constraints: HashMap<u64, Constraint>,
    pub highest_constraint_id: u64,
}

impl Sketch {
    pub fn new() -> Self {
        Sketch {
            points: HashMap::new(),
            highest_point_id: 0,
            line_segments: HashMap::new(),
            highest_line_segment_id: 0,
            circles: HashMap::new(),
            highest_circle_id: 0,
            arcs: HashMap::new(),
            highest_arc_id: 0,
            constraints: HashMap::new(),
            highest_constraint_id: 0,
        }
    }

    pub fn from_faces(faces: &Vec<Face>, real_sketch: &RealSketch) -> Self {
        let mut new_sketch = Sketch::new();

        println!("Creating a sketch just from faces");
        for face in faces {
            println!("Face: {:?}", face);
        }

        new_sketch.points = real_sketch.points_2d.clone();

        let mut circles: HashMap<String, Circle2> = HashMap::new();
        let mut line_segments: HashMap<String, Line2> = HashMap::new();
        let mut arcs: HashMap<String, Arc2> = HashMap::new();

        fn include_ring(
            ring: &Ring,
            circles: &mut HashMap<String, Circle2>,
            line_segments: &mut HashMap<String, Line2>,
            arcs: &mut HashMap<String, Arc2>,
        ) {
            match ring {
                Ring::Circle(circle) => {
                    let cs = circle.canonical_string();
                    let search_result = circles.get(&cs);
                    match search_result {
                        Some(existing_circle) => {
                            circles.remove(&cs);
                        }
                        None => {
                            circles.insert(cs.clone(), circle.clone());
                        }
                    }
                }
                Ring::Segments(segments) => {
                    for segment in segments {
                        match segment {
                            Segment::Line(line) => {
                                let cs = line.canonical_string();
                                let search_result = line_segments.get(&cs);

                                match search_result {
                                    Some(existing_line) => {
                                        line_segments.remove(&cs);
                                    }
                                    None => {
                                        line_segments.insert(cs.clone(), line.clone());
                                    }
                                }
                            }
                            Segment::Arc(arc) => {
                                let cs = arc.canonical_string();
                                let search_result = arcs.get(&cs);

                                match search_result {
                                    Some(existing_arc) => {
                                        arcs.remove(&cs);
                                    }
                                    None => {
                                        arcs.insert(cs.clone(), arc.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        for face in faces {
            include_ring(&face.exterior, &mut circles, &mut line_segments, &mut arcs);
            for ring in &face.holes {
                include_ring(ring, &mut circles, &mut line_segments, &mut arcs)
            }
        }

        for (index, circle) in circles.values().enumerate() {
            new_sketch.circles.insert(index as u64, circle.clone());
        }

        for (index, line) in line_segments.values().enumerate() {
            new_sketch.line_segments.insert(index as u64, line.clone());
        }

        for (index, arc) in arcs.values().enumerate() {
            new_sketch.arcs.insert(index as u64, arc.clone());
        }

        new_sketch
    }

    pub fn arc_angle(&self, arc: &Arc2) -> f64 {
        let center = self.points.get(&arc.center).unwrap();
        let start = self.points.get(&arc.start).unwrap();
        let end = self.points.get(&arc.end).unwrap();

        match arc.clockwise {
            false => angle(start, center, end),
            true => TAU - angle(start, center, end),
        }
    }

    pub fn arc_end_angle(&self, arc: &Arc2) -> f64 {
        let center = self.points.get(&arc.center).unwrap();
        let end = self.points.get(&arc.end).unwrap();

        let dx = end.x - center.x;
        let dy = end.y - center.y;

        if arc.clockwise {
            dy.atan2(dx) - PI / 2.0
        } else {
            dy.atan2(dx) + PI / 2.0
        }
    }

    pub fn arc_start_angle(&self, arc: &Arc2) -> f64 {
        let center = self.points.get(&arc.center).unwrap();
        let start = self.points.get(&arc.start).unwrap();

        let dx = start.x - center.x;
        let dy = start.y - center.y;

        if arc.clockwise {
            dy.atan2(dx) - PI / 2.0
        } else {
            dy.atan2(dx) + PI / 2.0
        }
    }

    pub fn line_start_angle(&self, line: &Line2) -> f64 {
        let start = self.points.get(&line.start).unwrap();
        let end = self.points.get(&line.end).unwrap();

        let dx = end.x - start.x;
        let dy = end.y - start.y;

        dy.atan2(dx)
    }

    pub fn line_end_angle(&self, line: &Line2) -> f64 {
        self.line_start_angle(line)
    }

    pub fn pretty_print_arc(&self, arc: &Arc2) {
        let center = self.points.get(&arc.center).unwrap();
        let start = self.points.get(&arc.start).unwrap();
        let end = self.points.get(&arc.end).unwrap();

        println!(
            "Arc: center: {}: ({}, {}), start: {}: ({}, {}), end: {}: ({}, {}) CW: {}",
            arc.center,
            center.x,
            center.y,
            arc.start,
            start.x,
            start.y,
            arc.end,
            end.x,
            end.y,
            arc.clockwise
        );
        println!("Start angle:\t{}", self.arc_start_angle(arc) * 180.0 / PI);
        println!("End angle:  \t{}", self.arc_end_angle(arc) * 180.0 / PI);
        println!("Angle:      \t{}", self.arc_angle(arc) * 180.0 / PI);
    }

    pub fn face_as_polygon(&self, face: &Face) -> Polygon {
        let binding = self.as_polygon(&face.exterior);
        let exterior = binding.exterior();

        let mut interiors: Vec<LineString<f64>> = vec![];
        for ring in &face.holes {
            interiors.push(self.as_polygon(ring).exterior().clone());
        }

        Polygon::new(exterior.clone(), interiors)
    }

    pub fn as_polygon(&self, ring: &Ring) -> Polygon {
        match ring {
            Ring::Circle(circle) => {
                let mut b: Vec<(f64, f64)> = vec![];
                let center = self.points.get(&circle.center).unwrap();

                let num_pts = 36;
                for i in 0..num_pts {
                    let angle = i as f64 / num_pts as f64 * TAU;
                    let x = center.x + circle.radius * angle.cos();
                    let y = center.y + circle.radius * angle.sin();
                    b.push((x, y));
                }

                let polygon = Polygon::new(LineString::from(b), vec![]);
                polygon
            }
            Ring::Segments(segments) => {
                let mut b: Vec<(f64, f64)> = vec![];
                for segment in segments {
                    match segment {
                        Segment::Line(line) => {
                            // we only ever push the start point. Imagine what happens for a closed
                            // square--the final closing segment is inferred.
                            let start = self.points.get(&segment.get_start()).unwrap();
                            b.push((start.x, start.y));
                        }
                        Segment::Arc(arc) => {
                            // similarly, we push all the points except the final one. The final
                            // segment is inferred.
                            let points = self.arc_to_points(arc);
                            for point in points {
                                b.push((point.x, point.y));
                            }
                            b.pop();
                        }
                    }
                }
                let polygon = Polygon::new(LineString::from(b), vec![]);
                polygon
            }
        }
    }

    pub fn arc_to_points(&self, arc: &Arc2) -> Vec<Point2> {
        // println!("An arc to points: {:?}", arc);
        let center = self.points.get(&arc.center).unwrap();
        let start = self.points.get(&arc.start).unwrap();
        let end = self.points.get(&arc.end).unwrap();
        let clockwise = arc.clockwise;

        arc_to_points(start, end, center, clockwise)
    }

    pub fn signed_area(&self, ring: &Ring) -> f64 {
        match ring {
            Ring::Circle(circle) => circle.radius * circle.radius * std::f64::consts::PI,
            Ring::Segments(segments) => {
                let mut area: f64 = 0.0;

                for segment in segments {
                    match segment {
                        Segment::Line(line) => {
                            let end = self.points.get(&segment.get_end()).unwrap();
                            let start = self.points.get(&segment.get_start()).unwrap();
                            area += (end.x - start.x) * (end.y + start.y);
                        }
                        Segment::Arc(arc) => {
                            let points = self.arc_to_points(arc);
                            for i in 0..points.len() - 1 {
                                let end = &points[i + 1];
                                let start = &points[i];
                                area += (end.x - start.x) * (end.y + start.y);
                            }
                        }
                    }
                }
                return area / -2.0;
            }
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) -> u64 {
        let id = self.highest_point_id + 1;
        self.points.insert(id, Point2::new(x, y));
        self.highest_point_id += 1;
        id
    }

    pub fn add_hidden_point(&mut self, x: f64, y: f64) -> u64 {
        let id = self.highest_point_id + 1;
        self.points.insert(id, Point2::new_hidden(x, y));
        self.highest_point_id += 1;
        id
    }

    pub fn add_point_with_id(&mut self, x: f64, y: f64, id0: u64) -> Result<(), String> {
        if self.points.contains_key(&id0) {
            return Err("Point already exists".to_string());
        }
        if self.highest_point_id >= id0 {
            return Err("Point ID too low".to_string());
        }
        self.points.insert(id0, Point2::new(x, y));
        self.highest_point_id = id0;
        Ok(())
    }

    pub fn add_fixed_point(&mut self, x: f64, y: f64) -> u64 {
        let id = self.highest_point_id + 1;
        self.points.insert(id, Point2::new_fixed(x, y));
        self.highest_point_id += 1;
        id
    }

    pub fn add_arc(&mut self, center_id: u64, start_id: u64, end_id: u64, clockwise: bool) -> u64 {
        let a = Arc2 {
            center: center_id,
            start: start_id,
            end: end_id,
            clockwise,
        };
        let id = self.highest_arc_id + 1;
        self.arcs.insert(id, a);
        self.highest_arc_id += 1;
        id
    }

    pub fn add_circle(&mut self, point_id: u64, radius: f64) -> u64 {
        let center_pt = self.points.get(&point_id).unwrap();
        let top = self.add_point(center_pt.x, center_pt.y + radius);
        let top_point = self.points.get_mut(&top).unwrap();
        top_point.hidden = true; // sneaky!
        let c = Circle2 {
            center: point_id,
            radius,
            top,
        };
        let id = self.highest_circle_id + 1;
        self.circles.insert(id, c);
        self.highest_circle_id += 1;
        id
    }

    pub fn add_circle_between_points(&mut self, center_id: u64, edge_id: u64) -> u64 {
        let center_pt = self.points.get(&center_id).unwrap();
        let edge_pt = self.points.get(&edge_id).unwrap();
        let radius = center_pt.distance_to(edge_pt);
        let c = Circle2 {
            center: center_id,
            radius,
            top: edge_id,
        };
        let id = self.highest_circle_id + 1;
        self.circles.insert(id, c);
        self.highest_circle_id += 1;
        id
    }

    pub fn add_rectangle_between_points(
        &mut self,
        start_id: u64,
        end_id: u64,
    ) -> (Vec<u64>, Vec<u64>) {
        let start = self.points.get(&start_id).unwrap();
        let end = self.points.get(&end_id).unwrap();

        let dx = end.x - start.x;
        let dy = end.y - start.y;

        let mut points = vec![];
        let mut segments = vec![];

        // create the two missing points
        let p0 = {
            let start_point = self.points.get(&start_id).unwrap();
            self.add_point(start_point.x + dx, start_point.y)
        };
        let p1 = {
            let start_point = self.points.get(&start_id).unwrap();
            self.add_point(start_point.x, start_point.y + dy)
        };

        points.push(p0);
        points.push(p1);

        let s0 = self.add_segment(start_id, p1);
        let s1 = self.add_segment(p1, end_id);
        let s2 = self.add_segment(end_id, p0);
        let s3 = self.add_segment(p0, start_id);

        segments.push(s0);
        segments.push(s1);
        segments.push(s2);
        segments.push(s3);

        (points, segments)
    }

    pub fn add_segment(&mut self, id0: u64, id1: u64) -> u64 {
        let l = Line2 {
            start: id0,
            end: id1,
        };
        let id = self.highest_line_segment_id + 1;
        self.line_segments.insert(id, l);
        self.highest_line_segment_id += 1;
        id
    }

    pub fn add_line_segment(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> u64 {
        let id0 = self.add_point(x0, y0);
        let id1 = self.add_point(x1, y1);
        let l = Line2 {
            start: id0,
            end: id1,
        };
        let id = self.highest_line_segment_id + 1;
        self.line_segments.insert(id, l);
        self.highest_line_segment_id += 1;
        id
    }

    pub fn delete_circle(&mut self, id: u64) {
        let center_point_id = self.circles.get(&id).unwrap().center;
        let top_point_id = self.circles.get(&id).unwrap().top;
        let mut center_is_safe = false;
        let mut top_is_safe = false;

        for (line_id, line) in self.line_segments.iter() {
            if line.start == center_point_id || line.end == center_point_id {
                center_is_safe = true;
            }
            if line.start == top_point_id || line.end == top_point_id {
                top_is_safe = true;
            }
        }

        for (arc_id, arc) in self.arcs.iter() {
            if arc.start == center_point_id
                || arc.end == center_point_id
                || arc.center == center_point_id
            {
                center_is_safe = true;
            }
            if arc.start == top_point_id || arc.end == top_point_id || arc.center == top_point_id {
                top_is_safe = true;
            }
        }

        for (circle_id, circle) in self.circles.iter() {
            if *circle_id != id {
                if circle.center == center_point_id || circle.top == center_point_id {
                    center_is_safe = true;
                }
                if circle.center == top_point_id || circle.top == top_point_id {
                    top_is_safe = true;
                }
            }
        }

        if !center_is_safe {
            self.points.remove(&center_point_id);
        }
        if !top_is_safe {
            self.points.remove(&top_point_id);
        }

        self.circles.remove(&id);
    }

    pub fn delete_arc(&mut self, id: u64) {
        // TODO: return a result instead of crashing if the arc doesn't exist
        // TODO: remove any constraints that reference this arc
        let start_point_id = self.arcs.get(&id).unwrap().start;
        let end_point_id = self.arcs.get(&id).unwrap().end;
        let center_point_id = self.arcs.get(&id).unwrap().center;
        let mut start_is_safe = false;
        let mut end_is_safe = false;
        let mut center_is_safe = false;

        for (line_id, line) in self.line_segments.iter() {
            if line.start == start_point_id || line.end == start_point_id {
                start_is_safe = true;
            }
            if line.start == end_point_id || line.end == end_point_id {
                end_is_safe = true;
            }
            if line.start == center_point_id || line.end == center_point_id {
                center_is_safe = true;
            }
        }
        for (arc_id, arc) in self.arcs.iter() {
            if (*arc_id != id) {
                if arc.start == start_point_id
                    || arc.end == start_point_id
                    || arc.center == start_point_id
                {
                    start_is_safe = true;
                }
                if arc.start == end_point_id
                    || arc.end == end_point_id
                    || arc.center == end_point_id
                {
                    end_is_safe = true;
                }
                if arc.start == center_point_id
                    || arc.end == center_point_id
                    || arc.center == center_point_id
                {
                    center_is_safe = true;
                }
            }
        }
        for (circle_id, circle) in self.circles.iter() {
            if circle.center == start_point_id || circle.top == start_point_id {
                start_is_safe = true;
            }
            if circle.center == end_point_id || circle.top == end_point_id {
                end_is_safe = true;
            }
            if circle.center == center_point_id || circle.top == center_point_id {
                center_is_safe = true;
            }
        }
        if !start_is_safe {
            self.points.remove(&start_point_id);
        }
        if !end_is_safe {
            self.points.remove(&end_point_id);
        }
        if !center_is_safe {
            self.points.remove(&center_point_id);
        }

        self.arcs.remove(&id);
    }

    pub fn delete_line_segment(&mut self, id: u64) {
        // TODO: return a result instead of crashing if the line segment doesn't exist
        // TODO: remove any constraints that reference this line segment
        let start_point_id = self.line_segments.get(&id).unwrap().start;
        let end_point_id = self.line_segments.get(&id).unwrap().end;
        let mut start_is_safe = false;
        let mut end_is_safe = false;
        for (line_id, line) in self.line_segments.iter() {
            if *line_id != id {
                if line.start == start_point_id || line.end == start_point_id {
                    start_is_safe = true;
                }
                if line.start == end_point_id || line.end == end_point_id {
                    end_is_safe = true;
                }
            }
        }
        for (arc_id, arc) in self.arcs.iter() {
            if arc.start == start_point_id
                || arc.end == start_point_id
                || arc.center == start_point_id
            {
                start_is_safe = true;
            }
            if arc.start == end_point_id || arc.end == end_point_id || arc.center == end_point_id {
                end_is_safe = true;
            }
        }
        for (circle_id, circle) in self.circles.iter() {
            if circle.center == start_point_id || circle.top == start_point_id {
                start_is_safe = true;
            }
            if circle.center == end_point_id || circle.top == end_point_id {
                end_is_safe = true;
            }
        }
        if !start_is_safe {
            self.points.remove(&start_point_id);
        }
        if !end_is_safe {
            self.points.remove(&end_point_id);
        }

        self.line_segments.remove(&id);
    }

    pub fn add_line_with_id(&mut self, start_id: u64, end_id: u64, id: u64) -> Result<(), String> {
        if self.line_segments.contains_key(&id) {
            return Err("Line already exists".to_string());
        }
        if self.highest_line_segment_id >= id {
            return Err("Line ID too low".to_string());
        }
        if !self.points.contains_key(&start_id) {
            return Err("Start point does not exist".to_string());
        }
        if !self.points.contains_key(&end_id) {
            return Err("End point does not exist".to_string());
        }

        let l = Line2 {
            start: start_id,
            end: end_id,
        };
        self.line_segments.insert(id, l);
        self.highest_line_segment_id = id;
        Ok(())
    }

    pub fn add_segment_length_constraint(&mut self, segment_id: u64, length: f64) -> u64 {
        let mut constraint = Constraint::SegmentLength {
            segment_id,
            length,
            normal_offset: 0.15,
            parallel_offset: 0.0,
            kp: 2.0,
            kd: 0.3,
            error: 0.0,
        };

        let id = self.highest_constraint_id + 1;
        self.constraints.insert(id, constraint);
        self.highest_constraint_id += 1;

        let err = self.constraint_error(id);
        let c = self.constraints.get_mut(&id).unwrap();
        if let Constraint::SegmentLength { error, .. } = c {
            *error = err;
        }

        id
    }

    pub fn add_segment_vertical_constraint(&mut self, segment_id: u64) -> u64 {
        let current_angle = self.segment_angle(segment_id);
        if current_angle >= 0.0 {
            // it roughly points up
            self.add_segment_angle_constraint(segment_id, PI / 2.0)
        } else {
            self.add_segment_angle_constraint(segment_id, -PI / 2.0)
        }
    }

    pub fn add_segment_horizontal_constraint(&mut self, segment_id: u64) -> u64 {
        let current_angle = self.segment_angle(segment_id);
        if current_angle.abs() <= PI / 2.0 {
            // it roughly points right
            self.add_segment_angle_constraint(segment_id, 0.0)
        } else {
            self.add_segment_angle_constraint(segment_id, PI)
        }
    }

    pub fn add_segment_angle_constraint(&mut self, segment_id: u64, angle: f64) -> u64 {
        let constraint = Constraint::SegmentAngle {
            segment_id,
            angle,
            x_offset: 0.0,
            y_offset: 0.0,
            kp: 2.0,
            kd: 0.3,
            error: 0.0,
        };

        let id = self.highest_constraint_id + 1;
        self.constraints.insert(id, constraint);
        self.highest_constraint_id += 1;

        let err = self.constraint_error(id);
        let c = self.constraints.get_mut(&id).unwrap();
        if let Constraint::SegmentAngle { error, .. } = c {
            *error = err;
        }

        id
    }

    pub fn add_circle_diameter_constraint(&mut self, circle_id: u64, diameter: f64) -> u64 {
        let constraint = Constraint::CircleDiameter {
            circle_id,
            diameter,
            angle_offset: 3.0 * PI / 4.0,
            r_offset: 0.20,
            kp: 2.0,
            kd: 0.3,
            error: 0.0,
        };

        let id = self.highest_constraint_id + 1;
        self.constraints.insert(id, constraint);
        self.highest_constraint_id += 1;

        let err = self.constraint_error(id);
        let c = self.constraints.get_mut(&id).unwrap();
        if let Constraint::CircleDiameter { error, .. } = c {
            *error = err;
        }

        id
    }

    pub fn add_segments_equal_constraint(&mut self, segment_a_id: u64, segment_b_id: u64) -> u64 {
        let constraint = Constraint::SegmentsEqual {
            segment_a_id,
            segment_b_id,
            kp: 2.0,
            kd: 0.3,
            error: 0.0,
        };

        let id = self.highest_constraint_id + 1;
        self.constraints.insert(id, constraint);
        self.highest_constraint_id += 1;

        let err = self.constraint_error(id);
        let c = self.constraints.get_mut(&id).unwrap();
        if let Constraint::SegmentsEqual { error, .. } = c {
            *error = err;
        }

        id
    }

    pub fn compute_constraint_errors(&mut self) {
        let key_to_errors = self
            .constraints
            .iter()
            .map(|(k, _v)| (*k, self.constraint_error(*k)))
            .collect::<HashMap<_, _>>();
        for (constraint_id, err) in key_to_errors.iter() {
            let constraint = self.constraints.get_mut(constraint_id).unwrap();
            match constraint {
                Constraint::SegmentLength { error, .. } => {
                    *error = *err;
                }
                Constraint::CircleDiameter { error, .. } => {
                    *error = *err;
                }
                Constraint::SegmentAngle { error, .. } => {
                    *error = *err;
                }
                Constraint::SegmentsEqual { error, .. } => {
                    *error = *err;
                }
            }
        }
    }

    pub fn constraint_error(&self, constraint_id: u64) -> f64 {
        let constraint = self.constraints.get(&constraint_id).unwrap();
        let value = self.constraint_value(constraint_id);
        match constraint {
            Constraint::SegmentLength { length, .. } => value - length,
            Constraint::CircleDiameter { diameter, .. } => value - diameter,
            Constraint::SegmentAngle { angle, .. } => value - angle,
            Constraint::SegmentsEqual { .. } => value,
        }
    }

    pub fn constraint_value(&self, constraint_id: u64) -> f64 {
        let constraint = self.constraints.get(&constraint_id).unwrap();
        match constraint {
            Constraint::SegmentLength {
                segment_id, length, ..
            } => {
                let segment = self.line_segments.get(&segment_id).unwrap();
                let start = self.points.get(&segment.start).unwrap();
                let end = self.points.get(&segment.end).unwrap();
                start.distance_to(end)
            }

            Constraint::CircleDiameter {
                circle_id,
                diameter,
                ..
            } => {
                let circle = self.circles.get(&circle_id).unwrap();
                circle.radius * 2.0
            }

            Constraint::SegmentAngle {
                segment_id, angle, ..
            } => {
                let segment = self.line_segments.get(&segment_id).unwrap();
                let start = self.points.get(&segment.start).unwrap();
                let end = self.points.get(&segment.end).unwrap();
                start.angle_to(end)
            }

            Constraint::SegmentsEqual {
                segment_a_id,
                segment_b_id,
                ..
            } => {
                let a = self.segment_length(*segment_a_id);
                let b = self.segment_length(*segment_b_id);
                a - b
            }
        }
    }

    pub fn constraint_is_satisfied(&self, constraint_id: u64) -> bool {
        let tolerance = 1e-10;
        let constraint = self.constraints.get(&constraint_id).unwrap();
        let error = self.constraint_error(constraint_id);
        error.abs() < tolerance
    }

    pub fn all_constraints_are_satisfied(&self) -> bool {
        for (constraint_id, _constraint) in self.constraints.iter() {
            if !self.constraint_is_satisfied(*constraint_id) {
                return false;
            }
        }
        true
    }

    pub fn segment_length(&self, segment_id: u64) -> f64 {
        let segment = self.line_segments.get(&segment_id).unwrap();
        let start = self.points.get(&segment.start).unwrap();
        let end = self.points.get(&segment.end).unwrap();
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        dx.hypot(dy)
    }

    pub fn segment_angle(&self, segment_id: u64) -> f64 {
        let segment = self.line_segments.get(&segment_id).unwrap();
        let start = self.points.get(&segment.start).unwrap();
        let end = self.points.get(&segment.end).unwrap();
        start.angle_to(end)
    }

    fn apply_length_forces(
        &mut self,
        point_a_id: u64,
        point_b_id: u64,
        rest: f64,
        kp: f64,
        kd: f64,
    ) {
        let mut fx = 0.0;
        let mut fy = 0.0;
        let mut pa_hidden = false;
        let mut pb_hidden = false;
        {
            let point_a = self.points.get(&point_a_id).unwrap();
            let point_b = self.points.get(&point_b_id).unwrap();

            let dx = point_b.x - point_a.x;
            let dy = point_b.y - point_a.y;
            let dist = dx.hypot(dy);
            let err = dist - rest;

            let relative_dx = point_b.dx - point_a.dx;
            let relative_dy = point_b.dy - point_a.dy;

            // project the relative velocity onto the vector between the points
            // a is the velocity
            // b is the vector between the points
            // a dot b / |b|
            let closing_velocity = (relative_dx * dx + relative_dy * dy) / dist;

            let f = kp * err + kd * closing_velocity;
            fx = f * dx / dist;
            fy = f * dy / dist;

            pa_hidden = point_a.hidden;
            pb_hidden = point_b.hidden;
        }

        // if a point is hidden, it feels forces but does not exert them
        if !pa_hidden {
            let point_b = self.points.get_mut(&point_b_id).unwrap();
            point_b.fx -= fx;
            point_b.fy -= fy;
        }
        if !pb_hidden {
            let point_a = self.points.get_mut(&point_a_id).unwrap();
            point_a.fx += fx;
            point_a.fy += fy;
        }
    }

    fn apply_torsion_forces(
        &mut self,
        point_a_id: u64,
        point_b_id: u64,
        rest: f64,
        kp: f64,
        kd: f64,
    ) {
        let mut fx = 0.0;
        let mut fy = 0.0;
        {
            let point_a = self.points.get(&point_a_id).unwrap();
            let point_b = self.points.get(&point_b_id).unwrap();

            let dt = 0.01;

            let angle = (point_b.y - point_a.y).atan2(point_b.x - point_a.x);

            let mut err = rest - angle;
            // println!("Err: {}", err);
            if err > PI {
                err = err - PI * 2.0;
            }
            if err < -PI {
                err = err + PI * 2.0;
            }

            let point_a_stepped = point_a.step(dt);
            let point_b_stepped = point_b.step(dt);
            let angle_stepped = (point_b_stepped.1 - point_a_stepped.1)
                .atan2(point_b_stepped.0 - point_a_stepped.0);
            let mut angle_change = angle_stepped - angle;
            // println!("Dangle: {}", angle_change);

            if angle_change > PI {
                angle_change = angle_change - PI * 2.0;
            }
            if angle_change < -PI {
                angle_change = angle_change + PI * 2.0;
            }

            let d_angle = angle_change / dt;
            let torque = kp * err - kd * d_angle;

            let dx = point_b.x - point_a.x;
            let dy = point_b.y - point_a.y;
            let dist = dx.hypot(dy);

            let f_mag = torque / dist;

            fx = f_mag * dy;
            fy = -f_mag * dx;
        }

        let point_a = self.points.get_mut(&point_a_id).unwrap();
        point_a.fx += fx;
        point_a.fy += fy;

        let point_b = self.points.get_mut(&point_b_id).unwrap();
        point_b.fx -= fx;
        point_b.fy -= fy;
    }

    pub fn solve(&mut self, steps: u64) -> bool {
        let tolerance = 1e-12;

        for _ in 0..steps {
            let retval = self.take_a_step();
            if retval < tolerance {
                return true;
            }
        }
        return false;
    }

    pub fn apply_forces(&mut self, constraint_id: u64) {
        let constraint = self.constraints.get(&constraint_id).unwrap().clone();

        match constraint {
            Constraint::SegmentsEqual {
                segment_a_id,
                segment_b_id,
                kp,
                kd,
                ..
            } => {
                let a = self.line_segments.get(&segment_a_id).unwrap();
                let b = self.line_segments.get(&segment_b_id).unwrap();

                // TODO: is there a better way to satisfy the borrow checker?
                let mut average_length = 0.0;
                let mut a_start = 0;
                let mut b_start = 0;
                let mut a_end = 0;
                let mut b_end = 0;
                {
                    average_length = (self.segment_length(segment_a_id)
                        + self.segment_length(segment_b_id))
                        / 2.0;
                    a_start = a.start;
                    b_start = b.start;
                    a_end = a.end;
                    b_end = b.end;
                }
                self.apply_length_forces(a_start, a_end, average_length, kp, kd);
                self.apply_length_forces(b_start, b_end, average_length, kp, kd);
            }
            Constraint::SegmentLength {
                segment_id,
                length,
                kp,
                kd,
                ..
            } => {
                let segment = self.line_segments.get(&segment_id).unwrap();
                self.apply_length_forces(segment.start, segment.end, length, kp, kd)
            }
            Constraint::CircleDiameter {
                circle_id,
                diameter,
                kp,
                kd,
                ..
            } => {
                let circle = self.circles.get(&circle_id).unwrap();
                let center = self.points.get(&circle.center).unwrap();
                let top = self.points.get(&circle.top).unwrap();
                let radius = center.distance_to(top);

                self.apply_length_forces(circle.center, circle.top, diameter / 2.0, kp, kd)
            }
            Constraint::SegmentAngle {
                segment_id,
                angle,
                kp,
                kd,
                ..
            } => {
                let segment = self.line_segments.get(&segment_id).unwrap();
                self.apply_torsion_forces(segment.start, segment.end, angle, kp, kd);
            }
        }
    }

    pub fn take_a_step(&mut self) -> f64 {
        let dt = 0.02; // at 0.04 the system can be unstable! especially manual_rectangle()
                       // TODO: switch to RK4?
        let mut biggest_change = 0.0;
        for (_point_id, point) in self.points.iter_mut() {
            point.reset_forces();
        }

        let constraint_keys = self
            .constraints
            .keys()
            .sorted()
            .map(|k| k.clone())
            .collect::<Vec<_>>();
        for constraint_id in constraint_keys {
            self.apply_forces(constraint_id);
        }

        for point in self.points.values_mut() {
            point.apply_drag_force();
        }

        for (point_id, point) in self.points.iter_mut() {
            if point.fixed {
                continue;
            }
            let ax = point.fx / point.m;
            let ay = point.fy / point.m;
            point.dx += ax;
            point.dy += ay;
            let delta_x = 0.5 * ax * dt * dt + point.dx * dt;
            let delta_y = 0.5 * ay * dt * dt + point.dy * dt;

            if delta_x.abs() > biggest_change {
                biggest_change = delta_x.abs();
            }
            if delta_y.abs() > biggest_change {
                biggest_change = delta_y.abs();
            }

            point.x += delta_x;
            point.y += delta_y;
        }

        // update any circles whose radii might have changed!
        for (_circle_id, circle) in self.circles.iter_mut() {
            let center = self.points.get(&circle.center).unwrap();
            let top = self.points.get(&circle.top).unwrap();
            circle.radius = center.distance_to(top);
        }

        biggest_change
    }

    pub fn print_state_minimal(&self) {
        let mut data = vec![];
        for (point_id, point) in self.points.iter().sorted_by_key(|(id, _)| *id) {
            data.push(*point_id as f64);
            data.push(point.x);
            data.push(point.y);
        }
        let strings = data.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        println!("{}", strings.join(","));
    }

    pub fn print_state(&self) {
        let mut data = vec![];
        for (_point_id, point) in self.points.iter() {
            data.push(point.x);
            data.push(point.y);
            data.push(point.dx);
            data.push(point.dy);
            data.push(point.fx);
            data.push(point.fy);
        }
        let strings = data.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        println!("{}", strings.join(","));
    }

    pub fn save_svg(&self, filename: &str) {
        // Find the maximum extent of the points so we can set a viewport
        let mut extended_points: Vec<Point2> = self.points.values().map(|p| p.clone()).collect();

        for (_circle_id, circle) in self.circles.iter() {
            let center = self.points.get(&circle.center).unwrap();
            let left = Point2::new(center.x - circle.radius, center.y);
            let right = Point2::new(center.x + circle.radius, center.y);
            let top = Point2::new(center.x, center.y + circle.radius);
            let bottom = Point2::new(center.x, center.y - circle.radius);
            extended_points.extend(vec![left, right, top, bottom]);
        }

        for (_arc_id, arc) in self.arcs.iter() {
            let center = self.points.get(&arc.center).unwrap();
            let start = self.points.get(&arc.start).unwrap();
            let r = (center.x - start.x).hypot(center.y - start.y);
            let left = Point2::new(center.x - r, center.y);
            let right = Point2::new(center.x + r, center.y);
            let top = Point2::new(center.x, center.y + r);
            let bottom = Point2::new(center.x, center.y - r);
            extended_points.extend(vec![left, right, top, bottom]);
        }

        if extended_points.len() == 0 {
            extended_points.push(Point2::new(0.0, 0.0));
            extended_points.push(Point2::new(1.0, 1.0));
        }
        let point0 = &extended_points[0];
        let mut min_x = point0.x;
        let mut min_y = point0.y;
        let mut max_x = point0.x;
        let mut max_y = point0.y;
        for point in extended_points {
            if point.x < min_x {
                min_x = point.x;
            }
            if point.y < min_y {
                min_y = point.y;
            }
            if point.x > max_x {
                max_x = point.x;
            }
            if point.y > max_y {
                max_y = point.y;
            }
        }

        let dx = max_x - min_x;
        let dy = max_y - min_y;
        let buffer_percent = 10.0;
        let buffer_x = dx * buffer_percent / 100.0;
        let buffer_y = dy * buffer_percent / 100.0;

        let mut document = Document::new().set(
            "viewBox",
            (
                min_x - buffer_x,
                -(max_y + buffer_y),
                dx + buffer_x * 2.0,
                dy + buffer_y * 2.0,
            ),
        );

        // Start by creating shapes for each face
        let (faces, unused_segments) = self.find_faces();

        // println!("Making SVG. Faces:");
        // for face in faces.iter() {
        //     println!("{:?}", face);
        // }
        for face in faces.iter() {
            let exterior = &face.exterior;

            let mut data = self.ring_to_data(exterior, Data::new());

            for hole in face.holes.iter() {
                data = self.ring_to_data(hole, data);
            }

            let path = Path::new()
                .set("fill", "none")
                .set("stroke", "black")
                .set("stroke-width", 0.01)
                .set("fill-rule", "evenodd")
                .set("d", data);

            document = document.add(path);
        }

        for segment in unused_segments.iter() {
            let mut data = Data::new();

            match segment {
                Segment::Line(line) => {
                    let start = self.points.get(&line.start).unwrap();
                    let end = self.points.get(&line.end).unwrap();
                    data = data.move_to((start.x, -start.y));
                    data = data.line_to((end.x, -end.y));
                }
                Segment::Arc(arc) => {
                    let center = self.points.get(&arc.center).unwrap();
                    let start = self.points.get(&arc.start).unwrap();
                    let end = self.points.get(&arc.end).unwrap();

                    let r = (center.x - start.x).hypot(center.y - start.y);

                    data = data.move_to((start.x, -start.y));

                    let arc_angle_degrees = self.arc_angle(arc) * 180.0 / PI;
                    println!("arc_angle: {}", arc_angle_degrees);

                    if arc_angle_degrees > 180.0 {
                        println!("large arc flag!");
                        //A rx ry x-axis-rotation large-arc-flag sweep-flag x y
                        data = data.elliptical_arc_to((r, r, 0.0, 1, 0, end.x, -end.y));
                    } else {
                        //A rx ry x-axis-rotation large-arc-flag sweep-flag x y
                        data = data.elliptical_arc_to((r, r, 0.0, 0, 0, end.x, -end.y));
                    }
                }
            }

            let path = Path::new()
                .set("fill", "none")
                .set("stroke", "black")
                .set("stroke-width", 0.01)
                .set("d", data);

            document = document.add(path);
        }

        // for (_circle_id, circle) in self.circles.iter() {
        //     let center = self.points.get(&circle.center).unwrap();

        //     let svg_circle = Circle::new()
        //         .set("cx", center.x)
        //         .set("cy", -center.y)
        //         .set("r", circle.radius)
        //         .set("fill", "none")
        //         .set("stroke", "black")
        //         .set("stroke-width", 0.01);

        //     document = document.add(svg_circle);
        // }

        svg::save(filename, &document).unwrap();
    }

    pub fn line_line_intersection(&self, line_a: &Line2, line_b: &Line2) -> Intersection {
        let a_start = self.points.get(&line_a.start).unwrap();
        let a_end = self.points.get(&line_a.end).unwrap();
        let b_start = self.points.get(&line_b.start).unwrap();
        let b_end = self.points.get(&line_b.end).unwrap();

        // println!("a start: {:?}", a_start);
        // println!("a end: {:?}", a_end);
        // println!("b start: {:?}", b_start);
        // println!("b end: {:?}", b_end);

        // Start by handling the degenerate cases
        if line_a.start == line_b.start || line_a.start == line_b.end {
            // println!("A.start is degen");

            if line_a.end == line_b.start || line_a.end == line_b.end {
                // println!("AND A.end is degen");
                // these lines are equal or reverse equal
                return Intersection::Line(a_start.clone(), a_end.clone());
            }

            return Intersection::OnePoint(self.points.get(&line_a.start).unwrap().clone(), true);
        }
        if line_a.end == line_b.start || line_a.end == line_b.end {
            // println!("A.end is degen");
            return Intersection::OnePoint(self.points.get(&line_a.end).unwrap().clone(), true);
        }

        // println!("Was not degenerate");

        fn normal_form(start: &Point2, end: &Point2) -> (f64, f64, f64) {
            let a = start.y - end.y;
            let b = end.x - start.x;
            let c = (start.x - end.x) * start.y + (end.y - start.y) * start.x;
            return (a, b, c);
        }

        let (a1, b1, c1) = normal_form(&a_start, &a_end);
        let (a2, b2, c2) = normal_form(&b_start, &b_end);

        let x_intercept = (b1 * c2 - b2 * c1) / (a1 * b2 - a2 * b1);
        let y_intercept = (c1 * a2 - c2 * a1) / (a1 * b2 - a2 * b1);

        // println!(
        //     "Computed X and Y intercept: {}, {}",
        //     x_intercept, y_intercept
        // );

        if x_intercept.is_infinite() || y_intercept.is_infinite() {
            // println!("Infinite intercept, so lines are parallel");
            return Intersection::None;
        }

        fn within_range(x: f64, a: f64, b: f64, epsilon: f64) -> bool {
            if a < b {
                return x >= a - epsilon && x <= b + epsilon;
            } else {
                return x >= b - epsilon && x <= a + epsilon;
            }
        }

        if x_intercept.is_nan() && y_intercept.is_nan() {
            // println!("NaN intercept, so either there is line overlap or they are disjoint");

            let mut special_points: Vec<Point2> = vec![];

            if within_range(a_start.x, b_start.x, b_end.x, 0.0)
                && within_range(a_start.y, b_start.y, b_end.y, 0.0)
            {
                special_points.push(a_start.clone());
            }

            if within_range(a_end.x, b_start.x, b_end.x, 0.0)
                && within_range(a_end.y, b_start.y, b_end.y, 0.0)
            {
                special_points.push(a_end.clone());
            }

            if within_range(b_start.x, a_start.x, a_end.x, 0.0)
                && within_range(b_start.y, a_start.y, a_end.y, 0.0)
            {
                special_points.push(b_start.clone());
            }

            if within_range(b_end.x, a_start.x, a_end.x, 0.0)
                && within_range(b_end.y, a_start.y, a_end.y, 0.0)
            {
                special_points.push(b_end.clone());
            }

            match special_points.len() {
                0 => return Intersection::None,
                2 => {
                    return Intersection::Line(special_points[0].clone(), special_points[1].clone())
                }
                _ => panic!(
                    "Unexpected number of special points: {}",
                    special_points.len()
                ),
            }
        }

        // it only counts as an intersection if it falls within both the segments
        // Check that the x-intercept is within the x-range of the first segment

        // println!("X intercept: {}", x_intercept);
        // println!("{}", a_start.x.min(a_end.x));
        // println!("{}", a_start.x.max(a_end.x));

        // if x_intercept >= a_start.x.min(a_end.x) && x_intercept <= a_start.x.max(a_end.x) {
        let epsilon = 1e-12;
        if within_range(x_intercept, a_start.x, a_end.x, epsilon)
            && within_range(y_intercept, a_start.y, a_end.y, epsilon)
        {
            // println!("Fulfilled x range on a");
            // check that the x-intercept is within the x-range of the second segment
            // if x_intercept >= b_start.x.min(b_end.x) && x_intercept <= b_start.x.max(b_end.x) {
            if within_range(x_intercept, b_start.x, b_end.x, epsilon)
                && within_range(y_intercept, b_start.y, b_end.y, epsilon)
            {
                // println!("Fulfilled x range on b");
                return Intersection::OnePoint(Point2::new(x_intercept, y_intercept), false);
            }
        }

        // println!("Did not fulfill x range somehow");

        Intersection::None
    }

    pub fn circle_circle_intersection(
        &self,
        circle_a: &Circle2,
        circle_b: &Circle2,
    ) -> Intersection {
        let center_a = self.points.get(&circle_a.center).unwrap();
        let center_b = self.points.get(&circle_b.center).unwrap();
        let r_a = circle_a.radius;
        let r_b = circle_b.radius;

        // compute distance between centers
        let center_dx = center_b.x - center_a.x;
        let center_dy = center_b.y - center_a.y;
        let center_dist = center_dx.hypot(center_dy);

        // if the circles are too far away OR too close, they don't intersect
        if center_dist > r_a + r_b {
            return Intersection::None;
        }
        if center_dist < (r_a - r_b).abs() {
            return Intersection::None;
        }

        let r_2 = center_dist * center_dist;
        let r_4 = r_2 * r_2;
        let a = (r_a * r_a - r_b * r_b) / (2.0 * r_2);
        let r_2_r_2 = r_a * r_a - r_b * r_b;
        let c = (2.0 * (r_a * r_a + r_b * r_b) / r_2 - r_2_r_2 * r_2_r_2 / r_4 - 1.0).sqrt();

        let fx = (center_a.x + center_b.x) / 2.0 + a * (center_b.x - center_a.x);
        let gx = c * (center_b.y - center_a.y) / 2.0;
        let ix1 = fx + gx;
        let ix2 = fx - gx;

        let fy = (center_a.y + center_b.y) / 2.0 + a * (center_b.y - center_a.y);
        let gy = c * (center_a.x - center_b.x) / 2.0;
        let iy1 = fy + gy;
        let iy2 = fy - gy;

        Intersection::TwoPoints(Point2::new(ix1, iy1), false, Point2::new(ix2, iy2), false)
    }

    pub fn circle_arc_intersection(&self, circle: &Circle2, arc: &Arc2) -> Intersection {
        // treat this is circle/circle intersection, then just do some checks
        // afterwards to make sure the intersection points really do fall within
        // the bounds of the arc
        let arc_center = self.points.get(&arc.center).unwrap();
        let arc_start = self.points.get(&arc.start).unwrap();
        let arc_dx = arc_center.x - arc_start.x;
        let arc_dy = arc_center.y - arc_start.y;
        let arc_radius = arc_dx.hypot(arc_dy);
        let fake_circle = Circle2 {
            center: arc.center,
            radius: arc_radius,
            top: arc.start,
        };

        let fake_intersection = self.circle_circle_intersection(circle, &fake_circle);
        println!("Fake intersection: {:?}", fake_intersection);

        match fake_intersection {
            Intersection::None => Intersection::None,
            Intersection::OnePoint(_, _) => todo!(),
            Intersection::TwoPoints(_, _, _, _) => {
                // check to make sure that both points fall within the arc. If only one
                // of them does, just return that one. if none do, return none.
                // if both do, return both.
                Intersection::None
            }
            Intersection::Line(_, _) => todo!(),
            Intersection::Arc(_) => todo!(),
            Intersection::Circle(_) => todo!(),
        }
    }

    pub fn arc_arc_intersection(&self, arc_a: &Arc2, arc_b: &Arc2) -> Intersection {
        // treat this is circle/circle intersection, then just do some checks
        // afterwards to make sure the intersection points really do fall within
        // the bounds of the arcs

        Intersection::None
    }

    pub fn shape_intersection(&self, shape_a: &Shape, shape_b: &Shape) -> Intersection {
        match (shape_a, shape_b) {
            (Shape::Line(line_a), Shape::Line(line_b)) => {
                self.line_line_intersection(&line_a, &line_b)
            }
            (Shape::Circle(circle_a), Shape::Circle(circle_b)) => {
                self.circle_circle_intersection(&circle_a, &circle_b)
            }
            (Shape::Circle(circle), Shape::Arc(arc)) => self.circle_arc_intersection(&circle, &arc),
            _ => Intersection::None,
        }
    }

    pub fn split_intersections(&self) -> Self {
        let mut temp_sketch = self.clone();

        // First put all segments: Arcs, Lines, Circles into one big collection
        let mut all_shapes: IncrementingMap<Shape> = IncrementingMap::new();

        let line_ids: Vec<u64> = temp_sketch.line_segments.keys().cloned().sorted().collect();
        for line_id in line_ids {
            let line = temp_sketch.line_segments.get(&line_id).unwrap();
            all_shapes.add_item(Shape::Line(line.clone()));
        }

        let circle_ids: Vec<u64> = temp_sketch.circles.keys().cloned().sorted().collect();
        for circle_id in circle_ids {
            let circle = temp_sketch.circles.get(&circle_id).unwrap();
            all_shapes.add_item(Shape::Circle(circle.clone()));
        }

        let arc_ids: Vec<u64> = temp_sketch.arcs.keys().cloned().sorted().collect();
        for arc_id in arc_ids {
            let arc = temp_sketch.arcs.get(&arc_id).unwrap();
            all_shapes.add_item(Shape::Arc(arc.clone()));
        }

        // println!("JUST debugging 0 vs 5");
        // let intersection = temp_sketch.shape_intersection(
        //     all_shapes.items.get(&0).unwrap(),
        //     all_shapes.items.get(&5).unwrap(),
        // );
        // println!("Intersection: {:?}", intersection);

        // return temp_sketch;

        // then compare each one against each other one (once, skipping itself)
        // let mut all_intersections: Vec<(u64, u64, Intersection)> = vec![];
        let mut all_intersections: VecDeque<(u64, u64, Intersection)> = VecDeque::new();
        for (shape_a_id, shape_a) in all_shapes.items.iter() {
            for (shape_b_id, shape_b) in all_shapes.items.iter() {
                if (shape_a_id >= shape_b_id) {
                    continue;
                }

                let intersection = temp_sketch.shape_intersection(shape_a, shape_b);

                // if *shape_a_id == 11 && *shape_b_id == 13 {
                //     println!("What's up at 11, 13?");
                //     println!("{:?}", intersection);
                // }

                match intersection {
                    Intersection::None => {}
                    Intersection::OnePoint(_, true) => {}
                    _ => {
                        println!("Got a real intersection");
                        all_intersections.push_back((*shape_a_id, *shape_b_id, intersection));
                    }
                }
            }
        }

        println!("\nHave intersections, now splitting shapes");
        for i in all_intersections.iter() {
            println!("{:?}", i);
        }
        loop {
            if all_intersections.len() == 0 {
                break;
            }
            let intersection_tuple = all_intersections.pop_front().unwrap();

            println!(
                "Now considering this intersection:\n{:?}",
                intersection_tuple
            );

            println!("Leaving the following:");
            for inters in all_intersections.iter() {
                println!("{:?}", inters);
            }

            let (shape_a_id, shape_b_id, intersection) = intersection_tuple;
            let shape_a = all_shapes.get_item(shape_a_id).unwrap().clone();
            let shape_b = all_shapes.get_item(shape_b_id).unwrap().clone();

            match intersection {
                Intersection::None => {}
                Intersection::OnePoint(point, _) => {
                    // this code currently assumes line vs line intersection. need to
                    // move this under a match statement
                    let new_point_id = temp_sketch.add_point(point.x, point.y);
                    println!("Add an intersection point with ID: {:?}", new_point_id);

                    let (new_shape_one, new_shape_two) = shape_a.split_at_point_id(new_point_id);
                    let (new_shape_three, new_shape_four) = shape_b.split_at_point_id(new_point_id);

                    let mut new_shapes: Vec<u64> = vec![];
                    new_shapes.push(all_shapes.add_item(new_shape_one.clone()));
                    new_shapes.push(all_shapes.add_item(new_shape_two.clone()));
                    new_shapes.push(all_shapes.add_item(new_shape_three.clone()));
                    new_shapes.push(all_shapes.add_item(new_shape_four.clone()));
                    all_shapes.remove_item(shape_a_id);
                    all_shapes.remove_item(shape_b_id);

                    println!(
                        "Adding new shapes: {:?}, {:?}, {:?}, {:?}",
                        new_shapes.get(new_shapes.len() - 4),
                        new_shapes.get(new_shapes.len() - 3),
                        new_shapes.get(new_shapes.len() - 2),
                        new_shapes.get(new_shapes.len() - 1),
                    );

                    println!("Removing shapes: {:?}, {:?}", shape_a_id, shape_b_id);

                    // now we have to sweep through all the existing intersections and remove any that reference the
                    // ids of the shapes we just removed, adding each one to a list of possible new intersections to check for
                    let mut possibilities: Vec<u64> = vec![];
                    let mut indices_to_remove: Vec<usize> = vec![];
                    for (index, (a, b, _)) in all_intersections.iter().enumerate() {
                        if *a == shape_a_id || *a == shape_b_id {
                            possibilities.push(*b);
                            indices_to_remove.push(index);
                        }
                        if *b == shape_a_id || *b == shape_b_id {
                            possibilities.push(*a);
                            indices_to_remove.push(index);
                        }
                    }

                    println!("New possibilities: {:?}", possibilities);
                    println!("Indices to remove: {:?}", indices_to_remove);

                    for index_to_remove in indices_to_remove.iter().rev() {
                        all_intersections.remove(*index_to_remove);
                    }
                    println!("All intersections after removing stale ones:");
                    for existing_intersection in all_intersections.iter() {
                        println!("{:?}", existing_intersection);
                    }
                    println!("that was all of them");

                    for possibility_id in possibilities {
                        for new_line_id in new_shapes.iter() {
                            // println!("Checking {:?} against {:?}", possibility_id, new_line_id);
                            let possibility = all_shapes.get_item(possibility_id).unwrap();
                            let new_line = all_shapes.get_item(*new_line_id).unwrap();
                            let intersection =
                                temp_sketch.shape_intersection(possibility, new_line);

                            match intersection {
                                Intersection::None => {}
                                Intersection::OnePoint(_, true) => {}
                                _ => {
                                    all_intersections.push_back((
                                        possibility_id,
                                        *new_line_id,
                                        intersection,
                                    ));
                                    // println!("  hit!");
                                }
                            }
                        }
                    }
                }
                Intersection::Line(point_a, point_b) => {
                    println!("Intersection line: {:?} {:?}", point_a, point_b);
                }
                Intersection::TwoPoints(point_a, false, point_b, false) => {
                    match (shape_a, shape_b) {
                        (Shape::Circle(circle_a), Shape::Circle(circle_b)) => {
                            // we need to add two new points, one for each of these intersections
                            let new_point_0 = temp_sketch.add_point(point_a.x, point_a.y);
                            let new_point_1 = temp_sketch.add_point(point_b.x, point_b.y);

                            // then break each circle up into two arcs:
                            let arc_a_0 = Arc2 {
                                center: circle_a.center,
                                start: new_point_0,
                                end: new_point_1,
                                clockwise: false,
                            };
                            let arc_a_1 = Arc2 {
                                center: circle_a.center,
                                start: new_point_1,
                                end: new_point_0,
                                clockwise: false,
                            };
                            let arc_b_0 = Arc2 {
                                center: circle_b.center,
                                start: new_point_0,
                                end: new_point_1,
                                clockwise: false,
                            };
                            let arc_b_1 = Arc2 {
                                center: circle_b.center,
                                start: new_point_1,
                                end: new_point_0,
                                clockwise: false,
                            };

                            // add these four new arcs
                            let mut new_shapes: Vec<u64> = vec![];
                            new_shapes.push(all_shapes.add_item(Shape::Arc(arc_a_0)));
                            new_shapes.push(all_shapes.add_item(Shape::Arc(arc_a_1)));
                            new_shapes.push(all_shapes.add_item(Shape::Arc(arc_b_0)));
                            new_shapes.push(all_shapes.add_item(Shape::Arc(arc_b_1)));

                            // remove the two circles
                            all_shapes.remove_item(shape_a_id);
                            all_shapes.remove_item(shape_b_id);

                            // now we have to sweep through all the existing intersections and remove any that reference the
                            // ids of the circles we just removed, adding each one to a list of possible new intersections to check for
                            let mut possibilities: Vec<u64> = vec![];
                            let mut indices_to_remove: Vec<usize> = vec![];
                            for (index, (a, b, _)) in all_intersections.iter().enumerate() {
                                if *a == shape_a_id || *a == shape_b_id {
                                    possibilities.push(*b);
                                    indices_to_remove.push(index);
                                }
                                if *b == shape_a_id || *b == shape_b_id {
                                    possibilities.push(*a);
                                    indices_to_remove.push(index);
                                }
                            }

                            println!("New possibilities: {:?}", possibilities);
                            println!("Indices to remove: {:?}", indices_to_remove);

                            for index_to_remove in indices_to_remove.iter().rev() {
                                all_intersections.remove(*index_to_remove);
                            }
                            println!("All intersections after removing stale ones:");
                            for existing_intersection in all_intersections.iter() {
                                println!("{:?}", existing_intersection);
                            }
                            println!("that was all of them");

                            for possibility_id in possibilities {
                                for new_arc_id in new_shapes.iter() {
                                    println!(
                                        "Checking {:?} against {:?}",
                                        possibility_id, new_arc_id
                                    );
                                    let possibility = all_shapes.get_item(possibility_id).unwrap();
                                    let new_line = all_shapes.get_item(*new_arc_id).unwrap();
                                    let intersection =
                                        temp_sketch.shape_intersection(possibility, new_line);

                                    println!("intersection: {:?}", intersection);

                                    // match intersection {
                                    //     Intersection::None => {}
                                    //     Intersection::OnePoint(_, true) => {}
                                    //     _ => {
                                    //         all_intersections.push_back((
                                    //             possibility_id,
                                    //             *new_arc_id,
                                    //             intersection,
                                    //         ));
                                    //         // println!("  hit!");
                                    //     }
                                    // }
                                }
                            }
                        }
                        (_, _) => todo!(),
                    }
                }
                Intersection::TwoPoints(point_a, _, point_b, _) => todo!(),
                Intersection::Arc(_) => todo!(),
                Intersection::Circle(_) => todo!(),
            }

            if all_intersections.len() == 0 {
                break;
            }
        }

        // println!("All Shapes at the end: {:?}", all_shapes.items);

        let mut final_sketch = Sketch::new();
        final_sketch.points = temp_sketch.points;
        final_sketch.highest_point_id = temp_sketch.highest_point_id;
        for shape in all_shapes.items.iter() {
            match shape {
                (id, Shape::Line(line)) => {
                    final_sketch.add_segment(line.start, line.end);
                }
                (id, Shape::Circle(circle)) => {
                    final_sketch.add_circle(circle.center, circle.radius);
                }
                (id, Shape::Arc(arc)) => {
                    final_sketch.add_arc(arc.center, arc.start, arc.end, arc.clockwise);
                }
                _ => {}
            }
        }

        println!("So, in summary I've generated these shapes:");
        for shape in all_shapes.items.iter() {
            println!("{:?}", shape);
        }

        return final_sketch;

        // First compare every line segment against every line segment to see if they intersect
        // TODO: this doesn't correctly handle when one line crosses *two* others
        let mut count = 0;
        let mut intersections: Vec<(u64, u64, Point2)> = vec![];

        // for (i, (line_a_id, line_a)) in temp_sketch.line_segments.iter().enumerate() {
        for (i, line_a_id) in temp_sketch.line_segments.keys().sorted().enumerate() {
            let line_a = temp_sketch.line_segments.get(line_a_id).unwrap();

            // for (j, (line_b_id, line_b)) in temp_sketch.line_segments.iter().enumerate() {
            for (j, line_b_id) in temp_sketch.line_segments.keys().sorted().enumerate() {
                let line_b = temp_sketch.line_segments.get(&line_b_id).unwrap();
                // we only need to compare each line segment to each other line segment once
                // so we can skip indices where i > j.
                // Also, every line intersects itself so no need to check that.
                if i >= j {
                    continue;
                }

                // line segments which share a point do intersect, but there's nothing to be done
                // so let's just skip
                if line_a.start == line_b.start
                    || line_a.start == line_b.end
                    || line_a.end == line_b.end
                    || line_a.end == line_b.start
                {
                    continue;
                }

                count += 1;

                let intersection = temp_sketch.line_intersection(line_a, line_b);
                match intersection {
                    None => {}
                    Some(point) => {
                        intersections.push((*line_a_id, *line_b_id, point));
                    }
                }
            }
        }

        for (line_a_id, line_b_id, point) in intersections {
            println!("Intersection ids: {} {}", line_a_id, line_b_id);

            let line_a = temp_sketch.line_segments.get(&line_a_id).unwrap().clone();
            let line_b = temp_sketch.line_segments.get(&line_b_id).unwrap().clone();

            let new_point_id = temp_sketch.add_point(point.x, point.y);
            temp_sketch.add_segment(line_a.start, new_point_id);
            temp_sketch.add_segment(new_point_id, line_a.end);
            temp_sketch.add_segment(line_b.start, new_point_id);
            temp_sketch.add_segment(new_point_id, line_b.end);

            temp_sketch.line_segments.remove(&line_a_id);
            temp_sketch.line_segments.remove(&line_b_id);
        }

        // Second, compare every circle against every other circle to see if they intersect
        let mut circle_intersections: Vec<(u64, u64, Vec<Point2>)> = vec![];

        // for (i, (circle_a_id, circle_a)) in temp_sketch.circles.iter().enumerate() {
        for (i, circle_a_id) in temp_sketch.circles.keys().sorted().enumerate() {
            let circle_a = temp_sketch.circles.get(&circle_a_id).unwrap();

            // for (j, (circle_b_id, circle_b)) in temp_sketch.circles.iter().enumerate() {
            for (j, circle_b_id) in temp_sketch.circles.keys().sorted().enumerate() {
                let circle_b = temp_sketch.circles.get(&circle_b_id).unwrap();

                // we only need to compare each circle to each other circle once
                // so we can skip indices where i > j.
                // Also, every line intersects itself so no need to check that.
                if i >= j {
                    continue;
                }

                // circles which share a point do intersect, but there's nothing to be done
                // so let's just skip
                if circle_a.center == circle_b.center {
                    continue;
                }

                count += 1;

                let intersection = temp_sketch.circle_intersection(circle_a, circle_b);
                // If the circles intersect, then we'll need to do some splitting
                if intersection.len() > 0 {
                    circle_intersections.push((*circle_a_id, *circle_b_id, intersection));
                }
            }
        }

        println!("Found {} intersections", circle_intersections.len());
        for (circle_a_id, circle_b_id, points) in circle_intersections {
            // TODO: check for duplicates! Things get hairy if 3 circles intersect at the same point!
            let circle_a = temp_sketch.circles.get(&circle_a_id).unwrap().clone();
            let circle_b = temp_sketch.circles.get(&circle_b_id).unwrap().clone();

            let center_a = temp_sketch.points.get(&circle_a.center).unwrap().clone();
            let center_b = temp_sketch.points.get(&circle_b.center).unwrap().clone();

            println!(
                "Circle A: {:?} centered: at {}, {}",
                circle_a, center_a.x, center_a.y
            );
            println!(
                "Circle B: {:?} centered: at {}, {}",
                circle_b, center_b.x, center_b.y
            );

            let new_point_0 = temp_sketch.add_point(points[0].x, points[0].y);
            let new_point_1 = temp_sketch.add_point(points[1].x, points[1].y);

            println!(
                "Intersections at: {}: ({}, {}) and {}: ({}, {})",
                new_point_0, points[0].x, points[0].y, new_point_1, points[1].x, points[1].y
            );

            temp_sketch.add_arc(circle_a.center, new_point_0, new_point_1, false);
            temp_sketch.add_arc(circle_a.center, new_point_1, new_point_0, false);

            temp_sketch.add_arc(circle_b.center, new_point_0, new_point_1, false);
            temp_sketch.add_arc(circle_b.center, new_point_1, new_point_0, false);

            temp_sketch.circles.remove(&circle_a_id);
            temp_sketch.circles.remove(&circle_b_id);

            println!(
                "So in the end, temp sketch has: {} circles, {} arcs, {} segments",
                temp_sketch.circles.len(),
                temp_sketch.arcs.len(),
                temp_sketch.line_segments.len()
            );

            for arc_id in temp_sketch.arcs.keys().sorted() {
                let arc = temp_sketch.arcs.get(arc_id).unwrap();
                print!("Arc: {} ", arc_id);
                temp_sketch.pretty_print_arc(arc);
            }
        }

        temp_sketch
    }

    pub fn find_faces(&self) -> (Vec<Face>, Vec<Segment>) {
        let mut segments_overall: Vec<Segment> = vec![];

        for line_id in self.line_segments.keys().sorted() {
            let line = self.line_segments.get(line_id).unwrap();
            segments_overall.push(Segment::Line(line.clone()));
        }
        for arc_id in self.arcs.keys().sorted() {
            let arc = self.arcs.get(arc_id).unwrap();
            segments_overall.push(Segment::Arc(arc.clone()));
        }

        let (rings, unused_segments) = self.find_rings(segments_overall, false);
        // println!("Found {} rings", rings.len());
        // for ring in &rings {
        //     println!("{:?}", ring);
        // }
        // println!("Found {} unused segments", unused_segments.len());
        let mut faces: Vec<Face> = rings.iter().map(|r| Face::from_ring(r)).collect();

        if rings.len() == 0 {
            return (faces, unused_segments);
        }

        // this next block of code converts everything to Polygons just so we can
        // determine what faces contain which other faces. It's a bit of a waste
        // because geo is a relatively heavy dependency and we don't need all of it
        let polygons: Vec<Polygon> = rings.iter().map(|r| self.as_polygon(r)).collect();
        // they are already sorted from smallest to largest area - self.find_rings does this
        let mut what_contains_what: Vec<(usize, usize)> = vec![];

        for smaller_polygon_index in 0..polygons.len() - 1 {
            let smaller_polygon = &polygons[smaller_polygon_index];

            for bigger_polygon_index in smaller_polygon_index + 1..polygons.len() {
                let bigger_polygon = &polygons[bigger_polygon_index];
                let inside = bigger_polygon.contains(smaller_polygon);

                if inside {
                    what_contains_what.push((bigger_polygon_index, smaller_polygon_index));
                    break;
                }
            }
        }

        // cool, now we know what faces contain which other faces. Let's just add the holes
        for (bigger_index, smaller_index) in what_contains_what {
            let smaller_face = &faces[smaller_index].clone();
            faces[bigger_index].add_hole(smaller_face)
        }

        // let faces: Vec<Face> = polygons.iter().map(|p| Face::from_polygon(p)).collect();
        (faces, unused_segments)
    }

    pub fn find_rings(&self, segments: Vec<Segment>, debug: bool) -> (Vec<Ring>, Vec<Segment>) {
        // We are handed all of the segments to consider
        let mut segments_overall = segments.clone();
        let num_segments = segments_overall.len();

        // Let's double it by reversing each one and adding it to the list of
        // segments to consider
        let segments_reversed: Vec<Segment> =
            segments_overall.iter().map(|s| s.reverse()).collect();
        segments_overall.extend(segments_reversed);

        // keep track of every index we've already used--each segment can only be used once
        let mut used_indices: Vec<usize> = vec![];
        // this is the output variable
        let mut new_rings: Vec<Vec<usize>> = vec![];

        for (seg_idx, s) in segments_overall.iter().enumerate() {
            if debug {
                // println!("Starting a loop with segment: {:?}", s);
                match s {
                    Segment::Line(line) => {
                        println!(
                            "Line: ({}, {}) to ({}, {})",
                            self.points.get(&line.start).unwrap().x,
                            self.points.get(&line.start).unwrap().y,
                            self.points.get(&line.end).unwrap().x,
                            self.points.get(&line.end).unwrap().y
                        );
                    }
                    Segment::Arc(arc) => {
                        println!(
                            "Arc: center: ({}, {}), start: ({}, {}), end: ({}, {})",
                            self.points.get(&arc.center).unwrap().x,
                            self.points.get(&arc.center).unwrap().y,
                            self.points.get(&arc.start).unwrap().x,
                            self.points.get(&arc.start).unwrap().y,
                            self.points.get(&arc.end).unwrap().x,
                            self.points.get(&arc.end).unwrap().y
                        );
                    }
                }
            }
            if used_indices.contains(&seg_idx) {
                if debug {
                    println!("Skipping because it's been used");
                }
                continue;
            }
            // this variable will accumulate the indices of our new ring
            let mut new_ring_indices: Vec<usize> = vec![];
            let starting_point = s.get_start();
            if debug {
                println!("Starting point: {:?}", starting_point);
            }

            let mut next_segment_index: usize = seg_idx;
            for _i in 1..segments_overall.len() {
                let next_segment = segments_overall.get(next_segment_index).unwrap();
                if debug {
                    println!("next segment: {:?}", next_segment);
                }
                new_ring_indices.push(next_segment_index);

                match self.find_next_segment_index(
                    &segments_overall,
                    next_segment,
                    &used_indices,
                    debug,
                ) {
                    None => {
                        if debug {
                            println!("\tno viable next segments!");
                        }
                        break;
                    }
                    Some(idx) => next_segment_index = idx,
                }
                if next_segment.get_end() == starting_point {
                    if debug {
                        println!("\tomg finished!");
                        println!("\tring indices: {:?}", new_ring_indices);
                    }
                    new_rings.push(new_ring_indices.clone());
                    used_indices.extend(new_ring_indices);
                    break;
                }
            }
        }

        let used_indices_set = used_indices.iter().cloned().collect::<HashSet<_>>();
        let all_indices_set = (0..segments_overall.len()).collect::<HashSet<_>>();

        let unused_indices_set = all_indices_set
            .difference(&used_indices_set)
            .collect::<HashSet<_>>();
        let unused_indices = unused_indices_set
            .iter()
            .cloned()
            .filter(|idx| return *idx < &num_segments)
            .collect::<Vec<_>>();
        let unused_segments = unused_indices
            .iter()
            .cloned()
            .map(|idx| segments_overall.get(*idx).unwrap().clone())
            .collect::<Vec<_>>();

        let mut all_rings: Vec<Ring> = vec![];
        for ring_indices in new_rings.iter() {
            // let mut this_ring: Ring = Ring::Segments(vec![]);
            let mut this_ring: Vec<Segment> = vec![];
            for segment_index in ring_indices {
                let actual_segment: &Segment = segments_overall.get(*segment_index).unwrap();
                this_ring.push(actual_segment.clone());
            }
            all_rings.push(Ring::Segments(this_ring));
        }

        // println!("--Found {} rings", all_rings.len());

        // Circles are trivially rings!
        for (_circle_id, circle) in self.circles.iter() {
            all_rings.push(Ring::Circle(circle.clone()));
        }

        all_rings.sort_by(|r1, r2| {
            // TODO: implement signed_area for a ring which is made of arcs
            self.signed_area(r1)
                .partial_cmp(&self.signed_area(r2))
                .unwrap()
        });

        // filter out to only the positive-valued ones
        all_rings = all_rings
            .iter()
            .filter(|r| self.signed_area(r) > 0.0)
            .cloned()
            .collect();

        // println!("--Found {} rings", all_rings.len());

        (all_rings, unused_segments)
    }

    pub fn find_next_segment_index(
        &self,
        segments: &Vec<Segment>,
        starting_segment: &Segment,
        used_indices: &Vec<usize>,
        debug: bool,
    ) -> Option<usize> {
        // println!("Finding next segment index");
        let mut matches: Vec<(usize, f64, f64)> = vec![];
        let mut this_segment_end_angle = match starting_segment {
            Segment::Line(line) => self.line_end_angle(line),
            Segment::Arc(arc) => self.arc_end_angle(arc),
        };
        this_segment_end_angle = (this_segment_end_angle + PI) % (2.0 * PI);

        for (idx, s2) in segments.iter().enumerate() {
            if used_indices.contains(&idx) {
                continue;
            }
            if s2.continues(&starting_segment) && !s2.equals_or_reverse_equals(&starting_segment) {
                let starting_angle = match s2 {
                    Segment::Line(line) => self.line_start_angle(line),
                    Segment::Arc(arc) => self.arc_start_angle(arc),
                };
                let angle_diff = angle_difference(this_segment_end_angle, starting_angle);
                matches.push((idx, starting_angle, angle_diff));
                // angle_diff measures how hard you'd have to turn left to continue the path from
                // starting_segment to s2, where a straight line would be 180, a left turn 270, a right turn 90.
                // This is important later because to make the smallest loops possible, we always want to be
                // turning left as hard as possible when finding rings.
            }
        }

        if matches.len() == 0 {
            None
        } else if matches.len() == 1 {
            Some(matches[0].0)
        } else {
            if debug {
                println!("\tMultiple options! Deciding which one to take...");
            }

            let mut best_option = 0;
            let mut hardest_left_turn = 0.0;
            for o in matches.iter() {
                // println!("Option: {:?}", segments.get(o.0).unwrap());
                // println!("Option: {} angle {}", o.0, o.1 * 180.0 / PI);
                // println!("Option: {}", o.2 * 180.0 / PI);
                // println!();

                if o.2 > hardest_left_turn {
                    hardest_left_turn = o.2;
                    best_option = o.0;
                }
            }
            // println!("Best option: {}", best_option);
            Some(best_option)
        }
    }

    pub fn circle_intersection(&self, circle_a: &Circle2, circle_b: &Circle2) -> Vec<Point2> {
        // See https://math.stackexchange.com/questions/256100/how-can-i-find-the-points-at-which-two-circles-intersect#comment4306998_1367732
        // See https://gist.github.com/jupdike/bfe5eb23d1c395d8a0a1a4ddd94882ac
        let center_a = self.points.get(&circle_a.center).unwrap();
        let center_b = self.points.get(&circle_b.center).unwrap();
        let r_a = circle_a.radius;
        let r_b = circle_b.radius;

        let center_dx = center_b.x - center_a.x;
        let center_dy = center_b.y - center_a.y;
        let center_dist = center_dx.hypot(center_dy);

        if !(center_dist <= r_a + r_b && center_dist >= r_a - r_b) {
            return vec![];
        }

        let r_2 = center_dist * center_dist;
        let r_4 = r_2 * r_2;
        let a = (r_a * r_a - r_b * r_b) / (2.0 * r_2);
        let r_2_r_2 = r_a * r_a - r_b * r_b;
        let c = (2.0 * (r_a * r_a + r_b * r_b) / r_2 - r_2_r_2 * r_2_r_2 / r_4 - 1.0).sqrt();

        let fx = (center_a.x + center_b.x) / 2.0 + a * (center_b.x - center_a.x);
        let gx = c * (center_b.y - center_a.y) / 2.0;
        let ix1 = fx + gx;
        let ix2 = fx - gx;

        let fy = (center_a.y + center_b.y) / 2.0 + a * (center_b.y - center_a.y);
        let gy = c * (center_a.x - center_b.x) / 2.0;
        let iy1 = fy + gy;
        let iy2 = fy - gy;

        vec![Point2::new(ix1, iy1), Point2::new(ix2, iy2)]
    }

    pub fn line_intersection(&self, line_a: &Line2, line_b: &Line2) -> Option<Point2> {
        let start_a = self.points.get(&line_a.start).unwrap();
        let end_a = self.points.get(&line_a.end).unwrap();
        let start_b = self.points.get(&line_b.start).unwrap();
        let end_b = self.points.get(&line_b.end).unwrap();

        let line_a = Line::new(
            geo::Coord {
                x: start_a.x,
                y: start_a.y,
            },
            geo::Coord {
                x: end_a.x,
                y: end_a.y,
            },
        );
        let line_b = Line::new(
            geo::Coord {
                x: start_b.x,
                y: start_b.y,
            },
            geo::Coord {
                x: end_b.x,
                y: end_b.y,
            },
        );

        let intersection = line_intersection(line_a, line_b);

        match intersection {
            Some(line_intersection) => match line_intersection {
                LineIntersection::SinglePoint {
                    intersection,
                    is_proper,
                } => Some(Point2::new(intersection.x, intersection.y)),
                LineIntersection::Collinear { intersection } => panic!("Collinear!"),
            },
            None => None,
        }
    }

    pub fn ring_to_data(&self, ring: &Ring, mut data: Data) -> Data {
        match ring {
            Ring::Circle(circle) => {
                let center = self.points.get(&circle.center).unwrap();
                let radius = circle.radius;
                data = data.move_to((center.x, -center.y + radius)); // starts at bottom
                data = data.elliptical_arc_to((
                    radius,
                    radius,
                    0.0,
                    0,
                    0,
                    center.x,
                    -center.y - radius,
                )); // arc to top

                data = data.elliptical_arc_to((
                    radius,
                    radius,
                    0.0,
                    0,
                    0,
                    center.x,
                    -center.y + radius,
                )); // arc back to bottom

                data
            }
            Ring::Segments(segments) => {
                let mut first = true;
                for segment in segments {
                    match segment {
                        Segment::Line(line) => {
                            let start = self.points.get(&line.start).unwrap();
                            let end = self.points.get(&line.end).unwrap();

                            if first {
                                data = data.move_to((start.x, -start.y));
                                first = false;
                            }
                            data = data.line_to((end.x, -end.y));
                        }
                        Segment::Arc(arc) => {
                            let center = self.points.get(&arc.center).unwrap();
                            let start = self.points.get(&arc.start).unwrap();
                            let end = self.points.get(&arc.end).unwrap();

                            let r = (center.x - start.x).hypot(center.y - start.y);

                            if first {
                                data = data.move_to((start.x, -start.y));
                                first = false;
                            }

                            let arc_angle_degrees = self.arc_angle(arc) * 180.0 / PI;
                            println!("arc_angle: {}", arc_angle_degrees);

                            // most small simple arcs should have this flag set to 0
                            let mut large_arc_flag = 0;
                            // most arcs are counterclockwise, so this flag is usually 0
                            let mut sweep_flag = 0;

                            if arc_angle_degrees > 180.0 {
                                println!("large arc flag!");
                                large_arc_flag = 1;
                            }

                            if arc.clockwise {
                                sweep_flag = 1;
                            }

                            //A rx ry x-axis-rotation large-arc-flag sweep-flag x y
                            data = data.elliptical_arc_to((
                                r,
                                r,
                                0.0,
                                large_arc_flag,
                                sweep_flag,
                                end.x,
                                -end.y,
                            ));
                        }
                    }
                }
                data
            }
        }
    }
}

pub fn arc_to_points(
    start: &Point2,
    end: &Point2,
    center: &Point2,
    clockwise: bool,
) -> Vec<Point2> {
    let r = (center.x - start.x).hypot(center.y - start.y);
    let circle_tolerance: f64 = 0.001; // in meters
    let k = circle_tolerance / r;
    let mut n = (PI / (2.0 * k).sqrt()).ceil() as i64;

    let segment_angle = (2.0 * PI) / n as f64;
    let segment_length = r * segment_angle;
    let start_angle = (start.y - center.y).atan2(start.x - center.x);

    let mut line_vertices: Vec<Point2> = vec![];
    line_vertices.push(Point2::new(start.x, start.y));

    if clockwise {
        n = -n;
    }

    for i in 1..n.abs() {
        let theta = ((2.0 * PI) / n as f64) * i as f64 + start_angle;
        let x_component = r * theta.cos();
        let y_component = r * theta.sin();
        let point = Point2::new(x_component + center.x, y_component + center.y);
        line_vertices.push(point.clone());

        let distance_to_end = point.distance_to(end);
        if (distance_to_end <= segment_length) {
            line_vertices.push(Point2::new(end.x, end.y));
            break;
        }
    }

    line_vertices
}

pub struct IncrementingMap<T> {
    pub items: IndexMap<u64, T>,
    next_id: u64,
}

impl<T> IncrementingMap<T> {
    pub fn new() -> Self {
        IncrementingMap {
            items: IndexMap::new(),
            next_id: 0,
        }
    }

    pub fn add_item(&mut self, item: T) -> u64 {
        let id = self.next_id;
        self.items.insert(id, item);
        self.next_id += 1;
        id
    }

    pub fn remove_item(&mut self, id: u64) {
        self.items.remove(&id);
    }

    pub fn get_item(&self, id: u64) -> Option<&T> {
        self.items.get(&id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Constraint {
    SegmentLength {
        segment_id: u64,
        length: f64,
        normal_offset: f64,
        parallel_offset: f64,
        kp: f64, // kp is the proportional gain, the spring constant
        kd: f64, // kd is the derivative gain, the damping constant
        error: f64,
    },
    SegmentAngle {
        segment_id: u64,
        angle: f64,
        x_offset: f64,
        y_offset: f64,
        kp: f64,
        kd: f64,
        error: f64,
    },
    CircleDiameter {
        circle_id: u64,
        diameter: f64,
        angle_offset: f64,
        r_offset: f64,
        kp: f64,
        kd: f64,
        error: f64,
    },
    SegmentsEqual {
        segment_a_id: u64,
        segment_b_id: u64,
        kp: f64,
        kd: f64,
        error: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
    m: f64,
    dx: f64,
    dy: f64,
    fx: f64,
    fy: f64,
    fixed: bool,
    pub hidden: bool,
}

impl Point2 {
    pub fn new(x: f64, y: f64) -> Self {
        Point2 {
            x,
            y,
            m: 1.0,
            dx: 0.0,
            dy: 0.0,
            fx: 0.0,
            fy: 0.0,
            fixed: false,
            hidden: false,
        }
    }

    pub fn new_fixed(x: f64, y: f64) -> Self {
        Point2 {
            x,
            y,
            m: 1.0,
            dx: 0.0,
            dy: 0.0,
            fx: 0.0,
            fy: 0.0,
            fixed: true,
            hidden: false,
        }
    }

    pub fn new_hidden(x: f64, y: f64) -> Self {
        Point2 {
            x,
            y,
            m: 1.0,
            dx: 0.0,
            dy: 0.0,
            fx: 0.0,
            fy: 0.0,
            fixed: false,
            hidden: true,
        }
    }

    fn reset_forces(&mut self) {
        self.fx = 0.0;
        self.fy = 0.0;
    }

    pub fn apply_drag_force(&mut self) {
        let drag_coefficient = 0.1;
        let drag_force = -drag_coefficient * self.dx;
        self.fx += drag_force;
        let drag_force = -drag_coefficient * self.dy;
        self.fy += drag_force;
    }

    fn step(&self, dt: f64) -> (f64, f64) {
        (self.x + self.dx * dt, self.y + self.dy * dt)
    }

    fn distance_to(&self, other: &Point2) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }

    fn angle_to(&self, other: &Point2) -> f64 {
        (other.y - self.y).atan2(other.x - self.x)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub fn new(x: f64, y: f64) -> Self {
        Vector2 { x, y }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Circle2 {
    pub center: u64,
    pub radius: f64,
    pub top: u64,
}

impl Circle2 {
    pub fn equals(&self, other: &Self) -> bool {
        self.center == other.center && self.radius == other.radius
    }

    pub fn canonical_string(&self) -> String {
        format!("{}-{}", self.center, self.radius)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Arc2 {
    pub center: u64,
    pub start: u64,
    pub end: u64,
    pub clockwise: bool,
}

impl Arc2 {
    pub fn reverse(&self) -> Self {
        Arc2 {
            center: self.center,
            start: self.end,
            end: self.start,
            clockwise: !self.clockwise,
        }
    }

    pub fn canonical_string(&self) -> String {
        if self.start < self.end {
            format!(
                "{}-{}-{}-{}",
                self.start, self.end, self.center, self.clockwise
            )
        } else {
            self.reverse().canonical_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Line2 {
    pub start: u64,
    pub end: u64,
}

impl Line2 {
    pub fn reverse(&self) -> Self {
        Line2 {
            start: self.end,
            end: self.start,
        }
    }

    pub fn canonical_string(&self) -> String {
        if self.start < self.end {
            format!("{}-{}", self.start, self.end)
        } else {
            format!("{}-{}", self.end, self.start)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Circle(Circle2),
    Arc(Arc2),
    Line(Line2),
}

impl Shape {
    pub fn split_at_point_id(&self, new_point_id: u64) -> (Shape, Shape) {
        match self {
            Shape::Line(line) => {
                let new_line_1 = Line2 {
                    start: line.start,
                    end: new_point_id,
                };
                let new_line_2 = Line2 {
                    start: new_point_id,
                    end: line.end,
                };
                (Shape::Line(new_line_1), Shape::Line(new_line_2))
            }
            Shape::Circle(circle) => todo!(),
            Shape::Arc(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Intersection {
    None,
    OnePoint(Point2, bool),
    TwoPoints(Point2, bool, Point2, bool),
    Line(Point2, Point2),
    Arc(Arc2),
    Circle(Circle2),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Segment {
    Line(Line2),
    Arc(Arc2),
}

impl Segment {
    pub fn reverse(&self) -> Self {
        match self {
            Segment::Line(line) => Segment::Line(line.reverse()),
            Segment::Arc(arc) => Segment::Arc(arc.reverse()),
        }
    }

    pub fn get_start(&self) -> u64 {
        match self {
            Segment::Line(line) => line.start,
            Segment::Arc(arc) => arc.start,
        }
    }

    pub fn get_end(&self) -> u64 {
        match self {
            Segment::Line(line) => line.end,
            Segment::Arc(arc) => arc.end,
        }
    }

    pub fn continues(&self, prior_segment: &Segment) -> bool {
        // determines if this segment continues the prior segment
        prior_segment.get_end() == self.get_start()
    }

    pub fn equals_or_reverse_equals(&self, other: &Self) -> bool {
        self == other || self == &other.reverse()
    }

    pub fn reverse_equals(&self, other: &Self) -> bool {
        self == &other.reverse()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ring {
    Circle(Circle2),
    Segments(Vec<Segment>),
}

impl Ring {
    pub fn adjacent_edges(&self, other: &Self) -> Option<(Vec<usize>, Vec<usize>)> {
        match (self, other) {
            (Ring::Segments(segments_a), Ring::Segments(segments_b)) => {
                let mut edge_indices_a: Vec<usize> = vec![];
                let mut edge_indices_b: Vec<usize> = vec![];
                for (index_a, segment_a) in segments_a.iter().enumerate() {
                    for (index_b, segment_b) in segments_b.iter().enumerate() {
                        if segment_a.reverse_equals(segment_b) {
                            edge_indices_a.push(index_a);
                            edge_indices_b.push(index_b);
                        }
                    }
                }
                if edge_indices_a.len() == 0 {
                    return None;
                } else {
                    Some((edge_indices_a, edge_indices_b))
                }
            }
            _ => None,
        }
    }

    pub fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Ring::Circle(circle_a), Ring::Circle(circle_b)) => circle_a.equals(circle_b),
            (Ring::Segments(segments_a), Ring::Segments(segments_b)) => {
                segments_a.len() == segments_b.len()
                    && segments_a
                        .iter()
                        .zip(segments_b.iter())
                        .all(|(a, b)| a == b)
            }
            _ => false,
        }
    }

    pub fn canonical_form(&self) -> Self {
        // sort the segments in order by first finding the segment with the smallest start point
        // and then rotating the list so that that segment is first
        match self {
            Ring::Circle(circle) => Ring::Circle(circle.clone()),
            Ring::Segments(segments) => {
                let mut canonical_segments: Vec<Segment> = vec![];
                let mut min_index = 0;
                let mut min_segment = segments.get(0).unwrap();
                for (i, segment) in segments.iter().enumerate() {
                    if segment.get_start() < min_segment.get_start() {
                        min_index = i;
                        min_segment = segment;
                    }
                }

                for i in 0..segments.len() {
                    canonical_segments.push(
                        segments
                            .get((i + min_index) % segments.len())
                            .unwrap()
                            .clone(),
                    );
                }

                Ring::Segments(canonical_segments)
            }
        }
    }

    pub fn reverse(&self) -> Self {
        match self {
            Ring::Circle(circle) => Ring::Circle(circle.clone()),
            Ring::Segments(segments) => {
                let mut reversed_segments: Vec<Segment> = vec![];
                for segment in segments.iter().rev() {
                    reversed_segments.push(segment.reverse());
                }
                Ring::Segments(reversed_segments)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Face {
    pub exterior: Ring,
    pub holes: Vec<Ring>,
}

impl Face {
    pub fn from_ring(ring: &Ring) -> Face {
        Face {
            exterior: ring.clone(),
            holes: vec![],
        }
    }

    pub fn add_hole(&mut self, hole: &Face) {
        self.holes.push(hole.exterior.clone());
    }
}

pub fn angle(a: &Point2, b: &Point2, c: &Point2) -> f64 {
    // output range is (0, 2*PI]
    let ba_dx: f64 = a.x - b.x;
    let ba_dy: f64 = a.y - b.y;
    let ba_angle: f64 = ba_dy.atan2(ba_dx);

    let bc_dx = c.x - b.x;
    let bc_dy = c.y - b.y;
    let bc_angle = bc_dy.atan2(bc_dx);

    let mut naive_angle = bc_angle - ba_angle;
    if naive_angle <= 0.0 {
        naive_angle += TAU;
    }
    naive_angle
}

pub fn min_angle_diff(a0: f64, a1: f64) -> f64 {
    let path_a = angle_difference(a0, a1);
    let path_b = angle_difference(a1, a0);
    if path_a < path_b {
        path_a
    } else {
        path_b
    }
}

pub fn angle_difference(mut a0: f64, mut a1: f64) -> f64 {
    if a0 > TAU {
        a0 -= TAU;
    }
    if a0 < 0.0 {
        a0 += TAU;
    }

    if a1 > TAU {
        a1 -= TAU;
    }
    if a1 < 0.0 {
        a1 += TAU;
    }

    let mut naive_diff = a1 - a0;
    if naive_diff > TAU {
        naive_diff -= TAU;
    }
    if naive_diff < 0.0 {
        naive_diff += TAU;
    }

    naive_diff
}

#[cfg(test)]
mod tests {
    use crate::project::Project;

    use super::*;

    #[test]
    fn arc_to_points_90() {
        let mut sketch = Sketch::new();
        let center = sketch.add_point(0.0, 0.0);
        let start = sketch.add_point(1.0, 0.0);
        let end = sketch.add_point(0.0, 1.0);
        let arc_id = sketch.add_arc(center, start, end, false);
        let arc = sketch.arcs.get(&arc_id).unwrap();

        let points = sketch.arc_to_points(&arc);
        assert_eq!(points.len(), 19);
    }

    #[test]
    fn arc_to_points_neg_90() {
        let mut sketch = Sketch::new();
        let center = sketch.add_point(0.0, 0.0);
        let start = sketch.add_point(0.0, 1.0);
        let end = sketch.add_point(1.0, 0.0);
        let arc_id = sketch.add_arc(center, start, end, true);
        let arc = sketch.arcs.get(&arc_id).unwrap();

        let points = sketch.arc_to_points(&arc);
        assert_eq!(points.len(), 19);

        for point in points {
            println!("Point: ({}, {})", point.x, point.y);
        }
    }

    #[test]
    fn arc_to_points_180() {
        let mut sketch = Sketch::new();
        let center = sketch.add_point(0.0, 0.0);
        let start = sketch.add_point(1.0, 0.0);
        let end = sketch.add_point(-1.0, 0.0);
        let arc_id = sketch.add_arc(center, start, end, false);
        let arc = sketch.arcs.get(&arc_id).unwrap();

        let points = sketch.arc_to_points(&arc);
        assert_eq!(points.len(), 37);
    }

    #[test]
    fn arc_to_points70() {
        let mut sketch = Sketch::new();
        let center = sketch.add_point(0.0, 0.0);
        let start = sketch.add_point(1.0, 0.0);
        let end = sketch.add_point(0.0, -1.0);
        let arc_id = sketch.add_arc(center, start, end, false);
        let arc = sketch.arcs.get(&arc_id).unwrap();

        let points = sketch.arc_to_points(&arc);
        assert_eq!(points.len(), 55);
    }

    #[test]
    fn empty_to_svg() {
        let mut sketch = Sketch::new();
        sketch.save_svg("test_svgs/empty.svg");
    }

    #[test]
    fn no_rings_to_svg() {
        let mut sketch = Sketch::new();

        let center = sketch.add_point(0.0, 0.0);
        let right = sketch.add_point(1.0, 0.0);
        let top = sketch.add_point(0.0, 1.0);
        let left = sketch.add_point(-1.0, 0.0);
        let bottom = sketch.add_point(0.0, -1.0);

        sketch.add_segment(center, right);
        sketch.add_segment(center, top);
        sketch.add_segment(center, left);
        sketch.add_segment(center, bottom);

        sketch.save_svg("test_svgs/no_rings.svg");
    }

    #[test]
    fn circle_to_svg() {
        let mut sketch = Sketch::new();

        let id0 = sketch.add_point(1.0, 0.0);
        sketch.add_circle(id0, 1.0);

        sketch.save_svg("test_svgs/circle.svg");
    }

    #[test]
    fn square_to_svg() {
        let mut sketch = Sketch::new();

        let id0 = sketch.add_point(0.0, 0.0);
        let id1 = sketch.add_point(1.0, 0.0);
        let id2 = sketch.add_point(1.0, 1.0);
        let id3 = sketch.add_point(0.0, 1.0);

        sketch.add_segment(id0, id1);
        sketch.add_segment(id1, id2);
        sketch.add_segment(id2, id3);
        sketch.add_segment(id3, id0);

        sketch.save_svg("test_svgs/square.svg");
    }

    #[test]
    fn rounded_square_to_svg() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.25, 0.0);
        let b = sketch.add_point(0.75, 0.0);
        let c = sketch.add_point(1.0, 0.25);
        let d = sketch.add_point(1.0, 0.75);
        let e = sketch.add_point(0.75, 1.0);
        let f = sketch.add_point(0.25, 1.0);
        let g = sketch.add_point(0.0, 0.75);
        let h = sketch.add_point(0.0, 0.25);
        let i = sketch.add_point(0.75, 0.25);
        let j = sketch.add_point(0.75, 0.75);
        let k = sketch.add_point(0.25, 0.75);
        let l = sketch.add_point(0.25, 0.25);

        sketch.add_segment(a, b);
        sketch.add_arc(i, b, c, false);
        sketch.add_segment(c, d);
        sketch.add_arc(j, d, e, false);
        sketch.add_segment(e, f);
        sketch.add_arc(k, f, g, false);
        sketch.add_segment(g, h);
        sketch.add_arc(l, h, a, false);

        sketch.save_svg("test_svgs/rounded_square.svg");
    }

    #[test]
    fn square_with_hole_to_svg() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(1.0, 1.0);
        let d = sketch.add_point(0.0, 1.0);

        let e = sketch.add_point(0.25, 0.25);
        let f = sketch.add_point(0.75, 0.25);
        let g = sketch.add_point(0.75, 0.75);
        let h = sketch.add_point(0.25, 0.75);

        sketch.add_segment(a, b);
        sketch.add_segment(b, c);
        sketch.add_segment(c, d);
        sketch.add_segment(d, a);

        sketch.add_segment(e, f);
        sketch.add_segment(f, g);
        sketch.add_segment(g, h);
        sketch.add_segment(h, e);

        sketch.save_svg("test_svgs/square_with_hole.svg");
    }

    #[test]
    fn square_with_circular_hole_to_svg() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(1.0, 1.0);
        let d = sketch.add_point(0.0, 1.0);
        let center = sketch.add_point(0.5, 0.5);

        sketch.add_segment(a, b);
        sketch.add_segment(b, c);
        sketch.add_segment(c, d);
        sketch.add_segment(d, a);

        sketch.add_circle(center, 0.4);

        sketch.save_svg("test_svgs/square_with_circular_hole.svg");
    }

    #[test]
    fn circle_with_circular_hole_to_svg() {
        let mut sketch = Sketch::new();

        let center = sketch.add_point(0.5, 0.5);

        sketch.add_circle(center, 0.5);
        sketch.add_circle(center, 0.25);

        sketch.save_svg("test_svgs/circle_with_circular_hole.svg");
    }

    #[test]
    fn circle_with_square_hole_to_svg() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(1.0, 1.0);
        let d = sketch.add_point(0.0, 1.0);
        let center = sketch.add_point(0.5, 0.5);

        sketch.add_segment(a, b);
        sketch.add_segment(b, c);
        sketch.add_segment(c, d);
        sketch.add_segment(d, a);

        sketch.add_circle(center, 1.0);

        sketch.save_svg("test_svgs/circle_with_square_hole.svg");
    }

    #[test]
    fn two_intersecting_squares_to_svg() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(1.0, 1.0);
        let d = sketch.add_point(0.0, 1.0);
        sketch.add_segment(a, b);
        sketch.add_segment(b, c);
        sketch.add_segment(c, d);
        sketch.add_segment(d, a);

        let e = sketch.add_point(0.5, 0.5);
        let f = sketch.add_point(1.5, 0.5);
        let g = sketch.add_point(1.5, 1.5);
        let h = sketch.add_point(0.5, 1.5);
        sketch.add_segment(e, f);
        sketch.add_segment(f, g);
        sketch.add_segment(g, h);
        sketch.add_segment(h, e);

        sketch.save_svg("test_svgs/two_intersecting_squares_unsplit.svg");

        let sketch = sketch.split_intersections();
        sketch.save_svg("test_svgs/two_intersecting_squares_split.svg");
    }

    #[test]
    fn two_intersecting_circles_to_svg() {
        // Create a new sketch
        let mut sketch = Sketch::new();

        // Add two circles which happen to intersect
        let center_a = sketch.add_point(0.0, 0.0);
        sketch.add_circle(center_a, 1.0);
        let center_b = sketch.add_point(1.0, 0.0);
        sketch.add_circle(center_b, 1.0);

        // Save the naive svg: just two circular paths
        sketch.save_svg("test_svgs/two_intersecting_circles_unsplit.svg");

        // Split the intersections, creating a new and different sketch
        let sketch = sketch.split_intersections();

        // Save this one as an SVG, it will have three non-overlapping paths of two arcs each
        sketch.save_svg("test_svgs/two_intersecting_circles_split.svg");
    }

    #[test]
    fn two_arcs_in_a_circle_90() {
        let mut sketch = Sketch::new();

        let center = sketch.add_point(0.0, 0.0);
        let top = sketch.add_point(0.0, 1.0);
        let right = sketch.add_point(1.0, 0.0);

        sketch.add_arc(center, right, top, false);
        sketch.add_arc(center, top, right, false);

        sketch.save_svg("test_svgs/two_arcs_in_a_circle_90.svg");
    }

    #[test]
    fn two_arcs_in_a_circle_180() {
        let mut sketch = Sketch::new();

        let center = sketch.add_point(0.0, 0.0);
        let top = sketch.add_point(0.0, 1.0);
        let bottom = sketch.add_point(0.0, -1.0);

        sketch.add_arc(center, bottom, top, false);
        sketch.add_arc(center, top, bottom, false);

        sketch.save_svg("test_svgs/two_arcs_in_a_circle_180.svg");
    }

    #[test]
    fn segment_length_constraint() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);

        let segment_id = sketch.add_segment(a, b);

        let constraint_id = sketch.add_segment_length_constraint(segment_id, 2.0);

        assert!(sketch.solve(1000));
        println!("Segment length: {}", sketch.segment_length(segment_id));
        assert!(sketch.constraint_is_satisfied(constraint_id));
    }

    #[test]
    fn triangle_constraint() {
        let mut sketch = Sketch::new();

        // initialized as a right triangle with right angle at origin
        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(0.0, 1.0);

        let segment_ab = sketch.add_segment(a, b);
        let segment_bc = sketch.add_segment(b, c);
        let segment_ca = sketch.add_segment(c, a);

        let constraint_ab = sketch.add_segment_length_constraint(segment_ab, 2.0);
        let constraint_bc = sketch.add_segment_length_constraint(segment_bc, 2.0);
        let constraint_ca = sketch.add_segment_length_constraint(segment_ca, 2.0);

        assert!(sketch.solve(1000));

        assert!(sketch.all_constraints_are_satisfied());

        sketch.save_svg("test_svgs/constraint_triangle.svg");
    }

    #[test]
    fn segment_angle_constraint() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);

        let segment_id = sketch.add_segment(a, b);

        let constraint_id = sketch.add_segment_angle_constraint(segment_id, PI / 4.0);

        sketch.solve(10000);

        assert!(sketch.constraint_is_satisfied(constraint_id));
    }

    #[test]
    fn circle_diameter() {
        let mut sketch = Sketch::new();

        let center = sketch.add_point(0.0, 0.0);
        let circle_id = sketch.add_circle(center, 0.5);
        let constraint_id = sketch.add_circle_diameter_constraint(circle_id, 4.0);

        sketch.solve(4000);

        println!("Value: {}", sketch.constraint_value(constraint_id));
        println!("Error: {}", sketch.constraint_error(constraint_id));
        assert!(sketch.constraint_is_satisfied(constraint_id));

        sketch.save_svg("test_svgs/constraint_circle_diameter.svg");
    }

    #[test]
    fn segments_equal() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(0.5, 0.0);

        let c = sketch.add_point(0.0, 1.0);
        let d = sketch.add_point(1.0, 1.0);

        let segment_ab = sketch.add_segment(a, b); // length 0.5
        let segment_cd = sketch.add_segment(c, d); // length 1.0

        let constraint_id = sketch.add_segments_equal_constraint(segment_ab, segment_cd);

        sketch.save_svg("test_svgs/equality_constraint_unsolved.svg");
        assert!(sketch.solve(1000));
        sketch.save_svg("test_svgs/equality_constraint_solved.svg");
        println!("equality error: {}", sketch.constraint_error(constraint_id));
    }

    #[test]
    fn manual_square() {
        let mut sketch = Sketch::new();

        let a = sketch.add_fixed_point(0.0, 0.0);
        let b = sketch.add_point(1.0, -0.1);
        let c = sketch.add_point(1.1, 0.9);
        let d = sketch.add_point(-0.1, 0.9);

        let segment_ab = sketch.add_segment(a, b);
        let segment_bc = sketch.add_segment(b, c);
        let segment_cd = sketch.add_segment(c, d);
        let segment_da = sketch.add_segment(d, a);

        let length = 2.0;
        sketch.add_segment_length_constraint(segment_ab, length);
        sketch.add_segment_length_constraint(segment_bc, length);
        sketch.add_segment_length_constraint(segment_cd, length);
        sketch.add_segment_length_constraint(segment_da, length);

        sketch.add_segment_horizontal_constraint(segment_ab);
        sketch.add_segment_horizontal_constraint(segment_cd);
        sketch.add_segment_vertical_constraint(segment_da);
        sketch.add_segment_vertical_constraint(segment_bc);

        // for i in 0..100 {
        //     sketch.step();
        //     sketch.save_svg(&format!("test_svgs/manual_square/{}.svg", i));
        // }

        sketch.solve(1000);
        sketch.save_svg("test_svgs/manual_square_solved.svg");
    }

    #[test]
    fn manual_rectangle() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(1.0, 1.0);
        let d = sketch.add_point(0.0, 1.0);

        let segment_ab = sketch.add_segment(a, b);
        let segment_bc = sketch.add_segment(b, c);
        let segment_cd = sketch.add_segment(c, d);
        let segment_da = sketch.add_segment(d, a);

        sketch.add_segment_horizontal_constraint(segment_ab);
        sketch.add_segment_horizontal_constraint(segment_cd);
        sketch.add_segment_vertical_constraint(segment_da);
        sketch.add_segment_vertical_constraint(segment_bc);

        // fixed width of 1.0
        sketch.add_segment_length_constraint(segment_ab, 1.0);
        sketch.add_segment_length_constraint(segment_cd, 1.0);
        // This should cause it to adjust!
        sketch.add_segment_length_constraint(segment_da, 0.5);

        // for i in 0..800 {
        //     sketch.save_svg(&format!("test_svgs/manual_square/{}.svg", i));
        //     sketch.step();
        // }

        let solved = sketch.solve(1000);
        println!("did solve? {}", solved);
        sketch.save_svg("test_svgs/manual_rectangle_solved.svg");
    }

    #[test]
    fn delete_lines() {
        let mut sketch = Sketch::new();

        let a = sketch.add_fixed_point(0.0, 0.0);
        let b = sketch.add_point(1.0, -0.1);
        let c = sketch.add_point(1.1, 0.9);
        let d = sketch.add_point(-0.1, 0.9);

        let segment_ab = sketch.add_segment(a, b);
        let segment_bc = sketch.add_segment(b, c);
        let segment_cd = sketch.add_segment(c, d);
        let segment_da = sketch.add_segment(d, a);

        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 4);

        sketch.delete_line_segment(segment_ab);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 4);

        sketch.delete_line_segment(segment_bc);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 3);

        sketch.delete_line_segment(segment_cd);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 2);

        sketch.delete_line_segment(segment_da);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 0);
    }

    #[test]
    fn delete_arcs() {
        let mut sketch = Sketch::new();

        let a = sketch.add_point(1.0, 0.0);
        let b = sketch.add_point(2.0, 1.0);
        let c = sketch.add_point(1.0, 2.0);
        let d = sketch.add_point(0.0, 1.0);
        let center = sketch.add_point(1.0, 1.0);

        let arc_ab = sketch.add_arc(center, a, b, false);
        let arc_bc = sketch.add_arc(center, b, c, false);
        let arc_cd = sketch.add_arc(center, c, d, false);
        let arc_da = sketch.add_arc(center, d, a, false);

        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 5);

        sketch.delete_arc(arc_ab);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 5);

        sketch.delete_arc(arc_bc);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 4);

        sketch.delete_arc(arc_cd);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 3);

        sketch.delete_arc(arc_da);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 0);
    }

    #[test]
    fn delete_circles() {
        let mut sketch = Sketch::new();

        let center = sketch.add_point(1.0, 1.0);
        let circle_a = sketch.add_circle(center, 1.0);
        let circle_b = sketch.add_circle(center, 2.0);

        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 3);

        sketch.delete_circle(circle_a);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 2);

        sketch.delete_circle(circle_b);
        println!("points: {:?}", sketch.points.len());
        assert_eq!(sketch.points.len(), 0);
    }

    #[test]
    fn line_through_rectangle() {
        let contents =
            std::fs::read_to_string("src/test_inputs/line_through_rectangle.cadmium").unwrap();
        let p: Project = serde_json::from_str(&contents).unwrap();
        // println!("{:?}", p);

        let realized = p.get_realization(0, 1000);
        let (sketch_unsplit, sketch_split, _) = realized.sketches.get("Sketch-0").unwrap();
        println!("Sketch: {:?}", sketch_split);
        println!("Faces: {:?}", sketch_split.faces);
        println!("Number of faces: {:?}", sketch_split.faces.len());
        assert_eq!(sketch_split.faces.len(), 2);
    }

    #[test]
    fn line_through_many_rectangles() {
        let contents =
            std::fs::read_to_string("src/test_inputs/line_through_many_rectangles.cadmium")
                .unwrap();
        let p: Project = serde_json::from_str(&contents).unwrap();
        // println!("{:?}", p);

        let realized = p.get_realization(0, 1000);
        let (sketch_unsplit, sketch_split, _) = realized.sketches.get("Sketch-0").unwrap();
        // println!("Sketch: {:?}", sketch_split);
        // println!("Faces: {:?}", sketch_split.faces);
        println!("Number of faces: {:?}", sketch_split.faces.len());
        assert_eq!(sketch_split.faces.len(), 8);
    }

    #[test]
    fn two_circles_two_intersections() {
        // two intersecting circles should yield 3 extrudable faces
        let contents = std::fs::read_to_string(
            "src/test_inputs/sketches/circle_circle/two_circles_two_intersections.cadmium",
        )
        .unwrap();
        let p: Project = serde_json::from_str(&contents).unwrap();

        let realized = p.get_realization(0, 1000);
        let (sketch_unsplit, sketch_split, _) = realized.sketches.get("Sketch-0").unwrap();

        println!("Number of faces: {:?}", sketch_split.faces.len());
        assert_eq!(sketch_split.faces.len(), 3);
    }

    #[test]
    fn three_circles() {
        // three intersecting circles should yield 5 extrudable faces
        let contents =
            std::fs::read_to_string("src/test_inputs/sketches/circle_circle/three_circles.cadmium")
                .unwrap();
        let p: Project = serde_json::from_str(&contents).unwrap();

        let realized = p.get_realization(0, 1000);
        let (sketch_unsplit, sketch_split, _) = realized.sketches.get("Sketch-0").unwrap();

        println!("Number of faces: {:?}", sketch_split.faces.len());
        assert_eq!(sketch_split.faces.len(), 3);
    }

    #[test]
    fn circle_circle_intersection() {
        let mut sketch = Sketch::new();

        // two touching normally
        println!("two circles touching normally");
        let a_radius = 1.0;
        let a = sketch.add_point(0.0, 0.0);
        let a_top = sketch.add_point(0.0, a_radius);
        let b_radius = 1.0;
        let b = sketch.add_point(1.0, 0.0);
        let b_top = sketch.add_point(1.0, b_radius);
        let circle_a = Circle2 {
            center: a,
            radius: a_radius,
            top: a_top,
        };
        let circle_b = Circle2 {
            center: b,
            radius: b_radius,
            top: b_top,
        };
        let intersection = sketch.circle_circle_intersection(&circle_a, &circle_b);
        assert_eq!(
            intersection,
            Intersection::TwoPoints(
                Point2::new(0.5, -0.8660254037844386),
                false,
                Point2::new(0.5, 0.8660254037844386),
                false
            )
        );
    }

    #[test]
    fn line_line_intersection() {
        let mut sketch = Sketch::new();

        // simple cross
        println!("simple cross");
        let a = sketch.add_point(-1.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(0.0, -1.0);
        let d = sketch.add_point(0.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::OnePoint(Point2::new(0.0, 0.0), false)
        );

        // a T
        println!("a T");
        let a = sketch.add_point(-1.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(0.0, 0.0);
        let d = sketch.add_point(0.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::OnePoint(Point2::new(0.0, 0.0), false)
        );

        // parallel horizontal
        println!("parallel horizontal");
        let a = sketch.add_point(-1.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(-1.0, 1.0);
        let d = sketch.add_point(1.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(intersection, Intersection::None);

        // parallel vertical
        println!("parallel vertical");
        let a = sketch.add_point(0.0, -1.0);
        let b = sketch.add_point(0.0, 1.0);
        let c = sketch.add_point(1.0, -1.0);
        let d = sketch.add_point(1.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(intersection, Intersection::None);

        // perpendicular but not intersecting
        println!("perpendicular but not intersecting");
        let a = sketch.add_point(-1.0, 0.0);
        let b = sketch.add_point(1.0, 0.0);
        let c = sketch.add_point(3.0, 0.0);
        let d = sketch.add_point(3.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(intersection, Intersection::None);

        // share 1 point but only in the === sense not the == sense
        println!("share 1 point but only in the === sense not the == sense");
        let a = sketch.add_point(-1.0, 1.0);
        let b = sketch.add_point(0.0, 0.0);
        let c = sketch.add_point(0.0, 0.0);
        let d = sketch.add_point(1.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::OnePoint(Point2::new(0.0, 0.0), false)
        );

        // share 1 point in the == sense
        println!("share 1 point in the == sense");
        let a = sketch.add_point(-1.0, 1.0);
        let b = sketch.add_point(0.0, 0.0);
        let d = sketch.add_point(1.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: b, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::OnePoint(Point2::new(0.0, 0.0), true)
        );

        // colinear, horizontal no intersection
        println!("colinear horizontal no intersection");
        let a = sketch.add_point(-1.0, 0.0);
        let b = sketch.add_point(0.0, 0.0);
        let c = sketch.add_point(1.0, 0.0);
        let d = sketch.add_point(2.0, 0.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(intersection, Intersection::None);

        // colinear, vertical no intersection
        println!("colinear vertical no intersection");
        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(0.0, 1.0);
        let c = sketch.add_point(0.0, 2.0);
        let d = sketch.add_point(0.0, 3.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(intersection, Intersection::None);

        // Lines are exactly equal
        println!("Exactly equal");
        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(0.0, 1.0);
        let line_ab = Line2 { start: a, end: b };
        let intersection = sketch.line_line_intersection(&line_ab, &line_ab);
        assert_eq!(
            intersection,
            Intersection::Line(Point2::new(0.0, 0.0), Point2::new(0.0, 1.0))
        );

        println!("\nLine Overlap:");
        // lines overlap somewhat, both vertical
        println!("lines overlap somewhat, both vertical");
        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(0.0, 2.0);
        let c = sketch.add_point(0.0, 1.0);
        let d = sketch.add_point(0.0, 3.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::Line(Point2::new(0.0, 2.0), Point2::new(0.0, 1.0))
        );
        // for future reference: the ordering of points here and for all of the tests below is inconsequential
        // Feel free to swap the order here if the implementation changes. Maybe these should always come
        // in a canonical order?

        // lines overlap somewhat, both horizontal
        println!("lines overlap somewhat, both horizontal");
        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(2.0, 0.0);
        let c = sketch.add_point(1.0, 0.0);
        let d = sketch.add_point(3.0, 0.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::Line(Point2::new(2.0, 0.0), Point2::new(1.0, 0.0))
        );

        // lines overlap somewhat, both angled
        println!("lines overlap somewhat, both angled");
        let a = sketch.add_point(0.0, 0.0);
        let b = sketch.add_point(2.0, 2.0);
        let c = sketch.add_point(1.0, 1.0);
        let d = sketch.add_point(3.0, 3.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::Line(Point2::new(2.0, 2.0), Point2::new(1.0, 1.0))
        );

        // one line engulfs the other, both angled
        println!("one line engulfs the other, both angled");
        let a = sketch.add_point(1.0, 1.0);
        let b = sketch.add_point(2.0, 2.0);
        let c = sketch.add_point(0.0, 0.0);
        let d = sketch.add_point(3.0, 3.0);
        let line_ab = Line2 { start: a, end: b };
        let line_cd = Line2 { start: c, end: d };
        let intersection = sketch.line_line_intersection(&line_ab, &line_cd);
        assert_eq!(
            intersection,
            Intersection::Line(Point2::new(1.0, 1.0), Point2::new(2.0, 2.0))
        );
    }
}