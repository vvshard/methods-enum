methods_enum::impl_match! { (!)

enum Shape {
    Circle(f64): (radius)
        zoom(scale)    { Shape::Circle(radius * scale) }
        to_rect()      { *self = Shape::Rectangle { width: radius * 2., height: radius * 2.} }
        fmt(f) Display { write!(f, "Circle(R: {radius:.1})") }
    ,
    Rectangle { width: f64, height: f64 }: { width: w, height }
        zoom(scale)    { Shape::Rectangle{width: w * scale, height: height * scale} }
        fmt(f) Display { write!(f, "Rectangle(W: {w:.1}, H: {height:.1})") }
}
impl Shape {
    fn zoom(self, scale: f64) -> Shape              ~{ match self }
    fn to_rect(&mut self)                           ~{ match *self {} } 
}
use std::fmt::{Display, Formatter, Result};
impl Display for Shape{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result  ~{ match self } 
}

} //impl_match!

pub fn main() {
    let rect = Shape::Rectangle { width: 10., height: 10. };
    assert_eq!(format!("{rect}"), "Rectangle(W: 10.0, H: 10.0)");

    let mut circle = Shape::Circle(15.);
    assert_eq!(circle.to_string(), "Circle(R: 15.0)");
    circle.to_rect();
    assert_eq!(circle.to_string(), rect.zoom(3.).to_string());
}
