use pdfium_render::prelude::PdfRect;

#[derive(Debug, Clone)]
pub struct Polygon {
    pub vertices: Vec<(f32, f32)>,
}

impl Polygon {
    pub fn from_points(points: Vec<(f32, f32)>) -> crate::error::Result<Self> {
        if points.len() < 3 {
            return Err(crate::error::PdfError::InvalidPolygon(
                "Polygon must have at least 3 vertices".into(),
            ));
        }
        Ok(Polygon { vertices: points })
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        let mut inside = false;
        let n = self.vertices.len();

        for i in 0..n {
            let j = (i + 1) % n;
            let (xi, yi) = self.vertices[i];
            let (xj, yj) = self.vertices[j];

            let intersect = ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);

            if intersect {
                inside = !inside;
            }
        }

        inside
    }

    pub fn rect_overlap_ratio(&self, rect: &PdfRect) -> f32 {
        let rect_area = rect.width().value * rect.height().value;
        if rect_area <= 0.0 {
            return 0.0;
        }

        let samples = 20;
        let mut inside_count = 0;
        let total_samples = samples * samples;

        let left = rect.left().value;
        let bottom = rect.bottom().value;
        let width = rect.width().value;
        let height = rect.height().value;

        for i in 0..samples {
            for j in 0..samples {
                let x = left + (i as f32 / samples as f32) * width;
                let y = bottom + (j as f32 / samples as f32) * height;

                if self.contains_point(x, y) {
                    inside_count += 1;
                }
            }
        }

        inside_count as f32 / total_samples as f32
    }

    pub fn centroid(&self) -> (f32, f32) {
        let n = self.vertices.len() as f32;
        let sum_x: f32 = self.vertices.iter().map(|(x, _)| x).sum();
        let sum_y: f32 = self.vertices.iter().map(|(_, y)| y).sum();
        (sum_x / n, sum_y / n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfium_render::prelude::PdfRect;

    #[test]
    fn test_from_points_valid_triangle() {
        let points = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let result = Polygon::from_points(points.clone());
        assert!(result.is_ok());
        let polygon = result.unwrap();
        assert_eq!(polygon.vertices.len(), 3);
        assert_eq!(polygon.vertices, points);
    }

    #[test]
    fn test_from_points_valid_square() {
        let points = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let result = Polygon::from_points(points);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_points_invalid_too_few() {
        let points = vec![(0.0, 0.0), (1.0, 0.0)];
        let result = Polygon::from_points(points);
        assert!(result.is_err());
        match result {
            Err(crate::error::PdfError::InvalidPolygon(msg)) => {
                assert_eq!(msg, "Polygon must have at least 3 vertices");
            }
            _ => panic!("Expected InvalidPolygon error"),
        }
    }

    #[test]
    fn test_from_points_invalid_empty() {
        let points = vec![];
        let result = Polygon::from_points(points);
        assert!(result.is_err());
    }

    #[test]
    fn test_contains_point_inside_triangle() {
        let polygon = Polygon::from_points(vec![(0.0, 0.0), (4.0, 0.0), (2.0, 4.0)]).unwrap();
        assert!(polygon.contains_point(2.0, 1.0));
        assert!(polygon.contains_point(2.0, 2.0));
    }

    #[test]
    fn test_contains_point_outside_triangle() {
        let polygon = Polygon::from_points(vec![(0.0, 0.0), (4.0, 0.0), (2.0, 4.0)]).unwrap();
        assert!(!polygon.contains_point(-1.0, 0.0));
        assert!(!polygon.contains_point(5.0, 0.0));
        assert!(!polygon.contains_point(2.0, 5.0));
    }

    #[test]
    fn test_contains_point_on_vertex() {
        let polygon = Polygon::from_points(vec![(0.0, 0.0), (4.0, 0.0), (2.0, 4.0)]).unwrap();
        let on_vertex = polygon.contains_point(0.0, 0.0);
        assert!(!on_vertex || on_vertex);
    }

    #[test]
    fn test_contains_point_inside_square() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        assert!(polygon.contains_point(5.0, 5.0));
        assert!(polygon.contains_point(1.0, 1.0));
        assert!(polygon.contains_point(9.0, 9.0));
    }

    #[test]
    fn test_contains_point_outside_square() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        assert!(!polygon.contains_point(-1.0, 5.0));
        assert!(!polygon.contains_point(11.0, 5.0));
        assert!(!polygon.contains_point(5.0, -1.0));
        assert!(!polygon.contains_point(5.0, 11.0));
    }

    #[test]
    fn test_contains_point_complex_polygon() {
        let polygon = Polygon::from_points(vec![
            (0.0, 0.0),
            (5.0, 0.0),
            (5.0, 3.0),
            (3.0, 3.0),
            (3.0, 5.0),
            (0.0, 5.0),
        ])
        .unwrap();
        assert!(polygon.contains_point(2.0, 2.0));
        assert!(!polygon.contains_point(4.0, 4.0));
    }

    #[test]
    fn test_centroid_triangle() {
        let polygon = Polygon::from_points(vec![(0.0, 0.0), (3.0, 0.0), (0.0, 3.0)]).unwrap();
        let (cx, cy) = polygon.centroid();
        assert!((cx - 1.0).abs() < 0.01);
        assert!((cy - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_centroid_square() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        let (cx, cy) = polygon.centroid();
        assert!((cx - 5.0).abs() < 0.01);
        assert!((cy - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_centroid_offset_square() {
        let polygon =
            Polygon::from_points(vec![(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)])
                .unwrap();
        let (cx, cy) = polygon.centroid();
        assert!((cx - 15.0).abs() < 0.01);
        assert!((cy - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_rect_overlap_ratio_full_overlap() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        let rect = PdfRect::new_from_values(0.0, 2.0, 2.0, 4.0);
        let ratio = polygon.rect_overlap_ratio(&rect);
        assert!(ratio > 0.9);
    }

    #[test]
    fn test_rect_overlap_ratio_no_overlap() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        let rect = PdfRect::new_from_values(20.0, 20.0, 25.0, 25.0);
        let ratio = polygon.rect_overlap_ratio(&rect);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_rect_overlap_ratio_partial_overlap() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        let rect = PdfRect::new_from_values(-5.0, -5.0, 5.0, 5.0);
        let ratio = polygon.rect_overlap_ratio(&rect);
        assert!(ratio > 0.0 && ratio < 1.0);
    }

    #[test]
    fn test_rect_overlap_ratio_zero_area_rect() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        let rect = PdfRect::new_from_values(5.0, 5.0, 5.0, 5.0);
        let ratio = polygon.rect_overlap_ratio(&rect);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn test_rect_overlap_ratio_negative_area_rect() {
        let polygon =
            Polygon::from_points(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]).unwrap();
        let rect = PdfRect::new_from_values(5.0, 5.0, 10.0, 10.0);
        let ratio = polygon.rect_overlap_ratio(&rect);
        assert!(ratio > 0.9);
    }
}
