struct Foo {
    x: i32,
}

pub fn main() {
    let foo = Foo { x: 1 };

    assert_eq!(foo.st1(), "");
    assert_eq!(foo.st2(), "\n\"]{''");
    assert_eq!(foo.st3(), r#""\"#);
    assert_eq!(foo.a1(), [0, 1]);
    assert_eq!(foo.i1(), 5 % 2 + 5 / 2);
    assert_eq!(foo.foo2().x, 2);
}

#[rustfmt::skip]
#[methods_enum::gen(Meth, run, Out)]
impl Foo {
    fn st1(&self) -> &str { "" }
    fn st2(&self) -> &str { "\n\"]{''" }
    fn st3(&self) -> &str { r#""\"# }
    fn a1(&self) -> [i32; 2] { [0, 1] }
    fn i1(&self) -> i32 { 5 % 2 + 5 / 2 }
    fn foo2(&self) -> Foo { Foo{x: 2} }
    
    fn run(&self, _method: Meth) -> Out{
        Out::Unit
    }
}

