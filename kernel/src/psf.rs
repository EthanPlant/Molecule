use byteorder::{ByteOrder, LittleEndian};

const PSF_MAGIC: u32 = 0x864A_B572;

static FONT: &[u8] = include_bytes!("../../Tamsyn8x15b.psf");

pub struct PsfFont {
    _magic: u32,
    _version: u32,
    header_size: u32,
    _flags: u32,
    _num_glyphs: u32,
    bytes_per_glyph: u32,
    height: u32,
    width: u32,
}

impl PsfFont {
    pub fn read_glyph_row(&self, offset: usize, row: usize) -> u8 {
        let start = self.header_size as usize + (offset * self.bytes_per_glyph as usize);
        let index = start + row;
        if index >= FONT.len() {
            return 0x00;
        }

        FONT[index]
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }
}

impl Default for PsfFont {
    fn default() -> Self {
        Self {
            _magic: PSF_MAGIC,
            _version: LittleEndian::read_u32(&FONT[4..8]),
            header_size: LittleEndian::read_u32(&FONT[8..12]),
            _flags: LittleEndian::read_u32(&FONT[12..16]),
            _num_glyphs: LittleEndian::read_u32(&FONT[16..20]),
            bytes_per_glyph: LittleEndian::read_u32(&FONT[20..24]),
            height: LittleEndian::read_u32(&FONT[24..28]),
            width: LittleEndian::read_u32(&FONT[28..32]),
        }
    }
}
