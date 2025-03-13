#[derive(Debug, Copy, Clone)]
pub struct Color(u32);

impl Color {
    pub const BLACK: Color = Color(0xFF00_0000);
    pub const BLUE: Color = Color(0xFF00_00FF);
    pub const CYAN: Color = Color(0xFF00_FFFF);
    pub const GREEN: Color = Color(0xFF00_FF00);
    pub const MAGENTA: Color = Color(0xFFFF_00FF);
    pub const RED: Color = Color(0xFFFF_0000);
    pub const WHITE: Color = Color(0xFFFF_FFFF);
    pub const YELLOW: Color = Color(0xFFFF_FF00);

    pub fn value(self) -> u32 {
        self.0
    }
}
