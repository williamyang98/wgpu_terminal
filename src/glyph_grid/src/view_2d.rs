use cgmath::Vector2;

pub struct MutableView2d<'a,T> {
    pub data: &'a mut [T],
    pub row_stride: usize,
    pub size: Vector2<usize>,
}

pub struct ImmutableView2d<'a, T> {
    pub data: &'a [T],
    pub row_stride: usize,
    pub size: Vector2<usize>,
}
