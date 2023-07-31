methods_enum::impl_match! {

enum Shape<'a> {
//     Circle(f64, &'a str),                  // if you uncomment or remove these 4 lines 
//     Rectangle { width: f64, height: f64 }, //    it will work the same
// }
// @enum Shape<'a> {
    Circle(f64, &'a str): (radius, mark)
        zoom(scale)    { Shape::Circle(radius * scale, mark) }      // template change
        fmt(f) Display { write!(f, "{mark}(R: {radius:.1})") };     (_, mark) 
        mark_obj(obj)  { format!("{} {}", mark, obj) };             (radius, _)
        to_rect()      { *self = Shape::Rectangle { width: radius * 2., height: radius * 2.,} }
    ,
    Rectangle { width: f64, height: f64}: { width: w, height}
        zoom(scale)    { Shape::Rectangle { width: w * scale, height: height * scale } }
        fmt(f) Display { write!(f, "Rectangle(W: {w:.1}, H: {height:.1})") }; {..}
        mark_obj(obj)  { format!("⏹️ {}", obj) }
}
impl<'a> Shape<'a> {
    fn zoom(&mut self, scale: f64)                      ~{ *self = match *self }
    fn to_rect(&mut self) -> &mut Self                  ~{ match *self {}; self }
    fn mark_obj<T: Display>(&self, obj: &T) -> String   ~{ match self }
}

use std::fmt::{Display, Formatter, Result};

impl<'a> Display for Shape<'a>{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result      ~{ match self }
}

} // <--impl_match!

pub fn main() {
    let mut rect = Shape::Rectangle { width: 10., height: 10. };
    assert_eq!(format!("{rect}"), "Rectangle(W: 10.0, H: 10.0)");
    rect.zoom(3.);
    let mut circle = Shape::Circle(15., "⭕");
    assert_eq!(circle.mark_obj(&rect.mark_obj(&circle)), "⭕ ⏹️ ⭕(R: 15.0)");
    // "Rectangle(W: 30.0, H: 30.0)"
    assert_eq!(circle.to_rect().to_string(), rect.to_string());
}
