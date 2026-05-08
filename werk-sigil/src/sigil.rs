#[derive(Debug, Clone)]
pub struct SvgBytes(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct Sigil {
    pub svg: SvgBytes,
}

impl Sigil {
    pub fn new(svg: Vec<u8>) -> Self {
        Self { svg: SvgBytes(svg) }
    }
}
